use crate::{
    pack,
    provider::{OpContext, Provider},
    storage::atomic_write,
};
use anyhow::{bail, ensure, Context, Result};
use chrono::{DateTime, SecondsFormat, Utc};
use rusqlite::{Connection, OpenFlags};
use serde_json::{json, Value};
use std::{
    fs,
    path::{Path, PathBuf},
    time::Duration,
};
use uuid::Uuid;

mod antigravity;
mod claude;
mod codex;
mod opencode;

pub fn export_provider_chat(src: &dyn Provider, cid: &str, include_tools: bool) -> Result<Value> {
    src.call(
        "export_chat",
        &json!({"cid":cid,"include_tools":include_tools}),
        &OpContext::default(),
    )
}

pub fn import_doc_to_provider(
    doc: &Value,
    dst: &dyn Provider,
    title: Option<&str>,
    include_tools: bool,
) -> Result<Value> {
    dst.assert_writable(None)?;
    let p = dst.paths();
    let result = match p.id.as_str() {
        "antigravity" | "antigravity_cli" => antigravity::import(
            doc,
            &p.root,
            p.id == "antigravity_cli",
            title,
            include_tools,
        ),
        "codex" => codex::import(
            doc,
            &p.root,
            title,
            include_tools,
            configured_home(&p.root, "CODEX_HOME", ".codex"),
        ),
        "claude" => claude::import(
            doc,
            &p.root,
            title,
            include_tools,
            configured_home(&p.root, "CLAUDE_HOME", ".claude"),
            None,
        ),
        "opencode" => opencode::import(doc, &p.root.join("opencode.db"), title, include_tools),
        _ => bail!("Неизвестный целевой провайдер: {}", p.id),
    }
    .with_context(|| {
        format!(
            "Не удалось импортировать диалог в {}. Хранилище: «{}»",
            p.name,
            p.root.display()
        )
    })?;
    let _ = dst.call("convs", &json!({"refresh":true}), &OpContext::default());
    Ok(result)
}

pub fn convert_chat(
    src: &dyn Provider,
    cid: &str,
    dst: &dyn Provider,
    title: Option<&str>,
    include_tools: bool,
    ctx: &OpContext,
) -> Result<Value> {
    ctx.phase("read");
    let doc = export_provider_chat(src, cid, include_tools)?;
    ensure!(
        doc["entries"].as_array().is_some_and(|e| !e.is_empty()),
        "В выбранном диалоге нет сообщений для конвертации."
    );
    ctx.phase("write");
    let mut result = import_doc_to_provider(&doc, dst, title, include_tools)?;
    result["source_provider"] = json!(src.paths().id);
    result["source_id"] = json!(cid);
    ctx.phase("done");
    Ok(result)
}

pub fn export_to_k1tty_bytes(
    src: &dyn Provider,
    cid: &str,
    preset: &str,
    include_tools: bool,
) -> Result<(Vec<u8>, Value, String)> {
    let doc = export_provider_chat(src, cid, include_tools)?;
    ensure!(
        doc["entries"].as_array().is_some_and(|e| !e.is_empty()),
        "В выбранном диалоге нет сообщений для экспорта."
    );
    let (bytes, mut stats) = pack::pack_chat(&doc, preset, include_tools)?;
    let title = nonempty(s(&doc["meta"], "title")).unwrap_or_else(|| shorten(cid, 12));
    let re =
        regex::Regex::new(r"[^A-Za-zА-Яа-яЁёІіЇїЄєҐґ0-9._ -]+").expect("static filename pattern");
    let safe = shorten(
        re.replace_all(&title, "_").trim_matches([' ', '.', '_']),
        60,
    );
    let safe = if safe.is_empty() {
        shorten(cid, 12)
    } else {
        safe
    };
    stats["title"] = json!(title);
    stats["source_provider"] = json!(src.paths().id);
    stats["source_id"] = json!(cid);
    Ok((
        bytes,
        stats,
        format!("{safe}_{}.k1tty", src.paths().short.to_lowercase()),
    ))
}

pub fn import_k1tty_bytes(
    raw: &[u8],
    dst: &dyn Provider,
    title: Option<&str>,
    include_tools: bool,
) -> Result<Value> {
    let doc = pack::unpack_chat(raw)?;
    let mut result = import_doc_to_provider(&doc, dst, title, include_tools)?;
    result["source_provider"] = json!(s(&doc["meta"], "source_provider"));
    result["pack_info"] = doc["pack_info"].clone();
    Ok(result)
}

fn s(v: &Value, key: &str) -> String {
    match &v[key] {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        v => v.to_string(),
    }
}
fn nonempty(s: String) -> Option<String> {
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}
fn shorten(s: &str, n: usize) -> String {
    s.chars().take(n).collect()
}
fn now_sec() -> f64 {
    Utc::now().timestamp_micros() as f64 / 1_000_000.0
}
fn iso(sec: f64) -> String {
    let micros = (sec * 1_000_000.0).round() as i64;
    DateTime::<Utc>::from_timestamp_micros(micros)
        .map(|d| {
            d.to_rfc3339_opts(
                if micros % 1_000_000 == 0 {
                    SecondsFormat::Secs
                } else {
                    SecondsFormat::Micros
                },
                true,
            )
        })
        .unwrap_or_default()
}
fn ag_dt(sec: f64) -> String {
    DateTime::<Utc>::from_timestamp_micros((sec * 1_000_000.0).round() as i64)
        .map(|d| d.format("%Y-%m-%d %H:%M:%S%.6f+00:00").to_string())
        .unwrap_or_default()
}
fn home_dir() -> PathBuf {
    dirs::home_dir().unwrap_or_else(|| PathBuf::from("."))
}
fn configured_home(path: &Path, variable: &str, fallback: &str) -> bool {
    let configured = std::env::var_os(variable)
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| home_dir().join(fallback));
    if path == configured {
        return true;
    }
    match (path.canonicalize(), configured.canonicalize()) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    }
}
fn strip_long(s: &str) -> String {
    let s = s.trim();
    if s.get(..8)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case(r"\\?\UNC\"))
    {
        return format!(r"\\{}", &s[8..]);
    }
    if let Some(path) = s.strip_prefix(r"\\?\") {
        if path.as_bytes().first().is_some_and(u8::is_ascii_alphabetic)
            && path.as_bytes().get(1) == Some(&b':')
            && path.as_bytes().get(2) == Some(&b'\\')
        {
            return path.to_owned();
        }
    }
    s.to_owned()
}
fn lexical_normpath(path: &str) -> String {
    lexical_normpath_for(path, cfg!(windows))
}
fn lexical_normpath_for(path: &str, windows: bool) -> String {
    let path = if windows {
        path.replace('/', "\\")
    } else {
        path.to_owned()
    };
    let separator = if windows { '\\' } else { '/' };
    let (prefix, tail, rooted) = if windows && path.starts_with(r"\\") {
        let start = if path
            .get(..8)
            .is_some_and(|p| p.eq_ignore_ascii_case(r"\\?\UNC\"))
        {
            8
        } else {
            2
        };
        let share_end = path[start..].find('\\').and_then(|server_end| {
            let share_start = start + server_end + 1;
            path[share_start..].find('\\').map(|n| share_start + n)
        });
        if let Some(end) = share_end {
            (&path[..end + 1], &path[end + 1..], true)
        } else {
            (path.as_str(), "", false)
        }
    } else if path.starts_with(separator) {
        let root_len = if !windows && path.starts_with("//") && !path.starts_with("///") {
            2
        } else {
            1
        };
        (&path[..root_len], &path[root_len..], true)
    } else if let Some(drive_end) = windows
        .then(|| path.char_indices().nth(1))
        .flatten()
        .filter(|(_, ch)| *ch == ':')
        .map(|(i, _)| i + 1)
    {
        let rooted = path[drive_end..].starts_with('\\');
        let end = drive_end + usize::from(rooted);
        (&path[..end], &path[end..], rooted)
    } else {
        ("", path.as_str(), false)
    };
    let mut parts = Vec::new();
    for part in tail.split(separator) {
        match part {
            "" | "." => {}
            ".." if parts.last().is_some_and(|p| *p != "..") => {
                parts.pop();
            }
            ".." if rooted => {}
            _ => parts.push(part),
        }
    }
    let result = format!("{prefix}{}", parts.join(if windows { "\\" } else { "/" }));
    if result.is_empty() {
        ".".into()
    } else {
        result
    }
}
fn ephemeral(s: &str) -> bool {
    let p = s.replace('\\', "/").to_lowercase();
    (p.contains("/claude-data/") && p.contains("/scratch-workspaces/"))
        || p.contains("/claude/scratch-workspaces/")
        || p.contains("/documents/codex/")
}
fn filtered(doc: &Value, include_tools: bool) -> Result<Vec<Value>> {
    let entries = doc["entries"]
        .as_array()
        .context("В файле нет сообщений для импорта.")?;
    let out: Vec<_> = entries
        .iter()
        .filter(|e| {
            let text = s(e, "text");
            if include_tools {
                !text.trim().is_empty() || !s(e, "tool").is_empty()
            } else {
                matches!(s(e, "kind").as_str(), "user" | "assistant" | "compact")
                    && !text.trim().is_empty()
            }
        })
        .cloned()
        .collect();
    ensure!(!out.is_empty(), "В файле нет сообщений для импорта.");
    Ok(out)
}
fn derive_title(entries: &[Value], preferred: Option<&str>) -> String {
    if let Some(p) = preferred.filter(|v| !v.trim().is_empty()) {
        return shorten(p.trim(), 140);
    }
    for e in entries {
        if e["kind"] == "user" {
            let text = s(e, "text")
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ");
            if !text.is_empty() {
                return shorten(&text, 90);
            }
        }
    }
    "Imported chat".into()
}
fn title_for(doc: &Value, entries: &[Value], title: Option<&str>) -> String {
    derive_title(
        entries,
        title
            .filter(|s| !s.is_empty())
            .or_else(|| doc["meta"]["title"].as_str()),
    )
}
fn timestamps(entries: &[Value], now: f64) -> Vec<f64> {
    let base = now - entries.len().max(2) as f64 * 2.0;
    let mut prev = 0.0;
    let mut result = Vec::new();
    for (idx, e) in entries.iter().enumerate() {
        let fallback = if prev > 0.0 {
            prev + 1.0
        } else {
            base + idx as f64 * 1.5
        };
        let mut sec = if let Some(n) = e["ts"].as_f64() {
            if n > 10_000_000_000.0 {
                n / 1000.0
            } else {
                n
            }
        } else if let Some(st) = e["ts"].as_str() {
            DateTime::parse_from_rfc3339(st.trim())
                .map(|d| d.timestamp() as f64 + d.timestamp_subsec_nanos() as f64 / 1e9)
                .or_else(|_| {
                    chrono::NaiveDateTime::parse_from_str(st.trim(), "%Y-%m-%dT%H:%M:%S%.f")
                        .map(|d| d.and_utc().timestamp_micros() as f64 / 1e6)
                })
                .unwrap_or(fallback)
        } else {
            fallback
        };
        if sec <= 946_684_800.0 || !sec.is_finite() {
            sec = fallback;
        }
        if prev > 0.0 && sec < prev {
            sec = prev + 0.05;
        }
        result.push(sec);
        prev = sec;
    }
    result
}
fn uuid() -> String {
    Uuid::new_v4().to_string()
}
fn uuid_hex() -> String {
    Uuid::new_v4().simple().to_string()
}
fn uuid7(ms: i64) -> String {
    let mut b = *Uuid::new_v4().as_bytes();
    let t = (ms as u64).to_be_bytes();
    b[..6].copy_from_slice(&t[2..]);
    b[6] = 0x70 | (b[6] & 15);
    b[8] = 0x80 | (b[8] & 63);
    Uuid::from_bytes(b).to_string()
}
fn connection(path: &Path) -> Result<Connection> {
    if let Some(p) = path.parent() {
        fs::create_dir_all(p)?;
    }
    let c = Connection::open(path)?;
    c.busy_timeout(Duration::from_secs(30))?;
    Ok(c)
}
fn readonly(path: &Path) -> Result<Connection> {
    let c = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    c.busy_timeout(Duration::from_secs(5))?;
    Ok(c)
}
fn write_json(path: &Path, value: &Value) -> Result<()> {
    atomic_write(path, &serde_json::to_vec(value)?)
}
fn write_lines(path: &Path, lines: &[Value]) -> Result<()> {
    let mut bytes = Vec::new();
    for v in lines {
        serde_json::to_writer(&mut bytes, v)?;
        bytes.push(b'\n');
    }
    atomic_write(path, &bytes)
}
fn recent_files(root: &Path, depth: usize, extension: &str) -> Vec<PathBuf> {
    let mut files: Vec<_> = walkdir::WalkDir::new(root)
        .max_depth(depth)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file() && e.path().extension().is_some_and(|x| x == extension))
        .map(|e| e.into_path())
        .collect();
    files.sort_by_key(|p| std::cmp::Reverse(p.metadata().ok().and_then(|m| m.modified().ok())));
    files
}

#[derive(Default)]
struct ImportGuard {
    done: bool,
    paths: Vec<PathBuf>,
    database_rows: Vec<(PathBuf, &'static str, &'static str, String)>,
    indexes: Vec<(PathBuf, String, String, Option<Vec<u8>>, Vec<u8>)>,
}
impl ImportGuard {
    fn own(&mut self, path: PathBuf) {
        if fs::symlink_metadata(&path).is_err_and(|e| e.kind() == std::io::ErrorKind::NotFound) {
            self.paths.push(path);
        }
    }
    fn row(&mut self, path: PathBuf, table: &'static str, column: &'static str, id: &str) {
        self.database_rows
            .push((path, table, column, id.to_owned()));
    }
    fn index(
        &mut self,
        path: PathBuf,
        key: &str,
        id: &str,
        before: Option<Vec<u8>>,
        written: Vec<u8>,
    ) {
        self.indexes
            .push((path, key.to_owned(), id.to_owned(), before, written));
    }
    fn commit(mut self) {
        self.done = true;
    }
}
impl Drop for ImportGuard {
    fn drop(&mut self) {
        if self.done {
            return;
        }
        for (path, table, column, id) in &self.database_rows {
            if let Ok(c) = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_WRITE) {
                let _ = c.busy_timeout(Duration::from_secs(2));
                let _ = c.execute(&format!("DELETE FROM {table} WHERE {column}=?"), [id]);
            }
        }
        for (path, key, id, before, written) in &self.indexes {
            if let Ok(current) = fs::read(path) {
                if &current == written {
                    if let Some(before) = before {
                        let _ = atomic_write(path, before);
                    } else {
                        let _ = fs::remove_file(path);
                    }
                } else {
                    let mut kept = Vec::new();
                    for line in current.split_inclusive(|b| *b == b'\n') {
                        let owned = serde_json::from_slice::<Value>(line).ok().is_some_and(|v| {
                            v.get(key).and_then(Value::as_str) == Some(id.as_str())
                        });
                        if !owned {
                            kept.extend_from_slice(line);
                        }
                    }
                    if kept != current {
                        let _ = atomic_write(path, &kept);
                    }
                }
            }
        }
        for path in self.paths.iter().rev() {
            if let Ok(meta) = fs::symlink_metadata(path) {
                if meta.is_dir() {
                    let _ = fs::remove_dir_all(path);
                } else {
                    let _ = fs::remove_file(path);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests;
