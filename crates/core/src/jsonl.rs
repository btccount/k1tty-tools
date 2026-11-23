use anyhow::{bail, Context, Result};
use chrono::{DateTime, Local, Utc};
#[cfg(unix)]
use fs2::FileExt;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, OpenOptions},
    io::{BufRead, BufReader, Read, Seek, SeekFrom, Write},
    path::{Component, Path, PathBuf},
    time::UNIX_EPOCH,
};
use walkdir::WalkDir;

pub fn text(v: &Value) -> String {
    match v {
        Value::Null => String::new(),
        Value::String(s) => s.clone(),
        _ => serde_json::to_string_pretty(v).unwrap_or_default(),
    }
}
pub fn s<'a>(v: &'a Value, key: &str) -> &'a str {
    v.get(key).and_then(Value::as_str).unwrap_or("")
}
pub fn required<'a>(v: &'a Value, key: &str) -> Result<&'a str> {
    v.get(key)
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .with_context(|| format!("Требуется {key}."))
}
pub fn valid_id(id: &str) -> Result<()> {
    if id.is_empty() || id == "." || id == ".." || id.contains(['/', '\\', '\0', ':']) {
        bail!("Недопустимый идентификатор сессии.")
    }
    Ok(())
}
pub fn truncate(s: &str, n: usize) -> String {
    s.chars().take(n).collect()
}
pub fn sha(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
pub fn hash_file(path: &Path) -> Result<String> {
    let mut f = File::open(path)?;
    let mut h = Sha256::new();
    let mut b = [0u8; 65536];
    loop {
        let n = f.read(&mut b)?;
        if n == 0 {
            break;
        }
        h.update(&b[..n]);
    }
    Ok(format!("{:x}", h.finalize()))
}
pub fn load_json(path: &Path) -> Result<Value> {
    serde_json::from_reader(File::open(path)?)
        .with_context(|| format!("Некорректный JSON: {}", path.display()))
}
pub fn save_json(path: &Path, value: &Value) -> Result<()> {
    atomic_write(path, &serde_json::to_vec_pretty(value)?)
}
pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    crate::storage::atomic_write(path, bytes)
}
pub fn paths(root: &Path, extension: &str) -> Vec<PathBuf> {
    let mut p: Vec<_> = WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_type().is_file()
                && e.path().extension().and_then(|e| e.to_str()) == Some(extension)
        })
        .map(|e| e.into_path())
        .collect();
    p.sort();
    p
}
pub fn dirs(root: &Path) -> Vec<PathBuf> {
    let mut p: Vec<_> = fs::read_dir(root)
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_type()
                .map(|t| t.is_dir() && !t.is_symlink())
                .unwrap_or(false)
        })
        .map(|e| e.path())
        .collect();
    p.sort_by(|a, b| b.cmp(a));
    p
}
pub fn size(root: &Path) -> u64 {
    WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter_map(|e| e.metadata().ok())
        .map(|m| m.len())
        .sum()
}
pub fn mtime(path: &Path) -> f64 {
    fs::metadata(path)
        .ok()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|t| t.as_secs_f64())
        .unwrap_or(0.)
}
pub fn display_time(sec: f64) -> String {
    DateTime::from_timestamp(sec as i64, 0)
        .unwrap_or(DateTime::<Utc>::UNIX_EPOCH)
        .with_timezone(&Local)
        .format("%d.%m.%Y %H:%M")
        .to_string()
}
pub fn stamp() -> String {
    Local::now().format("%Y%m%d-%H%M%S-%6f").to_string()
}
pub fn now() -> String {
    Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Micros, true)
}
pub fn safe_backup(root: &Path, path: &Path) -> Result<PathBuf> {
    let r = root.canonicalize()?;
    let p = path.canonicalize()?;
    if !p.starts_with(&r) || p == r || p.parent() == Some(r.as_path()) || !p.is_dir() {
        bail!("Недопустимый путь резервной копии.")
    }
    Ok(p)
}
pub fn safe_join(root: &Path, rel: &str) -> Result<PathBuf> {
    let normalized = rel.replace('\\', "/");
    let p = Path::new(&normalized);
    if p.as_os_str().is_empty()
        || p.is_absolute()
        || p.components()
            .any(|c| !matches!(c, Component::Normal(_) | Component::CurDir))
    {
        bail!("Недопустимый путь назначения.")
    };
    let dest = root.join(p);
    let mut ancestor = dest.as_path();
    while !ancestor.exists() {
        ancestor = ancestor.parent().context("Нет каталога назначения")?;
    }
    if root.exists() && !ancestor.canonicalize()?.starts_with(root.canonicalize()?) {
        bail!("Путь назначения выходит за каталог данных.")
    }
    Ok(dest)
}
pub fn copy_checked(src: &Path, dst: &Path) -> Result<String> {
    if let Some(p) = dst.parent() {
        fs::create_dir_all(p)?;
    }
    let h = hash_file(src)?;
    fs::copy(src, dst)?;
    OpenOptions::new().write(true).open(dst)?.sync_all()?;
    if hash_file(dst)? != h {
        bail!("Контрольная сумма резервной копии не совпадает.")
    }
    Ok(h)
}

pub struct LockedFile {
    pub file: File,
    _writer: Option<File>,
}
impl LockedFile {
    pub fn open(path: &Path, codex: bool) -> Result<Self> {
        #[cfg(unix)]
        let mut writer = None;
        #[cfg(not(unix))]
        let writer = None;
        #[cfg(unix)]
        if codex {
            let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
            if stem.len() >= 36 {
                let tid = stem.get(stem.len() - 36..).unwrap_or("");
                if uuid::Uuid::parse_str(tid).is_ok() {
                    for par in path.ancestors() {
                        if par.file_name().and_then(|p| p.to_str()) == Some("sessions") {
                            let p = par
                                .parent()
                                .unwrap_or(par)
                                .join("thread-writer-locks")
                                .join(format!("{tid}.lock"));
                            if p.is_file() {
                                let f = OpenOptions::new()
                                    .read(true)
                                    .write(true)
                                    .open(&p)
                                    .or_else(|_| File::open(&p))?;
                                f.try_lock_exclusive()
                                    .context("Файл занят Codex. Закройте активный тред.")?;
                                writer = Some(f);
                            }
                            break;
                        }
                    }
                }
            }
        }
        #[cfg(not(unix))]
        let _ = codex;
        let mut options = OpenOptions::new();
        options.read(true).write(true);
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            options.share_mode(1);
        }
        let f = options
            .open(path)
            .with_context(|| format!("Файл занят или недоступен: {}", path.display()))?;
        #[cfg(unix)]
        f.try_lock_exclusive()
            .context("Файл занят другим процессом.")?;
        Ok(Self {
            file: f,
            _writer: writer,
        })
    }
    pub fn replace_from(&mut self, src: &Path) -> Result<()> {
        let mut f = File::open(src)?;
        self.file.seek(SeekFrom::Start(0))?;
        let len = std::io::copy(&mut f, &mut self.file)?;
        self.file.set_len(len)?;
        self.file.sync_all()?;
        Ok(())
    }
    pub fn replace_bytes(&mut self, bytes: &[u8]) -> Result<()> {
        self.file.seek(SeekFrom::Start(0))?;
        self.file.write_all(bytes)?;
        self.file.set_len(bytes.len() as u64)?;
        self.file.sync_all()?;
        Ok(())
    }
}
impl Drop for LockedFile {
    fn drop(&mut self) {
        #[cfg(unix)]
        {
            let _ = FileExt::unlock(&self.file);
            if let Some(f) = &self._writer {
                let _ = FileExt::unlock(f);
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct Line {
    pub offset: u64,
    pub raw: Vec<u8>,
}
impl Line {
    pub fn obj(&self) -> Result<Value> {
        Ok(serde_json::from_slice(&self.raw)?)
    }
    pub fn set(&mut self, v: &Value) -> Result<()> {
        let e = ending(&self.raw);
        let mut raw = serde_json::to_vec(v)?;
        raw.extend_from_slice(e);
        self.raw = raw;
        Ok(())
    }
}
pub fn ending(raw: &[u8]) -> &[u8] {
    if raw.ends_with(b"\r\n") {
        b"\r\n"
    } else if raw.ends_with(b"\n") {
        b"\n"
    } else {
        b""
    }
}
pub fn read_lines(path: &Path) -> Result<Vec<Line>> {
    let mut f = BufReader::new(File::open(path)?);
    let mut offset = 0;
    let mut out = Vec::new();
    loop {
        let mut raw = Vec::new();
        let n = f.read_until(b'\n', &mut raw)?;
        if n == 0 {
            break;
        }
        out.push(Line { offset, raw });
        offset += n as u64;
    }
    Ok(out)
}
pub fn write_lines(lines: &[Line]) -> Vec<u8> {
    let mut out = Vec::new();
    for l in lines {
        out.extend_from_slice(&l.raw);
    }
    out
}
pub fn read_at(path: &Path, offset: u64) -> Result<Line> {
    let mut f = File::open(path)?;
    let size = f.metadata()?.len();
    if offset >= size {
        bail!("Запись по смещению b{offset} не найдена.")
    }
    if offset > 0 {
        f.seek(SeekFrom::Start(offset - 1))?;
        let mut c = [0];
        f.read_exact(&mut c)?;
        if c[0] != b'\n' {
            bail!("Ключ b{offset} устарел (не граница строки).")
        }
    } else {
        f.seek(SeekFrom::Start(0))?;
    }
    let mut raw = Vec::new();
    BufReader::new(f).read_until(b'\n', &mut raw)?;
    Ok(Line { offset, raw })
}
pub fn reverse_lines(
    mut f: File,
    end: Option<u64>,
    mut callback: impl FnMut(u64, &[u8]) -> Result<bool>,
) -> Result<()> {
    let size = f.metadata()?.len();
    let mut pos = end.unwrap_or(size);
    if pos > size {
        bail!("Некорректный курсор истории.")
    };
    let mut fragments = Vec::<Vec<u8>>::new();
    while pos > 0 {
        let n = pos.min(65536) as usize;
        pos -= n as u64;
        f.seek(SeekFrom::Start(pos))?;
        let mut chunk = vec![0; n];
        f.read_exact(&mut chunk)?;
        let mut e = n;
        loop {
            if let Some(nl) = chunk[..e].iter().rposition(|&c| c == b'\n') {
                let mut line = chunk[nl + 1..e].to_vec();
                for frag in fragments.drain(..).rev() {
                    line.extend_from_slice(&frag)
                }
                if !line.is_empty() {
                    if line.last() == Some(&b'\r') {
                        line.pop();
                    }
                    if !callback(pos + nl as u64 + 1, &line)? {
                        return Ok(());
                    }
                }
                e = nl;
            } else {
                if e > 0 {
                    fragments.push(chunk[..e].to_vec());
                }
                break;
            }
        }
    }
    if !fragments.is_empty() {
        let mut line = Vec::new();
        for f in fragments.into_iter().rev() {
            line.extend(f);
        }
        if line.last() == Some(&b'\r') {
            line.pop();
        }
        let _ = callback(0, &line)?;
    }
    Ok(())
}
pub fn turns(items: Vec<Value>) -> Vec<Value> {
    let mut out = Vec::<Value>::new();
    for item in items {
        if s(&item, "kind") == "user" || out.is_empty() {
            out.push(json!({"items":[]}))
        }
        out.last_mut().unwrap()["items"]
            .as_array_mut()
            .unwrap()
            .push(item);
    }
    out
}
pub fn label(kind: &str, assistant: &str) -> String {
    match kind {
        "user" => "Вы",
        "assistant" => assistant,
        "tool" => "Инструмент",
        "compact" => "Компакция",
        "system" => "Система",
        "error" => "Ошибка",
        _ => "Прочее",
    }
    .into()
}
pub fn normalized_doc(
    id: &str,
    cid: &str,
    title: &str,
    directory: &str,
    raw_size: u64,
    entries: Vec<Value>,
    include_tools: bool,
) -> Value {
    let entries: Vec<_> = entries
        .into_iter()
        .filter(|e| {
            if include_tools {
                !s(e, "text").trim().is_empty() || !s(e, "tool").is_empty()
            } else {
                matches!(s(e, "kind"), "user" | "assistant" | "compact")
                    && !s(e, "text").trim().is_empty()
            }
        })
        .collect();
    let title = if !title.trim().is_empty() {
        truncate(title.trim(), 140)
    } else {
        entries
            .iter()
            .find(|e| s(e, "kind") == "user" && !s(e, "text").trim().is_empty())
            .map(|e| {
                truncate(
                    &s(e, "text")
                        .split_whitespace()
                        .collect::<Vec<_>>()
                        .join(" "),
                    90,
                )
            })
            .unwrap_or_else(|| "Imported chat".into())
    };
    let times: Vec<_> = entries
        .iter()
        .filter_map(|e| e["ts"].as_str())
        .filter(|t| !t.is_empty())
        .collect();
    let created = times.first().map(|s| s.to_string()).unwrap_or_else(now);
    let updated = times.last().map(|s| s.to_string()).unwrap_or_else(now);
    json!({"meta":{"source_provider":id,"source_id":cid,"title":title,"directory":directory,"created_at":created,"updated_at":updated,"raw_size":raw_size},"entries":entries})
}
pub fn strip_long_prefix(path: &str) -> String {
    if let Some(p) = path.strip_prefix("\\\\?\\UNC\\") {
        format!("\\\\{p}")
    } else {
        path.strip_prefix("\\\\?\\").unwrap_or(path).to_string()
    }
}
pub fn leak_check(targets: &[PathBuf], texts: &[String]) -> Value {
    let mut frags = Vec::new();
    for t in texts {
        for line in t.lines() {
            let f = line.replace(['\r', '\n', '\t'], " ");
            let f = f.trim();
            if f.chars().count() >= 40 {
                frags.push(truncate(f, 80));
                if frags.len() >= 15 {
                    break;
                }
            }
        }
        if frags.len() >= 25 {
            break;
        }
    }
    let mut leaks = Vec::new();
    for p in targets {
        if let Ok(raw) = fs::read_to_string(p) {
            for f in &frags {
                if raw.contains(f) {
                    leaks.push(json!([
                        p.file_name().unwrap_or_default().to_string_lossy(),
                        f
                    ]))
                }
            }
        }
    }
    Value::Array(leaks)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reverse_large_utf8_and_crlf() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("x");
        let big = "ж".repeat(70000);
        let data = format!("first\r\n{big}\nlast");
        fs::write(&p, data.as_bytes()).unwrap();
        let mut rows = Vec::new();
        reverse_lines(File::open(&p).unwrap(), None, |off, b| {
            rows.push((off, String::from_utf8_lossy(b).into_owned()));
            Ok(true)
        })
        .unwrap();
        assert_eq!(rows[0], ((7 + big.len() + 1) as u64, "last".into()));
        assert_eq!(rows[1], (7, big));
        assert_eq!(rows[2], (0, "first".into()));
    }
    #[test]
    fn rejects_path_escape() {
        let d = tempfile::tempdir().unwrap();
        assert!(safe_join(d.path(), "../outside").is_err());
        assert!(safe_join(d.path(), "a/../../outside").is_err());
        assert!(safe_join(d.path(), "a/file.jsonl").is_ok());
    }
}
