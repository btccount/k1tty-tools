use crate::{
    jsonl as j,
    provider::{truncate_preview_item, EntryRef, OpContext, Provider, ProviderPaths},
};
use anyhow::{bail, Context, Result};
use chrono::Utc;
#[cfg(unix)]
use fs2::FileExt;
use regex::Regex;
use rusqlite::{params, params_from_iter, Connection, OpenFlags, OptionalExtension};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    borrow::Cow,
    collections::{BTreeMap, HashMap, HashSet},
    fs::{self, File, OpenOptions},
    io::{BufRead, BufReader, BufWriter, Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
    time::{Duration, UNIX_EPOCH},
};
const TABLES: [&str; 4] = [
    "thread_items",
    "thread_turns",
    "thread_realtime_items",
    "thread_history_projection_state",
];
const PAGE_BYTE_BUDGET: usize = 2 * 1024 * 1024;

pub struct CodexProvider {
    paths: ProviderPaths,
    path_cache: Mutex<HashMap<String, Vec<PathBuf>>>,
    title_cache: Mutex<HashMap<PathBuf, (u64, u128, String, String)>>,
}
impl CodexProvider {
    pub fn new(paths: ProviderPaths) -> Self {
        Self {
            paths,
            path_cache: Mutex::new(HashMap::new()),
            title_cache: Mutex::new(HashMap::new()),
        }
    }
    fn invalidate_paths(&self, cid: Option<&str>) {
        let mut cache = self.path_cache.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(cid) = cid {
            cache.remove(cid);
        } else {
            cache.clear();
        }
    }
    fn sessions(&self) -> PathBuf {
        self.paths.root.join("sessions")
    }
    fn projection(&self) -> PathBuf {
        self.paths.root.join("thread_history_1.sqlite")
    }
    fn index(&self) -> PathBuf {
        self.paths.root.join("session_index.jsonl")
    }
    fn test_mode(&self) -> bool {
        matches!(
            self.paths
                .root
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_lowercase()
                .as_str(),
            "sandbox" | "codex_sandbox"
        )
    }
    fn running(&self) -> Vec<String> {
        crate::process::snapshot()
            .remove("codex")
            .unwrap_or_default()
    }
    fn all_files(&self) -> Vec<PathBuf> {
        j::paths(&self.sessions(), "jsonl")
            .into_iter()
            .filter(|p| {
                p.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.starts_with("rollout-"))
            })
            .collect()
    }
    fn cached_file_meta(&self, p: &Path) -> Result<(String, String, u64, f64)> {
        let meta = p.metadata()?;
        let len = meta.len();
        let mtime_dur = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok());
        let nanos = mtime_dur.map(|d| d.as_nanos()).unwrap_or(0);
        let ts = mtime_dur.map(|d| d.as_secs_f64()).unwrap_or(0.);
        if let Some((id, stamp)) = self
            .title_cache
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(p)
            .and_then(|(l, m, id, st)| (*l == len && *m == nanos).then(|| (id.clone(), st.clone())))
        {
            return Ok((id, stamp, len, ts));
        }
        let (id, stamp) = file_meta(p)?;
        self.title_cache
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(p.to_path_buf(), (len, nanos, id.clone(), stamp.clone()));
        Ok((id, stamp, len, ts))
    }
    fn files(&self, cid: &str) -> Result<Vec<PathBuf>> {
        j::valid_id(cid)?;
        if let Some(paths) = self
            .path_cache
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(cid)
            .cloned()
        {
            if !paths.is_empty() && paths.iter().all(|p| p.is_file()) {
                return Ok(paths);
            }
        }
        let mut groups = HashMap::<String, Vec<PathBuf>>::new();
        for p in self.all_files() {
            if let Ok((id, _, _, _)) = self.cached_file_meta(&p) {
                if !id.is_empty() {
                    groups.entry(id).or_default().push(p);
                }
            }
        }
        let found = groups.get(cid).cloned().unwrap_or_default();
        let mut cache = self.path_cache.lock().unwrap_or_else(|e| e.into_inner());
        for (k, v) in groups {
            cache.insert(k, v);
        }
        Ok(found)
    }
    fn title(&self, cid: &str) -> String {
        self.list()
            .unwrap_or_default()
            .into_iter()
            .find(|v| j::s(v, "id") == cid)
            .map(|v| j::s(&v, "title").into())
            .unwrap_or_else(|| cid.into())
    }
    fn revision_files(&self, files: &[PathBuf]) -> Result<String> {
        let mut h = Sha256::new();
        let full = self.test_mode() && files.len() != 1;
        for p in files {
            if full {
                h.update(
                    p.strip_prefix(self.sessions())?
                        .to_string_lossy()
                        .as_bytes(),
                );
                let mut reader = File::open(p)?;
                let mut buffer = [0_u8; 65_536];
                loop {
                    let n = reader.read(&mut buffer)?;
                    if n == 0 {
                        break;
                    }
                    h.update(&buffer[..n]);
                }
            } else {
                let metadata = p.metadata()?;
                let mtime = metadata
                    .modified()?
                    .duration_since(std::time::UNIX_EPOCH)?
                    .as_nanos();
                #[cfg(unix)]
                let ctime = {
                    use std::os::unix::fs::MetadataExt;
                    metadata.ctime() as i128 * 1_000_000_000 + metadata.ctime_nsec() as i128
                };
                #[cfg(not(unix))]
                let ctime = metadata
                    .created()
                    .or_else(|_| metadata.modified())?
                    .duration_since(std::time::UNIX_EPOCH)?
                    .as_nanos() as i128;
                h.update(
                    format!("{}|{}|{}|{}", p.display(), metadata.len(), mtime, ctime).as_bytes(),
                );
            }
        }
        Ok(format!("{:x}", h.finalize()))
    }
    #[cfg(test)]
    fn revision(&self, cid: &str) -> Result<String> {
        let files = self.files(cid)?;
        self.revision_files(&files)
    }
    fn check_revision_files(&self, files: &[PathBuf], rev: &str) -> Result<()> {
        if rev.is_empty() || self.revision_files(files)? != rev {
            bail!("История изменилась после предпросмотра.")
        }
        Ok(())
    }
    fn writable_files(&self, files: &[PathBuf]) -> bool {
        self.test_mode() || files.len() == 1
    }
    fn probe_lock_files(&self, files: Option<&[PathBuf]>) -> Result<Option<String>> {
        if let Some(files) = files {
            for p in files {
                if j::LockedFile::open(p, true).is_err() {
                    return Ok(Some("Файл диалога занят Codex. Закройте этот тред в Codex или закройте приложение.".into()));
                }
            }
        }
        if self.projection().is_file() {
            let probe = (|| -> Result<()> {
                let c = Connection::open(self.projection())?;
                c.busy_timeout(Duration::from_millis(100))?;
                c.execute_batch("BEGIN IMMEDIATE;ROLLBACK")?;
                Ok(())
            })();
            if probe.is_err() {
                return Ok(Some(
                    "База проекций Codex занята. Закройте Codex и повторите.".into(),
                ));
            }
        }
        Ok(None)
    }
    fn list(&self) -> Result<Vec<Value>> {
        static UUID_RE: OnceLock<Regex> = OnceLock::new();
        let mut titles = HashMap::<String, String>::new();
        if self.index().is_file() {
            for l in j::read_lines(&self.index())? {
                if let Ok(v) = l.obj() {
                    titles.insert(j::s(&v, "id").into(), j::s(&v, "thread_name").into());
                }
            }
        }
        let mut groups = BTreeMap::<String, Vec<(PathBuf, u64, f64)>>::new();
        for p in self.all_files() {
            if let Ok((id, _, len, ts)) = self.cached_file_meta(&p) {
                if !id.is_empty() {
                    groups.entry(id).or_default().push((p, len, ts));
                }
            }
        }
        {
            let mut cache = self.path_cache.lock().unwrap_or_else(|e| e.into_inner());
            cache.clear();
            for (cid, entries) in &groups {
                cache.insert(
                    cid.clone(),
                    entries.iter().map(|(p, _, _)| p.clone()).collect(),
                );
            }
        }
        let uuid = UUID_RE.get_or_init(|| {
            Regex::new(r"[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}").unwrap()
        });
        let mut out = Vec::new();
        for (cid, entries) in groups {
            let mut title = None;
            for (p, _, _) in &entries {
                if let Some(m) = uuid
                    .find_iter(&p.file_stem().unwrap_or_default().to_string_lossy())
                    .last()
                {
                    if let Some(t) = titles.get(m.as_str()).filter(|s| !s.is_empty()) {
                        title = Some(t.clone());
                        break;
                    }
                }
            }
            let title = title
                .or_else(|| titles.get(&cid).cloned().filter(|s| !s.is_empty()))
                .unwrap_or_else(|| "(без названия)".into());
            let size: u64 = entries.iter().map(|(_, len, _)| *len).sum();
            let time = entries.iter().map(|(_, _, ts)| *ts).fold(0f64, f64::max);
            let count = entries.len();
            out.push(json!({"provider":"codex","id":cid,"title":title,"subtitle":format!("rollout-файлов: {count}{}",if count==1{""}else{" (только чтение)"}),"badge":format!("{count} ф."),"size":size,"mtime":j::display_time(time),"mtime_ts":time,"writable":self.test_mode()||count==1}));
        }
        out.sort_by(|a, b| {
            b["mtime_ts"]
                .as_f64()
                .partial_cmp(&a["mtime_ts"].as_f64())
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        Ok(out)
    }
    fn entries(&self, data: &Value) -> Result<Value> {
        let cid = j::required(data, "cid")?;
        let files = self.files(cid)?;
        let writable = self.writable_files(&files);
        let (items, cursor, msgs, has_newer) =
            if let Some(around) = data["around_key"].as_str().filter(|s| !s.is_empty()) {
                read_page_around(&files, data, writable, around)?
            } else {
                let (items, cursor, msgs) = read_page(&files, data, writable)?;
                (items, cursor, msgs, false)
            };
        let lock = self.probe_lock_files(Some(&files))?;
        let mut resp = json!({"provider":"codex","cid":cid,"turns":j::turns(items),"page_msgs":msgs,"total_label":format!("загружено: {msgs}"),"cursor":cursor,"has_more":cursor.is_some(),"writable":writable,"file_locked":lock.is_some(),"lock_reason":lock.unwrap_or_default(),"blocked_info":self.detect_block_files(cid,&files)?,"block_presets":block_presets()});
        if has_newer {
            resp["has_newer"] = json!(true);
        }
        Ok(resp)
    }
    fn edit_preview(&self, data: &Value) -> Result<Value> {
        let cid = j::required(data, "cid")?;
        let files = self.files(cid)?;
        let rev = self.revision_files(&files)?;
        let (fi, off) = byte_key(&files, j::required(data, "key")?)?;
        let row = j::read_at(&files[fi], off)?.obj()?;
        let info = edit_info(&row)?;
        self.check_revision_files(&files, &rev)?;
        let writable = self.writable_files(&files);
        Ok(
            json!({"text":info["old"],"kind":info["label"],"hint":if writable{"Текст обновится в rollout-файле с удержанием блокировок, проекция треда будет сброшена. Перед записью создаётся проверенная резервная копия."}else{"Правка недоступна: нужен однофайловый rollout и поддерживаемая платформа."},"revision":if writable{Some(rev)}else{None},"writable":writable}),
        )
    }
    fn preview_cut(&self, data: &Value) -> Result<Value> {
        let cid = j::required(data, "cid")?;
        let files = self.files(cid)?;
        let mode = j::s(data, "mode");
        let rev = self.revision_files(&files)?;
        let ranges = cut_ranges(&files, j::required(data, "key")?, mode)?;
        let raw = cut_report(cid, mode, &files, &ranges)?;
        self.check_revision_files(&files, &rev)?;
        let writable = self.writable_files(&files);
        let preview = raw["will_delete"]["steps_preview"]
            .as_array()
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .map(|mut item| {
                item["text"] = json!(j::truncate(&j::s(&item, "text").replace('\n', " "), 130));
                item
            })
            .collect::<Vec<_>>();
        Ok(
            json!({"report":{"provider":"codex","mode":mode,"steps_count":raw["will_delete"]["steps_preview_count"],"raw_count":raw["rollout_lines"],"raw_label":"строк rollout","bytes":raw["bytes"],"extra_label":format!("файлов целиком: {}",raw["rollout_files"]),"count_incomplete":raw["count_incomplete"],"steps_preview":preview,"raw":raw},"revision":if writable{Some(rev)}else{None},"writable":writable,"hint":if writable{"Будут удалены записи в rollout-файле и сброшена проекция десктоп-приложения. Перед изменением создаётся SHA256-проверенная копия rollout и проекции."}else{"Изменение многосегментного треда вне песочницы недоступно (только чтение)."}}),
        )
    }
    fn backup_projection(&self, cid: &str, dest: &Path) -> Result<()> {
        if !self.projection().is_file() {
            return Ok(());
        }
        let src = Connection::open_with_flags(self.projection(), OpenFlags::SQLITE_OPEN_READ_ONLY)?;
        src.busy_timeout(Duration::from_millis(1000))?;
        let dst = Connection::open(dest)?;
        src.execute_batch("BEGIN")?;
        dst.execute_batch("BEGIN")?;
        for table in TABLES {
            let sql: String = src
                .query_row(
                    "SELECT sql FROM sqlite_master WHERE type='table' AND name=?",
                    [table],
                    |r| r.get(0),
                )
                .with_context(|| format!("Не найдена таблица проекции: {table}"))?;
            dst.execute_batch(&sql)?;
            let mut stmt = src.prepare(&format!("SELECT * FROM \"{table}\" WHERE thread_id=?"))?;
            let cols = stmt.column_count();
            let mut rows = stmt.query([cid])?;
            let insert = format!(
                "INSERT INTO \"{table}\" VALUES ({})",
                vec!["?"; cols].join(",")
            );
            while let Some(row) = rows.next()? {
                let values: Vec<rusqlite::types::Value> = (0..cols)
                    .map(|i| row.get(i))
                    .collect::<std::result::Result<_, _>>()?;
                dst.execute(&insert, params_from_iter(values))?;
            }
        }
        dst.execute_batch("COMMIT")?;
        let check: String = dst.query_row("PRAGMA quick_check", [], |r| r.get(0))?;
        if check != "ok" {
            bail!("Резервная копия проекции повреждена: {check}")
        }
        Ok(())
    }
    fn backup_files_for(&self, cid: &str, tag: &str, files: &[PathBuf]) -> Result<PathBuf> {
        if files.is_empty() {
            bail!("Тред {cid} не найден")
        };
        let bdir = self
            .paths
            .backup_root
            .join(cid)
            .join(format!("{}-{tag}", j::stamp()));
        fs::create_dir_all(&bdir)?;
        let mut hashes = serde_json::Map::new();
        for p in files {
            let rel = p.strip_prefix(self.sessions())?;
            let (h, _, _) = j::copy_checked_with_stats(p, &bdir.join("sessions").join(rel))?;
            hashes.insert(rel.to_string_lossy().into_owned(), json!(h));
        }
        let snapshot = bdir.join("projection.sqlite");
        self.backup_projection(cid, &snapshot)?;
        let phash = if snapshot.is_file() {
            j::hash_file(&snapshot)?
        } else {
            String::new()
        };
        let hash = if hashes.len() == 1 {
            hashes.values().next().unwrap().clone()
        } else {
            json!("")
        };
        let mut meta = json!({"tid":cid,"created":j::stamp(),"files":files.len(),"tag":tag,"sha256":hash,"file_hashes":hashes,"projection_sha256":phash});
        if tag == "delete" && self.index().is_file() {
            let stems: Vec<_> = files
                .iter()
                .filter_map(|p| p.file_stem().and_then(|s| s.to_str()))
                .collect();
            let idx: Vec<_> = j::read_lines(&self.index())?
                .into_iter()
                .filter(|l| {
                    l.obj().is_ok_and(|v| {
                        let id = j::s(&v, "id");
                        id == cid || (!id.is_empty() && stems.iter().any(|s| s.contains(id)))
                    })
                })
                .map(|l| {
                    String::from_utf8_lossy(&l.raw)
                        .trim_end_matches(['\r', '\n'])
                        .to_string()
                })
                .collect();
            meta["session_index_lines"] = json!(idx);
        }
        j::save_json(&bdir.join("backup.meta.json"), &meta)?;
        Ok(bdir)
    }
    fn projection_connection(&self) -> Result<Option<Connection>> {
        if !self.projection().is_file() {
            return Ok(None);
        }
        let c = Connection::open(self.projection())?;
        c.busy_timeout(Duration::from_secs(3))?;
        Ok(Some(c))
    }
    fn sync_state_db_delete(&self, cid: &str) {
        let db = self.paths.root.join("state_5.sqlite");
        if !db.is_file() {
            return;
        }
        if let Ok(c) = Connection::open(&db) {
            let _ = c.busy_timeout(Duration::from_millis(3000));
            let _ = c.execute("DELETE FROM threads WHERE id = ?", [cid]);
            let _ = c.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);");
        }
    }
    fn sync_state_db_touch(&self, cid: &str) {
        let db = self.paths.root.join("state_5.sqlite");
        if !db.is_file() {
            return;
        }
        if let Ok(c) = Connection::open(&db) {
            let _ = c.busy_timeout(Duration::from_millis(1000));
            let now_ts = Utc::now().timestamp();
            let _ = c.execute(
                "UPDATE threads SET updated_at = ? WHERE id = ?",
                params![now_ts, cid],
            );
            let _ = c.execute_batch("PRAGMA wal_checkpoint(PASSIVE);");
        }
    }
    fn sync_global_state_delete(&self, cid: &str) {
        let path = self.paths.root.join(".codex-global-state.json");
        if !path.is_file() {
            return;
        }
        if let Ok(mut v) = j::load_json(&path) {
            if strip_tid_refs(&mut v, cid) {
                if let Ok(bytes) = serde_json::to_vec(&v) {
                    let _ = crate::storage::atomic_write(&path, &bytes);
                }
            }
        }
    }
    fn remove_writer_lock_if_unlocked(&self, cid: &str) {
        let lock_path = self
            .paths
            .root
            .join("thread-writer-locks")
            .join(format!("{cid}.lock"));
        if !lock_path.is_file() {
            return;
        }
        #[cfg(unix)]
        {
            if let Ok(f) = OpenOptions::new()
                .read(true)
                .write(true)
                .open(&lock_path)
                .or_else(|_| File::open(&lock_path))
            {
                if f.try_lock_exclusive().is_ok() {
                    let _ = FileExt::unlock(&f);
                    drop(f);
                    let _ = fs::remove_file(&lock_path);
                }
            }
        }
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            if let Ok(f) = OpenOptions::new()
                .read(true)
                .write(true)
                .share_mode(0)
                .open(&lock_path)
            {
                drop(f);
                let _ = fs::remove_file(&lock_path);
            }
        }
    }
    fn mutate(&self, op: &str, data: &Value, ctx: &OpContext) -> Result<Value> {
        let cid = j::required(data, "cid")?;
        self.assert_writable(Some(cid))?;
        let files = self.files(cid)?;
        if files.is_empty() {
            bail!("Тред {cid} не найден")
        };
        if files.len() != 1 && !self.test_mode() {
            bail!("Изменение многосегментного треда вне песочницы недоступно (только чтение).")
        };
        let mut guards: Vec<_> = files
            .iter()
            .map(|p| j::LockedFile::open(p, true))
            .collect::<Result<_>>()?;
        self.check_revision_files(&files, j::s(data, "revision"))?;
        let mode = if op == "apply_edit" {
            "edit"
        } else {
            j::s(data, "mode")
        };
        let key = j::required(data, "key")?;
        let (fi, off) = byte_key(&files, key)?;
        for p in &files {
            validate_ordinals(p)?;
        }
        let ranges = if mode == "edit" {
            Vec::new()
        } else {
            cut_ranges(&files, key, mode)?
        };
        if mode == "head"
            && files.len() > 1
            && j::read_at(&files[fi], 0)?.obj()?.get("ordinal").is_some()
        {
            bail!("Обрезка начала многосегментного треда пока не поддерживается")
        };
        if mode == "tail" && files.len() == 1 && ranges.iter().any(|r| r.1 == 0) {
            bail!("Удаление всего файла треда не поддерживается")
        }
        let mut report = if mode == "edit" {
            json!({"thread":cid,"mode":"edit"})
        } else {
            cut_report(cid, mode, &files, &ranges)?
        };
        let mut candidates: Vec<Option<tempfile::NamedTempFile>> = Vec::new();
        for (idx, p) in files.iter().enumerate() {
            let mut tmp = tempfile::NamedTempFile::new_in(p.parent().unwrap())?;
            if mode == "edit" {
                if idx == fi {
                    let text = j::s(data, "text");
                    if text.trim().is_empty() {
                        bail!("Пустой текст, сохранение не выполнено.")
                    }
                    let r = edit_candidate(p, tmp.as_file_mut(), off, text)?;
                    report
                        .as_object_mut()
                        .unwrap()
                        .extend(r.as_object().unwrap().clone());
                } else {
                    std::io::copy(&mut File::open(p)?, tmp.as_file_mut())?;
                }
            } else {
                let my: Vec<_> = ranges.iter().filter(|r| r.0 == idx).copied().collect();
                if my
                    .iter()
                    .any(|r| r.1 == 0 && r.2 == p.metadata().map(|m| m.len()).unwrap_or(0))
                {
                    candidates.push(None);
                    continue;
                }
                cut_candidate(p, tmp.as_file_mut(), &my, mode == "head" && idx == fi)?;
            }
            tmp.as_file().sync_all()?;
            candidates.push(Some(tmp));
        }
        let backup = if ctx.auto_backup {
            ctx.phase("backup");
            Some(self.backup_files_for(cid, "stream", &files)?)
        } else {
            None
        };
        let marker = backup.as_ref().map(|b| b.join("transaction.json"));
        if let Some(p) = &marker {
            j::save_json(p, &json!({"status":"prepared","operation":mode}))?;
        }
        let con = self.projection_connection()?;
        if let Some(c) = &con {
            c.execute_batch("BEGIN IMMEDIATE")?;
        }
        let mut changed = Vec::new();
        let mut undo = Vec::new();
        if backup.is_none() {
            for p in &files {
                let mut t = tempfile::NamedTempFile::new_in(p.parent().unwrap())?;
                let mut src = File::open(p)?;
                let mut buf = vec![0u8; 256 * 1024];
                loop {
                    let n = src.read(&mut buf)?;
                    if n == 0 {
                        break;
                    }
                    t.write_all(&buf[..n])?;
                }
                t.as_file().sync_all()?;
                undo.push(t);
            }
        }
        ctx.phase(if mode == "edit" { "edit" } else { "cut" });
        let result = (|| -> Result<Value> {
            for (i, candidate) in candidates.iter().enumerate() {
                if let Some(c) = candidate {
                    changed.push(i);
                    guards[i].replace_from(c.path())?;
                }
            }
            let mut counts = serde_json::Map::new();
            if let Some(c) = &con {
                for table in TABLES {
                    let count =
                        c.execute(&format!("DELETE FROM \"{table}\" WHERE thread_id=?"), [cid])?;
                    counts.insert(table.into(), json!(count));
                }
                c.execute_batch("COMMIT")?;
                wal_checkpoint_truncate(c);
            }
            Ok(Value::Object(counts))
        })();
        match result {
            Ok(counts) => {
                report["projection"] = counts;
            }
            Err(error) => {
                if let Some(c) = &con {
                    let _ = c.execute_batch("ROLLBACK");
                }
                for i in changed {
                    if let Some(bdir) = &backup {
                        let rel = files[i].strip_prefix(self.sessions())?;
                        let saved = bdir.join("sessions").join(rel);
                        if saved.is_file() {
                            guards[i].replace_from(&saved).context(
                                "Не удалось вернуть исходный rollout после ошибки записи",
                            )?;
                        }
                    } else if let Some(u) = undo.get(i) {
                        guards[i]
                            .replace_from(u.path())
                            .context("Не удалось вернуть исходный rollout после ошибки записи")?;
                    }
                }
                return Err(error);
            }
        }
        drop(guards);
        self.sync_state_db_touch(cid);
        let mut removed_any_file = false;
        for (i, candidate) in candidates.iter().enumerate() {
            if candidate.is_none() {
                fs::remove_file(&files[i])?;
                removed_any_file = true;
            }
        }
        if removed_any_file {
            self.invalidate_paths(Some(cid));
        }
        if let Some(p) = marker {
            j::save_json(&p, &json!({"status":"complete","operation":mode}))?;
        }
        report["backup"] = json!(backup);
        report["leaks"] = Value::Null;
        report["warns"] = json!([if self.test_mode() {
            "ТЕСТ-РЕЖИМ: изменения пишутся в песочницу."
        } else {
            "После изменения заново откройте тред в Codex; перестроение проекции может занять время."
        }]);
        ctx.phase("done");
        Ok(report)
    }
    fn delete_conv(&self, data: &Value, ctx: &OpContext) -> Result<Value> {
        let cid = j::required(data, "cid")?;
        self.assert_writable(Some(cid))?;
        let files = self.files(cid)?;
        if files.is_empty() {
            bail!("Тред {cid} не найден")
        };
        let guards: Vec<_> = files
            .iter()
            .map(|p| j::LockedFile::open(p, true))
            .collect::<Result<_>>()?;
        let backup = if ctx.auto_backup {
            ctx.phase("backup");
            Some(self.backup_files_for(cid, "delete", &files)?)
        } else {
            None
        };
        ctx.phase("cut");
        if let Some(c) = self.projection_connection()? {
            c.execute_batch("BEGIN IMMEDIATE")?;
            for table in TABLES {
                c.execute(&format!("DELETE FROM \"{table}\" WHERE thread_id=?"), [cid])?;
            }
            c.execute_batch("COMMIT")?;
            wal_checkpoint_truncate(&c);
        }
        drop(guards);
        for p in &files {
            fs::remove_file(p)?;
        }
        self.invalidate_paths(Some(cid));
        if self.index().is_file() {
            let stems: Vec<_> = files
                .iter()
                .filter_map(|p| p.file_stem().and_then(|s| s.to_str()))
                .collect();
            let kept: Vec<_> = j::read_lines(&self.index())?
                .into_iter()
                .filter(|l| {
                    !l.obj().is_ok_and(|v| {
                        let id = j::s(&v, "id");
                        id == cid || (!id.is_empty() && stems.iter().any(|s| s.contains(id)))
                    })
                })
                .collect();
            j::atomic_write(&self.index(), &j::write_lines(&kept))?;
        }
        self.sync_state_db_delete(cid);
        self.sync_global_state_delete(cid);
        self.remove_writer_lock_if_unlocked(cid);
        ctx.phase("done");
        Ok(
            json!({"deleted":cid,"provider":"codex","backup":backup,"files_removed":files.len(),"warns":if self.running().is_empty(){Vec::<String>::new()}else{vec!["Обнаружен запущенный Codex. Перезапустите приложение для обновления списка.".into()]} }),
        )
    }
    fn backup_files(&self, bdir: &Path) -> Vec<PathBuf> {
        let mut files = j::paths(&bdir.join("sessions"), "jsonl")
            .into_iter()
            .filter(|p| {
                p.file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .starts_with("rollout-")
            })
            .collect::<Vec<_>>();
        if files.is_empty() {
            files = j::paths(bdir, "jsonl")
                .into_iter()
                .filter(|p| {
                    p.parent() == Some(bdir)
                        && p.file_name()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .starts_with("rollout")
                })
                .collect();
        }
        files
    }
    fn backups(&self, data: &Value) -> Result<Value> {
        let cid = j::s(data, "cid");
        if !cid.is_empty() {
            j::valid_id(cid)?;
        }
        let roots = if cid.is_empty() {
            j::dirs(&self.paths.backup_root)
        } else {
            vec![self.paths.backup_root.join(cid)]
        };
        let (mut groups, mut count, mut size) = (Vec::new(), 0, 0);
        for dir in roots {
            let cid = dir.file_name().unwrap_or_default().to_string_lossy();
            let mut rows = Vec::new();
            for bdir in j::dirs(&dir) {
                let has_meta = bdir.join("backup.meta.json").is_file();
                let meta = j::load_json(&bdir.join("backup.meta.json")).unwrap_or(json!({}));
                let tag = j::s(&meta, "tag");
                let restorable = has_meta
                    && (tag == "delete"
                        || meta["tail_backup"].is_object()
                        || matches!(tag, "k1tty_unlock" | "k1tty_lock")
                        || (!j::s(&meta, "sha256").is_empty()
                            && (!j::s(&meta, "projection_sha256").is_empty()
                                || !bdir.join("projection.sqlite").is_file()))
                        || self.test_mode());
                let sz = j::size(&bdir);
                count += 1;
                size += sz;
                rows.push(json!({"provider":"codex","cid":cid,"dir":bdir,"name":bdir.file_name().unwrap_or_default().to_string_lossy(),"size":sz,"meta":meta,"restorable":restorable,"inspectable":!self.backup_files(&bdir).is_empty(),"reason":if restorable{""}else if !has_meta{"Копия без backup.meta.json"}else{"Старая копия без контрольных сумм требует ручного восстановления"}}));
            }
            if !rows.is_empty() {
                groups.push(
                    json!({"provider":"codex","cid":cid,"title":self.title(&cid),"backups":rows}),
                );
            }
        }
        Ok(
            json!({"groups":groups,"root":self.paths.backup_root,"total_count":count,"total_bytes":size}),
        )
    }
    fn backup_entries(&self, data: &Value) -> Result<Value> {
        let bdir = j::safe_backup(
            &self.paths.backup_root,
            Path::new(j::required(data, "backup_dir")?),
        )?;
        let files = self.backup_files(&bdir);
        if files.is_empty() {
            bail!("В этой резервной копии нет полного файла rollout для просмотра.")
        }
        let meta = j::load_json(&bdir.join("backup.meta.json")).unwrap_or(json!({}));
        let cid = if !j::s(&meta, "tid").is_empty() {
            j::s(&meta, "tid").into()
        } else {
            bdir.parent()
                .unwrap()
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned()
        };
        let (items, cursor, msgs) = read_page(&files, data, false)?;
        Ok(
            json!({"provider":"codex","cid":cid,"backup_dir":bdir,"backup_name":bdir.file_name().unwrap_or_default().to_string_lossy(),"title":self.title(&cid),"readonly":true,"turns":j::turns(items),"page_msgs":msgs,"total_label":format!("загружено: {msgs}"),"cursor":cursor,"has_more":cursor.is_some(),"writable":false}),
        )
    }
    fn restore(&self, data: &Value, ctx: &OpContext) -> Result<Value> {
        let bdir = j::safe_backup(
            &self.paths.backup_root,
            Path::new(j::required(data, "backup_dir")?),
        )?;
        let meta = j::load_json(&bdir.join("backup.meta.json"))?;
        let cid = j::required(&meta, "tid")?;
        j::valid_id(cid)?;
        self.assert_writable(Some(cid))?;
        let snapshot = bdir.join("projection.sqlite");
        let expected = j::s(&meta, "projection_sha256");
        if !expected.is_empty() && (!snapshot.is_file() || j::hash_file(&snapshot)? != expected) {
            bail!("Контрольная сумма копии проекции не совпадает")
        };
        let tail = meta["tail_backup"].is_object();
        let files = self.backup_files(&bdir);
        if !tail && files.is_empty() {
            bail!("Резервная копия не содержит rollout-файлов.")
        }
        let mut targets = Vec::<(PathBuf, PathBuf)>::new();
        let mut temporary = None;
        if tail {
            let tb = &meta["tail_backup"];
            let target = j::safe_join(&self.sessions(), j::required(tb, "rel")?)?;
            let offset = tb["offset"]
                .as_u64()
                .context("Недопустимое смещение хвоста")?;
            let mut tmp = tempfile::NamedTempFile::new_in(target.parent().context("Нет папки")?)?;
            if target.metadata()?.len() < offset {
                bail!("Текущий rollout короче сохранённого смещения")
            };
            let mut source = File::open(&target)?;
            std::io::copy(
                &mut std::io::Read::by_ref(&mut source).take(offset),
                tmp.as_file_mut(),
            )?;
            let bytes = fs::read(bdir.join("tail.bin"))?;
            if let Some(n) = tb["tail_len"].as_u64() {
                if n != bytes.len() as u64 {
                    bail!("Резервная копия хвоста повреждена")
                }
            }
            if let Some(hash) = tb["sha256"].as_str() {
                if j::sha(&bytes) != hash {
                    bail!("Контрольная сумма хвоста не совпадает")
                }
            }
            tmp.write_all(&bytes)?;
            tmp.as_file().sync_all()?;
            targets.push((target, tmp.path().into()));
            temporary = Some(tmp);
        } else {
            if meta["files"]
                .as_u64()
                .is_some_and(|n| n != files.len() as u64)
            {
                bail!("Резервная копия неполная")
            };
            for src in &files {
                if file_meta(src)?.0 != cid {
                    bail!("Резервная копия относится к другому треду")
                };
                let rel = src
                    .strip_prefix(bdir.join("sessions"))
                    .or_else(|_| src.strip_prefix(&bdir))?;
                let rel_s = rel.to_string_lossy();
                let expected = meta["file_hashes"]
                    .get(rel_s.as_ref())
                    .and_then(Value::as_str)
                    .or_else(|| {
                        meta["file_hashes"]
                            .get(rel_s.replace('/', "\\"))
                            .and_then(Value::as_str)
                    })
                    .unwrap_or(if files.len() == 1 {
                        j::s(&meta, "sha256")
                    } else {
                        ""
                    });
                let allow_legacy = self.test_mode()
                    || matches!(j::s(&meta, "tag"), "k1tty_lock" | "k1tty_unlock" | "delete");
                if expected.is_empty() && !allow_legacy {
                    bail!("Старая копия без контрольных сумм требует ручного восстановления.")
                }
                if !expected.is_empty() && j::hash_file(src)? != expected {
                    bail!("Контрольная сумма {} не совпадает", rel.display())
                };
                targets.push((j::safe_join(&self.sessions(), &rel_s)?, src.clone()));
            }
        }
        let con = self.projection_connection()?;
        if let Some(c) = &con {
            if snapshot.is_file() {
                attach_checked(c, &snapshot)?;
            }
            c.execute_batch("BEGIN IMMEDIATE")?;
        }
        let live = self.files(cid)?;
        let undo = if !live.is_empty() {
            ctx.phase("backup");
            Some(self.backup_files_for(cid, "before-restore", &live)?)
        } else {
            None
        };
        let mut guards = Vec::new();
        for (target, _) in &targets {
            if target.is_file() {
                guards.push(Some(j::LockedFile::open(target, true)?));
            } else {
                guards.push(None);
            }
        }
        ctx.phase("restore");
        let result = (|| -> Result<()> {
            for (i, (target, src)) in targets.iter().enumerate() {
                if let Some(g) = &mut guards[i] {
                    g.replace_from(src)?;
                } else {
                    j::atomic_write(target, &fs::read(src)?)?;
                }
            }
            if let Some(c) = &con {
                for table in TABLES {
                    c.execute(
                        &format!("DELETE FROM main.\"{table}\" WHERE thread_id=?"),
                        [cid],
                    )?;
                    if snapshot.is_file() {
                        c.execute(&format!("INSERT INTO main.\"{table}\" SELECT * FROM saved.\"{table}\" WHERE thread_id=?"),[cid])?;
                    }
                }
                c.execute_batch("COMMIT")?;
                wal_checkpoint_truncate(c);
            }
            Ok(())
        })();
        if let Err(error) = result {
            if let Some(c) = &con {
                let _ = c.execute_batch("ROLLBACK");
            }
            if let Some(undo) = &undo {
                for (i, (target, _)) in targets.iter().enumerate() {
                    let saved = undo
                        .join("sessions")
                        .join(target.strip_prefix(self.sessions())?);
                    if saved.is_file() {
                        if let Some(g) = &mut guards[i] {
                            g.replace_from(&saved)?;
                        } else {
                            j::copy_checked(&saved, target)?;
                        }
                    }
                }
            }
            return Err(error);
        }
        drop(guards);
        drop(temporary);
        self.invalidate_paths(None);
        if let Some(saved) = meta["session_index_lines"].as_array() {
            let mut raw = fs::read_to_string(self.index()).unwrap_or_default();
            let mut lines: HashSet<String> = raw.lines().map(str::to_string).collect();
            for line in saved.iter().filter_map(Value::as_str) {
                if lines.insert(line.into()) {
                    if !raw.is_empty() && !raw.ends_with('\n') {
                        raw.push('\n')
                    }
                    raw.push_str(line);
                    raw.push('\n');
                }
            }
            j::atomic_write(&self.index(), raw.as_bytes())?;
        }
        ctx.phase("done");
        Ok(json!({"restored":bdir,"provider":"codex","backup":undo,"warns":[]}))
    }
    #[cfg(test)]
    fn detect_block(&self, cid: &str) -> Result<Option<Value>> {
        let files = self.files(cid)?;
        self.detect_block_files(cid, &files)
    }
    fn detect_block_files(&self, cid: &str, files: &[PathBuf]) -> Result<Option<Value>> {
        if self.projection().is_file() {
            if let Ok(c) =
                Connection::open_with_flags(self.projection(), OpenFlags::SQLITE_OPEN_READ_ONLY)
            {
                let _ = c.busy_timeout(Duration::from_millis(1000));
                let row=c.query_row("SELECT turn_id,status,error_json FROM thread_turns WHERE thread_id=? ORDER BY rollout_ordinal DESC LIMIT 1",[cid],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,Option<String>>(2)?))).optional();
                if let Ok(Some((turn, status, err))) = row {
                    if status == "failed" || err.as_ref().is_some_and(|s| !s.is_empty()) {
                        let value = err
                            .and_then(|s| serde_json::from_str::<Value>(&s).ok())
                            .unwrap_or(json!({}));
                        let mut desc = describe_error(
                            value
                                .get("codexErrorInfo")
                                .or_else(|| value.get("codex_error_info"))
                                .unwrap_or(&json!("failed")),
                            &json!(if j::s(&value, "message").is_empty() {
                                format!("Статус хода: {status}")
                            } else {
                                j::s(&value, "message").into()
                            }),
                        );
                        desc["is_blocked"] = json!(true);
                        desc["turn_id"] = json!(turn);
                        desc["status"] = json!(status);
                        return Ok(Some(desc));
                    }
                }
            }
        }
        let mut answer = None;
        if let Some(p) = files.last() {
            let mut n = 0;
            j::reverse_lines(File::open(p)?, None, |_, raw| {
                n += 1;
                if n > 40 {
                    return Ok(false);
                }
                let Ok(v) = serde_json::from_slice::<Value>(raw) else {
                    return Ok(true);
                };
                let pl = &v["payload"];
                if j::s(pl, "type") == "task_complete" {
                    if !pl["error"].is_null() && pl["error"] != false {
                        let e = &pl["error"];
                        let code = e
                            .get("codex_error_info")
                            .or_else(|| e.get("codexErrorInfo"))
                            .cloned()
                            .unwrap_or(json!("error"));
                        let msg = if e.is_object() {
                            e["message"].clone()
                        } else {
                            e.clone()
                        };
                        let mut d = describe_error(&code, &msg);
                        d["is_blocked"] = json!(true);
                        d["turn_id"] = pl["turn_id"].clone();
                        d["status"] = json!("failed");
                        answer = Some(d);
                    }
                    return Ok(false);
                }
                Ok(true)
            })?;
        }
        Ok(answer)
    }
    fn lock_backup_files(
        &self,
        cid: &str,
        tag: &str,
        files: &[PathBuf],
        path: &Path,
        offset: u64,
    ) -> Result<PathBuf> {
        let total: u64 = files
            .iter()
            .filter_map(|p| p.metadata().ok())
            .map(|m| m.len())
            .sum();
        if total <= 256 * 1024 * 1024 {
            return self.backup_files_for(cid, tag, files);
        }
        let bdir = self
            .paths
            .backup_root
            .join(cid)
            .join(format!("{}-{tag}", j::stamp()));
        fs::create_dir_all(&bdir)?;
        let mut source = File::open(path)?;
        source.seek(SeekFrom::Start(offset))?;
        let mut bytes = Vec::new();
        source.read_to_end(&mut bytes)?;
        j::atomic_write(&bdir.join("tail.bin"), &bytes)?;
        self.backup_projection(cid, &bdir.join("projection.sqlite"))?;
        let p = bdir.join("projection.sqlite");
        let meta = json!({"tid":cid,"created":j::stamp(),"files":files.len(),"tag":tag,"projection_sha256":if p.is_file(){j::hash_file(&p)?}else{String::new()},"tail_backup":{"rel":path.strip_prefix(self.sessions())?.to_string_lossy(),"offset":offset,"orig_size":path.metadata()?.len(),"tail_len":bytes.len(),"sha256":j::sha(&bytes)}});
        j::save_json(&bdir.join("backup.meta.json"), &meta)?;
        Ok(bdir)
    }
    fn lock_operation(&self, data: &Value, ctx: &OpContext, unlock: bool) -> Result<Value> {
        let cid = j::required(data, "cid")?;
        self.assert_writable(Some(cid))?;
        let files = self.files(cid)?;
        let path = files.last().context("Тред не найден")?;
        let mut guard = j::LockedFile::open(path, true)?;
        let orig_size = path.metadata()?.len();
        let mut rows = Vec::new();
        j::reverse_lines(File::open(path)?, None, |off, raw| {
            if rows.len() >= 80 {
                return Ok(false);
            }
            rows.push((off, serde_json::from_slice::<Value>(raw).ok()));
            Ok(true)
        })?;
        let mut patches = BTreeMap::<u64, Value>::new();
        let mut append = None;
        let code = if j::s(data, "code").is_empty() {
            "misalignmentPolicyViolation"
        } else {
            j::s(data, "code")
        };
        let (camel, snake) = code_variants(code);
        let message = j::s(data, "message").trim();
        let mut min_off = orig_size.saturating_sub(4096);
        if unlock {
            for (off, row) in &rows {
                if let Some(v) = row {
                    if j::s(&v["payload"], "type") == "task_complete"
                        && v["payload"].get("error").is_some()
                    {
                        let mut v = v.clone();
                        v["payload"].as_object_mut().unwrap().remove("error");
                        patches.insert(*off, v);
                    }
                }
            }
            if let Some(&o) = patches.keys().next() {
                min_off = o;
            }
        } else {
            if message.is_empty() {
                bail!("Укажите причину блокировки чата.")
            };
            let last_ordinal = rows
                .iter()
                .filter_map(|(_, v)| v.as_ref().and_then(|v| v["ordinal"].as_u64()))
                .next();
            let turn = rows
                .iter()
                .filter_map(|(_, v)| v.as_ref().map(|v| &v["payload"]))
                .find_map(|p| p["turn_id"].as_str().or_else(|| p["root_turn_id"].as_str()))
                .unwrap_or(cid);
            if let Some((off, Some(row))) = rows.iter().find(|(_, v)| {
                v.as_ref().is_some_and(|v| {
                    j::s(v, "type") == "event_msg" && j::s(&v["payload"], "type") == "task_complete"
                })
            }) {
                let mut v = row.clone();
                v["payload"]["error"] = json!({"message":message,"codex_error_info":snake});
                v["payload"]["last_agent_message"] = Value::Null;
                patches.insert(*off, v);
                min_off = *off;
            } else {
                let ts = Utc::now().timestamp();
                let mut v = json!({"timestamp":j::now(),"type":"event_msg","payload":{"type":"task_complete","root_turn_id":turn,"turn_id":turn,"last_agent_message":null,"error":{"message":message,"codex_error_info":snake},"started_at":ts,"completed_at":ts,"duration_ms":0}});
                if let Some(n) = last_ordinal {
                    v["ordinal"] = json!(n + 1)
                }
                append = Some(v);
                min_off = orig_size;
            }
        }
        let tag = if unlock { "k1tty_unlock" } else { "k1tty_lock" };
        let backup = if ctx.auto_backup {
            ctx.phase("backup");
            Some(self.lock_backup_files(cid, tag, &files, path, min_off)?)
        } else {
            None
        };
        let mut tmp = tempfile::NamedTempFile::new_in(path.parent().unwrap())?;
        let mut src = BufReader::new(File::open(path)?);
        let mut off = 0;
        let mut last_newline = true;
        loop {
            let mut raw = Vec::new();
            let n = src.read_until(b'\n', &mut raw)?;
            if n == 0 {
                break;
            }
            last_newline = raw.ends_with(b"\n");
            if let Some(v) = patches.get(&off) {
                let ending = if !unlock && j::ending(&raw).is_empty() {
                    b"\n".as_slice()
                } else {
                    j::ending(&raw)
                };
                let core = serde_json::to_vec(v)?;
                tmp.write_all(&core)?;
                let pad = raw.len().saturating_sub(core.len() + ending.len());
                for _ in 0..pad {
                    tmp.write_all(b" ")?;
                }
                tmp.write_all(ending)?;
            } else {
                tmp.write_all(&raw)?;
            }
            off += n as u64;
        }
        if let Some(v) = &append {
            if orig_size > 0 && !last_newline {
                tmp.write_all(b"\n")?;
            }
            serde_json::to_writer(tmp.as_file_mut(), v)?;
            tmp.write_all(b"\n")?;
        }
        tmp.as_file().sync_all()?;
        let new_size = tmp.as_file().metadata()?.len();
        let delta = new_size as i64 - orig_size as i64;
        let mut undo = tempfile::NamedTempFile::new_in(path.parent().unwrap())?;
        std::io::copy(&mut File::open(path)?, undo.as_file_mut())?;
        undo.as_file().sync_all()?;
        let con = self.projection_connection()?;
        if let Some(c) = &con {
            c.execute_batch("BEGIN IMMEDIATE")?;
        }
        ctx.phase(if unlock { "unlock" } else { "lock" });
        let result = (|| -> Result<usize> {
            guard.replace_from(tmp.path())?;
            let mut changed = 0;
            if let Some(c) = &con {
                if unlock {
                    changed=c.execute("UPDATE thread_turns SET status='completed',error_json=NULL WHERE thread_id=? AND (status='failed' OR error_json IS NOT NULL)",[cid])?;
                    c.execute("DELETE FROM thread_realtime_items WHERE thread_id=?", [cid])?;
                } else {
                    let latest=c.query_row("SELECT rowid,rollout_end_byte_offset FROM thread_turns WHERE thread_id=? ORDER BY rollout_ordinal DESC LIMIT 1",[cid],|r|Ok((r.get::<_,i64>(0)?,r.get::<_,Option<i64>>(1)?))).optional()?;
                    if let Some((row, end)) = latest {
                        let err = serde_json::to_string(
                            &json!({"message":message,"codexErrorInfo":camel,"additionalDetails":null,"misalignment":null}),
                        )?;
                        c.execute("UPDATE thread_turns SET status='failed',error_json=?,rollout_end_byte_offset=? WHERE rowid=?",params![err,end.map(|n|n+delta),row])?;
                        changed = 1;
                    }
                }
                if delta != 0 && files.len() == 1 {
                    c.execute("UPDATE thread_history_projection_state SET next_rollout_byte_offset=? WHERE thread_id=?",params![new_size,cid])?;
                }
                c.execute_batch("COMMIT")?;
                wal_checkpoint_truncate(c);
            }
            Ok(changed)
        })();
        let changed = match result {
            Ok(n) => n,
            Err(e) => {
                if let Some(c) = &con {
                    let _ = c.execute_batch("ROLLBACK");
                }
                guard.replace_from(undo.path())?;
                return Err(e);
            }
        };
        ctx.phase("done");
        let mut result = json!({"tid":cid,"backup":backup,"lines_patched":patches.len()+usize::from(append.is_some()),"warns":[if self.test_mode(){"ТЕСТ-РЕЖИМ: изменения сохраняются в песочницу."}else if unlock{"Чат успешно разблокирован K1tty Tools. Обновите диалог в приложении Codex."}else{"Чат заблокирован. Вы можете разбанить его в любой момент."}]});
        if unlock {
            result["unlocked"] = json!(true);
            result["turns_unlocked"] = json!(changed);
        } else {
            result["locked"] = json!(true);
            result["turns_locked"] = json!(changed);
            result["code"] = json!(camel);
            result["message"] = json!(message);
        }
        Ok(result)
    }
    fn export(&self, data: &Value) -> Result<Value> {
        let cid = j::required(data, "cid")?;
        let files = self.files(cid)?;
        if files.is_empty() {
            bail!("Тред Codex {cid} не найден.")
        };
        let directory = j::read_at(&files[0], 0)?
            .obj()?
            .get("payload")
            .map(|v| j::strip_long_prefix(j::s(v, "cwd")))
            .unwrap_or_default();
        let size = files
            .iter()
            .filter_map(|p| p.metadata().ok())
            .map(|m| m.len())
            .sum();
        let mut entries = Vec::new();
        for (fi, p) in files.iter().enumerate() {
            let mut f = BufReader::new(File::open(p)?);
            let mut offset = 0;
            loop {
                let mut raw = Vec::new();
                let n = f.read_until(b'\n', &mut raw)?;
                if n == 0 {
                    break;
                }
                if codex_raw_skip_visit(&raw) {
                    offset += n as u64;
                    continue;
                }
                if let Ok(v) = serde_json::from_slice::<Value>(&raw) {
                    if let Some(it) = history_item(&v, fi, &format!("b{offset}")) {
                        let k = j::s(&it, "kind");
                        let txt = j::s(&it, "text");
                        let label = j::s(&it, "label");
                        if !(txt.trim().is_empty() && k == "tool" && label == "reasoning") {
                            entries.push(json!({"kind":k,"subtype":if label=="reasoning"{"thinking"}else{it.get("subtype").and_then(Value::as_str).unwrap_or(k)},"label":if label.is_empty(){k}else{label},"text":txt,"ts":it["ts"],"tool":if k=="tool"{json!(label)}else{Value::Null},"status":it["status"],"code":it.get("code").cloned().unwrap_or(json!(""))}));
                        }
                    }
                }
                offset += n as u64;
            }
        }
        Ok(j::normalized_doc(
            "codex",
            cid,
            &self.title(cid),
            &directory,
            size,
            entries,
            data["include_tools"] != false,
        ))
    }
    fn preview_slim(&self, data: &Value) -> Result<Value> {
        let cid = j::required(data, "cid")?;
        let files = self.files(cid)?;
        if files.is_empty() {
            bail!("Тред {cid} не найден");
        }
        let can_slim = files.len() == 1 || self.test_mode();
        let threshold = (data["tool_threshold_kb"]
            .as_u64()
            .unwrap_or(4)
            .clamp(1, 512)
            * 1024) as usize;
        let mut thinking_count = 0usize;
        let mut thinking_bytes = 0usize;
        let mut heavy_tool_count = 0usize;
        let mut heavy_tool_bytes = 0usize;
        let mut raw = Vec::new();
        for p in &files {
            let mut reader = BufReader::with_capacity(256 * 1024, File::open(p)?);
            loop {
                raw.clear();
                let n = reader.read_until(b'\n', &mut raw)?;
                if n == 0 {
                    break;
                }
                let Ok(obj) = serde_json::from_slice::<Value>(&raw) else {
                    continue;
                };
                if j::s(&obj, "type") != "response_item" {
                    continue;
                }
                let pt = j::s(&obj["payload"], "type");
                if pt == "reasoning" {
                    thinking_count += 1;
                    thinking_bytes += raw.len();
                } else if matches!(pt, "function_call_output" | "custom_tool_call_output") {
                    let sz = obj["payload"]["output"].as_str().map(str::len).unwrap_or(0);
                    if sz > threshold {
                        heavy_tool_count += 1;
                        heavy_tool_bytes += sz.saturating_sub(48);
                    }
                }
            }
        }
        let remove_thinking = data["remove_thinking"].as_bool().unwrap_or(true);
        let trim_heavy_tools = data["trim_heavy_tools"].as_bool().unwrap_or(true);
        let freed_bytes = (if remove_thinking { thinking_bytes } else { 0 })
            + (if trim_heavy_tools {
                heavy_tool_bytes
            } else {
                0
            });
        let total_affected = (if remove_thinking { thinking_count } else { 0 })
            + (if trim_heavy_tools {
                heavy_tool_count
            } else {
                0
            });
        Ok(json!({
            "can_slim": can_slim,
            "thinking_count": thinking_count,
            "thinking_bytes": thinking_bytes,
            "thinking_tokens": (thinking_bytes + 3) / 4,
            "heavy_tool_count": heavy_tool_count,
            "heavy_tool_bytes": heavy_tool_bytes,
            "heavy_tool_tokens": (heavy_tool_bytes + 3) / 4,
            "total_affected": total_affected,
            "freed_bytes": freed_bytes,
            "freed_tokens": (freed_bytes + 3) / 4
        }))
    }
    fn apply_slim(&self, data: &Value, ctx: &OpContext) -> Result<Value> {
        let cid = j::required(data, "cid")?;
        self.assert_writable(Some(cid))?;
        let files = self.files(cid)?;
        if files.is_empty() {
            bail!("Тред {cid} не найден");
        }
        if files.len() != 1 && !self.test_mode() {
            bail!("Очистка многосегментного треда вне песочницы недоступна.");
        }
        let remove_thinking = data["remove_thinking"].as_bool().unwrap_or(true);
        let trim_heavy_tools = data["trim_heavy_tools"].as_bool().unwrap_or(true);
        let threshold = (data["tool_threshold_kb"]
            .as_u64()
            .unwrap_or(4)
            .clamp(1, 512)
            * 1024) as usize;
        if !remove_thinking && !trim_heavy_tools {
            bail!("Выберите хотя бы одну опцию очистки.");
        }
        let mut guards: Vec<_> = files
            .iter()
            .map(|p| j::LockedFile::open(p, true))
            .collect::<Result<_>>()?;
        let mut thinking_removed = 0usize;
        let mut tools_trimmed = 0usize;
        let mut original_total = 0usize;
        let mut new_total = 0usize;
        let mut outputs: Vec<Vec<u8>> = Vec::with_capacity(files.len());
        let mut originals: Vec<Vec<u8>> = Vec::with_capacity(files.len());
        for p in &files {
            let lines = j::read_lines(p)?;
            let orig = j::write_lines(&lines);
            original_total += orig.len();
            originals.push(orig);
            let mut kept = Vec::with_capacity(lines.len());
            let mut next_ordinal: Option<u64> = None;
            let mut first_line = true;
            for mut line in lines {
                let Ok(mut obj) = line.obj() else {
                    first_line = false;
                    kept.push(line);
                    continue;
                };
                if first_line {
                    first_line = false;
                    next_ordinal = obj["ordinal"].as_u64().map(|n| n + 1);
                    kept.push(line);
                    continue;
                }
                let mut modified = false;
                if j::s(&obj, "type") == "response_item" {
                    let pt = j::s(&obj["payload"], "type").to_owned();
                    if remove_thinking && pt == "reasoning" {
                        thinking_removed += 1;
                        continue;
                    }
                    if trim_heavy_tools
                        && matches!(
                            pt.as_str(),
                            "function_call_output" | "custom_tool_call_output"
                        )
                    {
                        let sz = obj["payload"]["output"].as_str().map(str::len).unwrap_or(0);
                        if sz > threshold {
                            obj["payload"]["output"] = json!(format!(
                                "[trimmed by K1tty Tools: {:.1} KB]",
                                sz as f64 / 1024.0
                            ));
                            modified = true;
                            tools_trimmed += 1;
                        }
                    }
                }
                if let Some(ord) = next_ordinal {
                    if obj["ordinal"].as_u64() != Some(ord) {
                        obj["ordinal"] = json!(ord);
                        modified = true;
                    }
                    next_ordinal = Some(ord + 1);
                }
                if modified {
                    line.set(&obj)?;
                }
                kept.push(line);
            }
            let out = j::write_lines(&kept);
            new_total += out.len();
            outputs.push(out);
        }
        if thinking_removed == 0 && tools_trimmed == 0 {
            return Ok(json!({
                "provider": "codex",
                "cid": cid,
                "changed": false,
                "thinking_removed": 0,
                "tools_trimmed": 0,
                "freed_bytes": 0,
                "freed_tokens": 0,
                "backup": null,
                "warns": []
            }));
        }
        let backup = if ctx.auto_backup {
            ctx.phase("backup");
            Some(self.backup_files_for(cid, "slim", &files)?)
        } else {
            None
        };
        let con = self.projection_connection()?;
        if let Some(c) = &con {
            c.execute_batch("BEGIN IMMEDIATE")?;
        }
        ctx.phase("cut");
        let res = (|| -> Result<()> {
            for (i, bytes) in outputs.iter().enumerate() {
                guards[i].replace_bytes(bytes)?;
            }
            if let Some(c) = &con {
                for table in TABLES {
                    c.execute(&format!("DELETE FROM \"{table}\" WHERE thread_id=?"), [cid])?;
                }
                c.execute_batch("COMMIT")?;
                wal_checkpoint_truncate(c);
            }
            Ok(())
        })();
        if let Err(error) = res {
            if let Some(c) = &con {
                let _ = c.execute_batch("ROLLBACK");
            }
            for (i, orig) in originals.iter().enumerate() {
                let _ = guards[i].replace_bytes(orig);
            }
            return Err(error);
        }
        drop(guards);
        self.sync_state_db_touch(cid);
        let freed_bytes = original_total.saturating_sub(new_total);
        ctx.phase("done");
        Ok(json!({
            "provider": "codex",
            "cid": cid,
            "changed": true,
            "thinking_removed": thinking_removed,
            "tools_trimmed": tools_trimmed,
            "freed_bytes": freed_bytes,
            "freed_tokens": (freed_bytes + 3) / 4,
            "backup": backup,
            "warns": [if self.test_mode() {
                "ТЕСТ-РЕЖИМ: изменения пишутся в песочницу."
            } else {
                "После изменения заново откройте тред в Codex."
            }]
        }))
    }
}
impl Provider for CodexProvider {
    fn paths(&self) -> &ProviderPaths {
        &self.paths
    }
    fn visit_entries(&self, cid: &str, f: &mut dyn FnMut(EntryRef<'_>) -> bool) -> Result<()> {
        let files = self.files(cid)?;
        let mut raw = Vec::new();
        for (fi, p) in files.iter().enumerate() {
            let mut reader = BufReader::with_capacity(256 * 1024, File::open(p)?);
            let mut offset = 0u64;
            loop {
                raw.clear();
                let n = reader.read_until(b'\n', &mut raw)?;
                if n == 0 {
                    break;
                }
                if codex_raw_skip_visit(&raw) {
                    offset += n as u64;
                    continue;
                }
                if let Ok(v) = serde_json::from_slice::<Value>(&raw) {
                    if let Some(it) = history_item(&v, fi, &format!("b{offset}")) {
                        let k = j::s(&it, "kind");
                        let raw_label = j::s(&it, "label");
                        let subtype = if raw_label == "reasoning" {
                            "thinking"
                        } else {
                            it.get("subtype").and_then(Value::as_str).unwrap_or(k)
                        };
                        let default_label;
                        let label =
                            if matches!(k, "tool" | "error" | "system") && !raw_label.is_empty() {
                                raw_label
                            } else {
                                default_label = j::label(k, "Codex");
                                &default_label
                            };
                        let entry = EntryRef {
                            key: j::s(&it, "key"),
                            kind: k,
                            subtype,
                            label,
                            ts: j::s(&it, "ts"),
                            text: j::s(&it, "text"),
                        };
                        if !f(entry) {
                            return Ok(());
                        }
                    }
                }
                offset += n as u64;
            }
        }
        Ok(())
    }
    fn read_entry_text(&self, cid: &str, key: &str) -> Result<Option<String>> {
        let files = self.files(cid)?;
        let Some((fi, pos)) = key.split_once(':') else {
            return Ok(None);
        };
        let Ok(fi) = fi.parse::<usize>() else {
            return Ok(None);
        };
        let Some(path) = files.get(fi) else {
            return Ok(None);
        };
        let offset = if let Some(byte) = pos.strip_prefix('b') {
            byte.parse::<u64>()?
        } else {
            let count = pos.parse::<usize>()?;
            let mut f = BufReader::new(File::open(path)?);
            let mut off = 0u64;
            let mut raw = Vec::new();
            for _ in 0..count {
                raw.clear();
                let n = f.read_until(b'\n', &mut raw)?;
                if n == 0 {
                    return Ok(None);
                }
                off += n as u64;
            }
            off
        };
        let row = j::read_at(path, offset)?.obj()?;
        if let Some(mut it) = history_item(&row, fi, &format!("b{offset}")) {
            if let Some(Value::String(text)) = it.as_object_mut().and_then(|m| m.remove("text")) {
                return Ok(Some(text));
            }
        }
        Ok(None)
    }
    fn probe_lock(&self, cid: Option<&str>) -> Result<Option<String>> {
        if let Some(cid) = cid {
            let files = self.files(cid)?;
            return self.probe_lock_files(Some(&files));
        }
        self.probe_lock_files(None)
    }
    fn content_signature(&self, cid: &str) -> Option<String> {
        let files = self.files(cid).ok()?;
        if files.is_empty() {
            return None;
        }
        if files.len() == 1 {
            return j::content_signature(&files[0]);
        }
        let sigs: Vec<String> = files
            .iter()
            .filter_map(|p| j::content_signature(p))
            .collect();
        (!sigs.is_empty()).then(|| sigs.join("|"))
    }
    fn call(&self, op: &str, data: &Value, ctx: &OpContext) -> Result<Value> {
        match op {
            "state" => {
                let running = self.running();
                let avail = self.sessions().is_dir();
                let note = if !avail {
                    format!("Каталог сессий не найден: {}", self.sessions().display())
                } else if self.test_mode() {
                    "ТЕСТ-РЕЖИМ: изменения пишутся в песочницу.".into()
                } else if !running.is_empty() {
                    format!("Запущено: {}. Закройте тред в Codex перед правкой и откройте заново после.",running.join(", "))
                } else {
                    "Готов к потоковой правке и обрезке однофайловых тредов с проверкой контрольных сумм.".into()
                };
                Ok(
                    json!({"id":"codex","name":"Codex","short":"CX","available":avail,"writable":avail,"blocked":false,"running":running,"test":self.test_mode(),"appdata":self.paths.root,"backups_root":self.paths.backup_root,"note":note}),
                )
            }
            "convs" => {
                if data["refresh"].as_bool().unwrap_or(false) {
                    self.invalidate_paths(None);
                }
                Ok(json!(self.list()?))
            }
            "entries" => self.entries(data),
            "edit_preview" => self.edit_preview(data),
            "preview_cut" => self.preview_cut(data),
            "preview_slim" => self.preview_slim(data),
            "apply_slim" => self.apply_slim(data, ctx),
            "apply_edit" | "apply_cut" => self.mutate(op, data, ctx),
            "delete_single" => {
                bail!("Удаление одиночной записи не поддерживается этим провайдером")
            }
            "delete_conv" => self.delete_conv(data, ctx),
            "backups" => self.backups(data),
            "backup_entries" => self.backup_entries(data),
            "restore" => self.restore(data, ctx),
            "unlock_chat" => self.lock_operation(data, ctx, true),
            "lock_chat" => self.lock_operation(data, ctx, false),
            "export_chat" => self.export(data),
            _ => bail!("Неизвестная операция Codex: {op}"),
        }
    }
}
fn file_meta(path: &Path) -> Result<(String, String)> {
    let mut first = Vec::new();
    BufReader::new(File::open(path)?).read_until(b'\n', &mut first)?;
    let v: Value = serde_json::from_slice(&first)?;
    let pl = &v["payload"];
    let source = &pl["source"];
    if source == "subagent" || source.get("subagent").is_some() {
        return Ok((String::new(), j::s(&v, "timestamp").into()));
    }
    let id = pl["session_id"]
        .as_str()
        .filter(|s| !s.is_empty())
        .or_else(|| pl["parent_thread_id"].as_str().filter(|s| !s.is_empty()))
        .or_else(|| pl["id"].as_str().filter(|s| !s.is_empty()))
        .unwrap_or("");
    Ok((id.into(), j::s(&v, "timestamp").into()))
}

fn join_text(content: &Value) -> String {
    content
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|v| v["text"].as_str())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n")
}
fn output_text(v: &Value) -> String {
    if v.is_array() {
        let joined = join_text(v);
        if !joined.is_empty() {
            return joined;
        }
    }
    j::text(v)
}

fn auto_injected(text: &str) -> bool {
    let t = text.trim_start();
    ["<environment_context", "<app-context", "<user_instructions"]
        .iter()
        .any(|p| t.starts_with(p))
}
pub fn history_item(v: &Value, fi: usize, position: &str) -> Option<Value> {
    let typ = j::s(v, "type");
    let pl = &v["payload"];
    let pt = j::s(pl, "type");
    let out = json!({"key":format!("{fi}:{position}"),"ts":v.get("timestamp").cloned().unwrap_or(json!("")),"fi":fi,"line":position});
    let extra = if typ == "event_msg" {
        match pt {
            "task_complete" if !pl["error"].is_null() && pl["error"] != false => {
                let e = &pl["error"];
                let message = if e.is_object() {
                    e["message"].clone()
                } else {
                    e.clone()
                };
                let code = e
                    .get("codex_error_info")
                    .or_else(|| e.get("codexErrorInfo"))
                    .cloned()
                    .unwrap_or(json!("error"));
                let meta = describe_error(&code, &message);
                json!({"kind":"error","subtype":"codex_error","label":meta["category"],"code":meta["code"],"text":meta["message"],"turn_id":pl["turn_id"]})
            }
            "turn_aborted" => {
                let reason = pl
                    .get("reason")
                    .and_then(Value::as_str)
                    .unwrap_or("interrupted");
                json!({"kind":"system","subtype":"aborted","label":"Ход прерван","text":if reason=="interrupted"{"Генерация прервана пользователем"}else{reason},"turn_id":pl["turn_id"]})
            }
            _ => return None,
        }
    } else if typ == "response_item" {
        match pt {
            "message" => {
                let role = j::s(pl, "role");
                let txt = join_text(&pl["content"]);
                let kind = match role {
                    "user" if !auto_injected(&txt) => "user",
                    "assistant" => "assistant",
                    _ => "system",
                };
                json!({"kind":kind,"text":txt,"id":pl["id"],"role":role})
            }
            "reasoning" => {
                json!({"kind":"tool","label":"reasoning","text":join_text(&pl["summary"])})
            }
            "custom_tool_call" | "function_call" => {
                json!({"kind":"tool","label":if j::s(pl,"name").is_empty(){pt}else{j::s(pl,"name")},"text":j::text(pl.get("input").unwrap_or(&pl["arguments"]))})
            }
            "custom_tool_call_output" | "function_call_output" => {
                json!({"kind":"tool","label":"вывод","text":output_text(&pl["output"]),"status":pl["status"]})
            }
            _ => return None,
        }
    } else {
        return None;
    };
    if let (Value::Object(mut out_map), Value::Object(extra_map)) = (out, extra) {
        out_map.extend(extra_map);
        Some(Value::Object(out_map))
    } else {
        None
    }
}
fn entry(it: Value, writable: bool) -> Value {
    let mut map = match it {
        Value::Object(m) => m,
        _ => return Value::Null,
    };
    let k = map
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_owned();
    let raw_subtype = map.remove("subtype");
    let raw_label = map.remove("label");
    let is_aborted = raw_subtype.as_ref().and_then(Value::as_str) == Some("aborted");
    let mut actions = Vec::new();
    if writable {
        if matches!(k.as_str(), "user" | "assistant" | "system" | "tool") && !is_aborted {
            actions.push("edit");
        }
        if k == "user" {
            actions.push("cut_head");
        }
        actions.push("cut_tail");
    }
    let raw_label_str = raw_label.as_ref().and_then(Value::as_str).unwrap_or("");
    let label = if matches!(k.as_str(), "tool" | "error" | "system") && !raw_label_str.is_empty() {
        raw_label_str.to_owned()
    } else {
        j::label(&k, "Codex")
    };
    let subtype = if raw_label_str == "reasoning" {
        Value::String("thinking".into())
    } else if let Some(s) = raw_subtype.as_ref().and_then(Value::as_str) {
        Value::String(s.to_owned())
    } else {
        Value::String(k.clone())
    };
    let mut out = serde_json::Map::with_capacity(11);
    out.insert("key".into(), map.remove("key").unwrap_or(Value::Null));
    out.insert("kind".into(), Value::String(k));
    out.insert("subtype".into(), subtype);
    out.insert("label".into(), Value::String(label));
    out.insert(
        "text".into(),
        map.remove("text").unwrap_or_else(|| json!("")),
    );
    out.insert("ts".into(), map.remove("ts").unwrap_or(Value::Null));
    out.insert("tool".into(), raw_label.unwrap_or(Value::Null));
    out.insert("code".into(), map.remove("code").unwrap_or(Value::Null));
    out.insert("status".into(), map.remove("status").unwrap_or(Value::Null));
    out.insert("in_db".into(), Value::Bool(true));
    out.insert("actions".into(), json!(actions));
    Value::Object(out)
}
#[derive(serde::Deserialize)]
struct CodexPayloadProbe<'a> {
    #[serde(rename = "type", borrow)]
    typ: Option<Cow<'a, str>>,
    #[serde(borrow)]
    role: Option<Cow<'a, str>>,
}
#[derive(serde::Deserialize)]
struct CodexKindProbe<'a> {
    #[serde(rename = "type", borrow)]
    typ: Option<Cow<'a, str>>,
    payload: Option<CodexPayloadProbe<'a>>,
}
fn codex_raw_may_be_dialogue(raw: &[u8]) -> bool {
    let Ok(probe) = serde_json::from_slice::<CodexKindProbe>(raw) else {
        return false;
    };
    let Some(pl) = probe.payload else {
        return false;
    };
    match (probe.typ.as_deref(), pl.typ.as_deref()) {
        (Some("response_item"), Some("message")) => {
            matches!(pl.role.as_deref(), Some("user" | "assistant"))
        }
        (Some("event_msg"), Some("task_complete")) => true,
        _ => false,
    }
}
fn read_page(
    files: &[PathBuf],
    data: &Value,
    writable: bool,
) -> Result<(Vec<Value>, Option<String>, usize)> {
    if files.is_empty() {
        return Ok((Vec::new(), None, 0));
    }
    let limit = data["limit"].as_u64().unwrap_or(80).clamp(1, 150) as usize;
    let include = data["include_tools"] == true;
    let mut last = files.len() - 1;
    let mut end = None;
    if let Some(before) = data["before"].as_str().filter(|s| !s.is_empty()) {
        let (fi, loc) = before
            .split_once(':')
            .context("Некорректный курсор истории")?;
        last = fi.parse()?;
        end = Some(
            loc.strip_prefix('b')
                .context("Некорректный курсор истории")?
                .parse::<u64>()?,
        );
        if last >= files.len() {
            bail!("Курсор больше не существует")
        }
    }
    let max_items = if include { 350 } else { limit * 2 };
    let (mut items, mut msgs, mut cursor) = (Vec::new(), 0, None);
    let mut page_bytes = 0usize;
    for fi in (0..=last).rev() {
        let mut stop = false;
        j::reverse_lines(
            File::open(&files[fi])?,
            if fi == last { end } else { None },
            |off, raw| {
                if !include && !codex_raw_may_be_dialogue(raw) {
                    return Ok(true);
                }
                let Ok(v) = serde_json::from_slice::<Value>(raw) else {
                    return Ok(true);
                };
                let Some(it) = history_item(&v, fi, &format!("b{off}")) else {
                    return Ok(true);
                };
                let kind = j::s(&it, "kind");
                if !include && !matches!(kind, "user" | "assistant" | "error") {
                    return Ok(true);
                }
                if matches!(kind, "user" | "assistant") {
                    msgs += 1;
                }
                let mut built = entry(it, writable);
                truncate_preview_item(&mut built);
                page_bytes += built["text"].as_str().map(str::len).unwrap_or(0);
                let item_key = built["key"].as_str().unwrap_or("").to_owned();
                items.push(built);
                if msgs >= limit
                    || items.len() >= max_items
                    || (!items.is_empty() && page_bytes >= PAGE_BYTE_BUDGET)
                {
                    cursor = (fi > 0 || off > 0).then_some(item_key);
                    stop = true;
                }
                Ok(!stop)
            },
        )?;
        if stop {
            break;
        }
    }
    items.reverse();
    Ok((items, cursor, msgs))
}
fn read_page_around(
    files: &[PathBuf],
    data: &Value,
    writable: bool,
    around_key: &str,
) -> Result<(Vec<Value>, Option<String>, usize, bool)> {
    if files.is_empty() {
        return Ok((Vec::new(), None, 0, false));
    }
    let limit = data["limit"].as_u64().unwrap_or(80).clamp(1, 150) as usize;
    let include = data["include_tools"] == true;
    let (target_fi, target_off) = byte_key(files, around_key)?;

    let after_limit = (limit / 2).max(15);
    let after_max_items = if include { 120 } else { limit };
    let half_budget = PAGE_BYTE_BUDGET / 2;
    let mut after_items = Vec::new();
    let mut after_msgs = 0usize;
    let mut after_bytes = 0usize;
    let mut has_newer = false;
    let mut raw = Vec::new();

    for fi in target_fi..files.len() {
        let mut f = File::open(&files[fi])?;
        let mut offset = if fi == target_fi {
            f.seek(SeekFrom::Start(target_off))?;
            target_off
        } else {
            0
        };
        let mut reader = BufReader::with_capacity(64 * 1024, f);
        loop {
            raw.clear();
            let n = reader.read_until(b'\n', &mut raw)?;
            if n == 0 {
                break;
            }
            let cur_off = offset;
            offset += n as u64;
            let is_target = fi == target_fi && cur_off == target_off;
            if !is_target && !include && !codex_raw_may_be_dialogue(&raw) {
                continue;
            }
            let Ok(v) = serde_json::from_slice::<Value>(&raw) else {
                continue;
            };
            let Some(it) = history_item(&v, fi, &format!("b{cur_off}")) else {
                continue;
            };
            let kind = j::s(&it, "kind");
            if !is_target && !include && !matches!(kind, "user" | "assistant" | "error") {
                continue;
            }
            if !after_items.is_empty()
                && (after_msgs >= after_limit
                    || after_items.len() >= after_max_items
                    || after_bytes >= half_budget)
            {
                has_newer = true;
                break;
            }
            if matches!(kind, "user" | "assistant") {
                after_msgs += 1;
            }
            let mut built = entry(it, writable);
            truncate_preview_item(&mut built);
            after_bytes += built["text"].as_str().map(str::len).unwrap_or(0);
            after_items.push(built);
        }
        if has_newer {
            break;
        }
    }

    let before_limit = limit.saturating_sub(after_msgs).max(15);
    let before_max_items = if include { 120 } else { limit };
    let mut before_items = Vec::new();
    let mut before_msgs = 0usize;
    let mut before_bytes = 0usize;
    let mut cursor = None;

    for fi in (0..=target_fi).rev() {
        let mut stop = false;
        j::reverse_lines(
            File::open(&files[fi])?,
            if fi == target_fi {
                Some(target_off)
            } else {
                None
            },
            |off, line_raw| {
                if !include && !codex_raw_may_be_dialogue(line_raw) {
                    return Ok(true);
                }
                let Ok(v) = serde_json::from_slice::<Value>(line_raw) else {
                    return Ok(true);
                };
                let Some(it) = history_item(&v, fi, &format!("b{off}")) else {
                    return Ok(true);
                };
                let kind = j::s(&it, "kind");
                if !include && !matches!(kind, "user" | "assistant" | "error") {
                    return Ok(true);
                }
                if matches!(kind, "user" | "assistant") {
                    before_msgs += 1;
                }
                let mut built = entry(it, writable);
                truncate_preview_item(&mut built);
                before_bytes += built["text"].as_str().map(str::len).unwrap_or(0);
                let item_key = built["key"].as_str().unwrap_or("").to_owned();
                before_items.push(built);
                if before_msgs >= before_limit
                    || before_items.len() >= before_max_items
                    || (!before_items.is_empty() && before_bytes >= half_budget)
                {
                    cursor = (fi > 0 || off > 0).then_some(item_key);
                    stop = true;
                }
                Ok(!stop)
            },
        )?;
        if stop {
            break;
        }
    }

    before_items.reverse();
    before_items.extend(after_items);
    Ok((before_items, cursor, before_msgs + after_msgs, has_newer))
}
fn byte_key(files: &[PathBuf], key: &str) -> Result<(usize, u64)> {
    let (fi, pos) = key.split_once(':').context("Недопустимый ключ сообщения")?;
    let fi = fi.parse::<usize>()?;
    let p = files
        .get(fi)
        .context("Ключ сообщения больше не существует")?;
    let offset = if let Some(byte) = pos.strip_prefix('b') {
        byte.parse::<u64>()?
    } else {
        let count = pos.parse::<usize>()?;
        let mut f = BufReader::new(File::open(p)?);
        let mut off = 0;
        for _ in 0..count {
            let mut raw = Vec::new();
            let n = f.read_until(b'\n', &mut raw)?;
            if n == 0 {
                bail!("Ключ сообщения больше не существует")
            }
            off += n as u64;
        }
        off
    };
    let line = j::read_at(p, offset)?;
    if history_item(&line.obj()?, fi, &format!("b{offset}")).is_none() {
        bail!("Ключ сообщения больше не существует")
    }
    Ok((fi, offset))
}
fn extend_events(path: &Path, offset: u64, kind: &str) -> Result<u64> {
    let mut f = File::open(path)?;
    f.seek(SeekFrom::Start(offset))?;
    let mut f = BufReader::new(f);
    let mut off = offset;
    let want = match kind {
        "user" => "UserMessage",
        "assistant" => "AgentMessage",
        _ => return Ok(offset),
    };
    loop {
        let mut raw = Vec::new();
        let n = f.read_until(b'\n', &mut raw)?;
        if n == 0 {
            return Ok(off);
        }
        let Ok(v) = serde_json::from_slice::<Value>(&raw) else {
            return Ok(off);
        };
        if j::s(&v["payload"], "type") != "item_completed"
            || j::s(&v["payload"]["item"], "type") != want
        {
            return Ok(off);
        }
        off += n as u64;
    }
}
type CutRange = (usize, u64, u64);
fn cut_ranges(files: &[PathBuf], key: &str, mode: &str) -> Result<Vec<CutRange>> {
    if !matches!(mode, "head" | "tail") {
        bail!("Недопустимый режим обрезки")
    };
    let (fi, off) = byte_key(files, key)?;
    let mut ranges = Vec::new();
    if mode == "tail" {
        let mut previous = None;
        for j in (0..=fi).rev() {
            j::reverse_lines(
                File::open(&files[j])?,
                if j == fi { Some(off) } else { None },
                |pos, raw| {
                    if let Ok(v) = serde_json::from_slice::<Value>(raw) {
                        if let Some(it) = history_item(&v, j, &format!("b{pos}")) {
                            let len = j::read_at(&files[j], pos)?.raw.len() as u64;
                            let end = extend_events(&files[j], pos + len, j::s(&it, "kind"))?;
                            previous = Some((j, end));
                            return Ok(false);
                        }
                    }
                    Ok(true)
                },
            )?;
            if previous.is_some() {
                break;
            }
        }
        for (j, p) in files.iter().enumerate() {
            let size = p.metadata()?.len();
            match previous {
                Some((keep, end)) if j == keep && end < size => ranges.push((j, end, size)),
                Some((keep, _)) if j > keep => ranges.push((j, 0, size)),
                None => ranges.push((j, 0, size)),
                _ => {}
            }
        }
    } else {
        for (j, p) in files.iter().enumerate() {
            let size = p.metadata()?.len();
            if j < fi {
                ranges.push((j, 0, size));
            } else if j == fi {
                let meta = j::read_at(p, 0)?.raw.len() as u64;
                if off < meta {
                    bail!("Нельзя удалить метаданные сессии")
                };
                if off > meta {
                    ranges.push((j, meta, off));
                }
            }
        }
    }
    Ok(ranges)
}
fn cut_report(cid: &str, mode: &str, files: &[PathBuf], ranges: &[CutRange]) -> Result<Value> {
    let bytes: u64 = ranges.iter().map(|r| r.2 - r.1).sum();
    let exact = bytes <= 16 * 1024 * 1024;
    let (mut lines, mut messages, mut previews) = (0, 0, Vec::new());
    if exact {
        for &(fi, start, end) in ranges {
            let mut f = File::open(&files[fi])?;
            f.seek(SeekFrom::Start(start))?;
            let mut f = BufReader::new(f.take(end - start));
            let mut off = start;
            loop {
                let mut raw = Vec::new();
                let n = f.read_until(b'\n', &mut raw)?;
                if n == 0 {
                    break;
                }
                lines += 1;
                if let Ok(v) = serde_json::from_slice::<Value>(&raw) {
                    if let Some(it) = history_item(&v, fi, &format!("b{off}")) {
                        if matches!(j::s(&it, "kind"), "user" | "assistant") {
                            messages += 1;
                            previews.push(json!({"key":it["key"],"label":if j::s(&it,"kind")=="user"{"Вы"}else{"Ассистент"},"text":j::truncate(j::s(&it,"text"),300)}));
                            if previews.len() > 30 {
                                previews.remove(0);
                            }
                        }
                    }
                }
                off += n as u64;
            }
        }
    }
    let whole = ranges
        .iter()
        .filter(|(i, s, e)| *s == 0 && files[*i].metadata().is_ok_and(|m| m.len() == *e))
        .count();
    Ok(
        json!({"thread":cid,"mode":mode,"files_total":files.len(),"rollout_lines":if exact{Some(lines)}else{None},"rollout_files":whole,"bytes":bytes,"count_incomplete":!exact,"will_delete":{"steps_preview":previews,"steps_preview_count":if exact{Some(messages)}else{None}}}),
    )
}
fn validate_ordinals(path: &Path) -> Result<()> {
    let mut reader = BufReader::new(File::open(path)?);
    let mut first = Vec::new();
    if reader.read_until(b'\n', &mut first)? == 0 {
        bail!("Пустой rollout")
    };
    let meta: Value = serde_json::from_slice(&first)?;
    if let Some(start) = meta["ordinal"].as_u64() {
        if start != 0
            && meta["payload"]["history_base"].is_null()
            && meta["payload"]["forked_from_id"].is_null()
        {
            bail!("Недопустимая нумерация rollout")
        };
        let mut ordinal = start + 1;
        loop {
            let mut raw = Vec::new();
            if reader.read_until(b'\n', &mut raw)? == 0 {
                break;
            }
            let row: Value = serde_json::from_slice(&raw)?;
            if row["ordinal"].as_u64() != Some(ordinal) {
                bail!("Нарушена нумерация rollout около записи {ordinal}; изменение отменено")
            };
            ordinal += 1;
        }
    }
    Ok(())
}
fn wal_checkpoint_truncate(c: &Connection) {
    let _ = c.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);");
}
fn strip_tid_refs(v: &mut Value, tid: &str) -> bool {
    match v {
        Value::Array(arr) => {
            let before = arr.len();
            arr.retain(|item| {
                if item.as_str() == Some(tid) {
                    return false;
                }
                if let Some(obj) = item.as_object() {
                    if obj.get("id").and_then(Value::as_str) == Some(tid)
                        || obj.get("threadId").and_then(Value::as_str) == Some(tid)
                        || obj.get("thread_id").and_then(Value::as_str) == Some(tid)
                    {
                        return false;
                    }
                }
                true
            });
            let mut changed = arr.len() != before;
            for item in arr.iter_mut() {
                if strip_tid_refs(item, tid) {
                    changed = true;
                }
            }
            changed
        }
        Value::Object(map) => {
            let before = map.len();
            map.retain(|k, val| k != tid && val.as_str() != Some(tid));
            let mut changed = map.len() != before;
            for val in map.values_mut() {
                if strip_tid_refs(val, tid) {
                    changed = true;
                }
            }
            changed
        }
        _ => false,
    }
}
fn contains_bytes(hay: &[u8], needle: &[u8]) -> bool {
    if needle.is_empty() || needle.len() > hay.len() {
        return false;
    }
    let first = needle[0];
    let rest = &needle[1..];
    let limit = hay.len() - rest.len();
    let mut i = 0usize;
    while i < limit {
        if let Some(pos) = hay[i..limit].iter().position(|&b| b == first) {
            let start = i + pos;
            if &hay[start + 1..start + needle.len()] == rest {
                return true;
            }
            i = start + 1;
        } else {
            break;
        }
    }
    false
}
fn codex_raw_skip_visit(raw: &[u8]) -> bool {
    (contains_bytes(raw, b"token_count")
        || contains_bytes(raw, b"turn_context")
        || contains_bytes(raw, b"session_meta"))
        && !contains_bytes(raw, b"response_item")
        && !contains_bytes(raw, b"task_complete")
        && !contains_bytes(raw, b"turn_aborted")
}
fn cut_candidate(path: &Path, dst: &mut File, ranges: &[CutRange], renumber: bool) -> Result<()> {
    let mut f = BufReader::with_capacity(256 * 1024, File::open(path)?);
    let mut writer = BufWriter::with_capacity(256 * 1024, dst);
    let mut offset = 0;
    let mut new_ordinal = None;
    loop {
        let mut raw = Vec::new();
        let n = f.read_until(b'\n', &mut raw)?;
        if n == 0 {
            break;
        }
        let skip = ranges
            .iter()
            .any(|(_, start, end)| offset >= *start && offset < *end);
        if !skip {
            if offset == 0 && renumber {
                let v: Value = serde_json::from_slice(&raw)?;
                new_ordinal = v["ordinal"].as_u64().map(|n| n + 1);
            } else if let Some(ord) = new_ordinal {
                let mut v: Value = serde_json::from_slice(&raw)?;
                v["ordinal"] = json!(ord);
                new_ordinal = Some(ord + 1);
                let ending = j::ending(&raw).to_vec();
                raw = serde_json::to_vec(&v)?;
                raw.extend(ending);
            }
            writer.write_all(&raw)?;
        }
        offset += n as u64;
    }
    writer.flush()?;
    Ok(())
}
fn edit_info(v: &Value) -> Result<Value> {
    if j::s(v, "type") != "response_item" {
        bail!("Ключ не указывает на редактируемую запись истории")
    };
    let pl = &v["payload"];
    let pt = j::s(pl, "type");
    let info = match pt {
        "message" => {
            let role = j::s(pl, "role");
            let text = join_text(&pl["content"]);
            let (kind, label, mirror) = match role {
                "user" if auto_injected(&text) => ("environment", "Контекст окружения", false),
                "user" => ("user", "Сообщение пользователя", true),
                "assistant" => ("assistant", "Ответ Codex", true),
                "developer" => ("developer", "Системные инструкции", false),
                _ => ("system", "Системное сообщение", false),
            };
            json!({"old":text,"kind":kind,"label":label,"role":role,"pt":pt,"mirror_events":mirror})
        }
        "reasoning" => {
            json!({"old":join_text(&pl["summary"]),"kind":"reasoning","label":"Размышления","role":null,"pt":pt,"mirror_events":false})
        }
        "custom_tool_call" | "function_call" => {
            json!({"old":j::text(pl.get("input").unwrap_or(&pl["arguments"])),"kind":"tool_call","label":format!("Вызов инструмента: {}",if j::s(pl,"name").is_empty(){pt}else{j::s(pl,"name")}),"role":null,"pt":pt,"mirror_events":false})
        }
        "custom_tool_call_output" | "function_call_output" => {
            json!({"old":output_text(&pl["output"]),"kind":"tool_output","label":"Вывод инструмента","role":null,"pt":pt,"mirror_events":false})
        }
        _ => bail!("Запись типа {pt} не поддерживает редактирование текста"),
    };
    Ok(info)
}
fn patch_item(pl: &mut Value, text: &str) -> Result<()> {
    match j::s(pl, "type") {
        "message" => {
            let mut content = Vec::new();
            let mut did = false;
            for mut c in pl["content"].as_array().cloned().unwrap_or_default() {
                if matches!(j::s(&c, "type"), "text" | "input_text" | "output_text")
                    && c.get("text").is_some()
                {
                    if !did {
                        c["text"] = json!(text);
                        content.push(c);
                        did = true;
                    }
                } else {
                    content.push(c)
                }
            }
            if !did {
                content.push(json!({"type":if j::s(pl,"role")=="assistant"{"output_text"}else{"input_text"},"text":text}))
            };
            pl["content"] = json!(content);
        }
        "reasoning" => {
            let a = pl["summary"].as_array().cloned().unwrap_or_default();
            let kind = a
                .iter()
                .find_map(|v| v["type"].as_str())
                .unwrap_or("summary_text");
            let mut out = vec![json!({"type":kind,"text":text})];
            out.extend(a.into_iter().filter(|v| v.get("text").is_none()));
            pl["summary"] = json!(out);
        }
        "custom_tool_call" | "function_call" => {
            let field = if pl.get("input").is_some() {
                "input"
            } else {
                "arguments"
            };
            pl[field] = if pl[field].is_object() || pl[field].is_array() {
                serde_json::from_str(text).unwrap_or(json!(text))
            } else {
                json!(text)
            };
        }
        "custom_tool_call_output" | "function_call_output" => {
            pl["output"] = if pl["output"].is_object() || pl["output"].is_array() {
                serde_json::from_str(text).unwrap_or(json!(text))
            } else {
                json!(text)
            };
        }
        _ => bail!("Запись не поддерживает редактирование"),
    };
    Ok(())
}
fn deep_replace(v: &mut Value, old: &str, new: &str) -> bool {
    match v {
        Value::String(s) if s == old => {
            *s = new.into();
            true
        }
        Value::Array(a) => a
            .iter_mut()
            .fold(false, |hit, v| deep_replace(v, old, new) || hit),
        Value::Object(o) => o
            .values_mut()
            .fold(false, |hit, v| deep_replace(v, old, new) || hit),
        _ => false,
    }
}
fn edit_candidate(path: &Path, dst: &mut File, offset: u64, text: &str) -> Result<Value> {
    let mut target = j::read_at(path, offset)?.obj()?;
    let info = edit_info(&target)?;
    let old = j::s(&info, "old");
    let mirror = info["mirror_events"] == true && !old.trim().is_empty();
    let mut duplicates = 0;
    if mirror {
        let mut f = BufReader::with_capacity(256 * 1024, File::open(path)?);
        let mut raw = Vec::new();
        loop {
            raw.clear();
            let n = f.read_until(b'\n', &mut raw)?;
            if n == 0 {
                break;
            }
            if !contains_bytes(&raw, b"response_item") || !contains_bytes(&raw, b"message") {
                continue;
            }
            if let Ok(v) = serde_json::from_slice::<Value>(&raw) {
                if j::s(&v, "type") == "response_item"
                    && j::s(&v["payload"], "type") == "message"
                    && join_text(&v["payload"]["content"]) == old
                {
                    duplicates += 1;
                }
            }
        }
    }
    let role = j::s(&target["payload"], "role").to_string();
    let id = target["payload"]["id"].clone();
    patch_item(&mut target["payload"], text)?;
    let mut f = BufReader::with_capacity(256 * 1024, File::open(path)?);
    let mut writer = BufWriter::with_capacity(256 * 1024, dst);
    let (mut pos, mut changed) = (0, Vec::new());
    let mut raw = Vec::new();
    loop {
        raw.clear();
        let n = f.read_until(b'\n', &mut raw)?;
        if n == 0 {
            break;
        }
        let mut row = None;
        if pos == offset {
            row = Some(target.clone())
        } else if mirror {
            let may_mirror = contains_bytes(&raw, b"event_msg")
                && if role == "assistant" {
                    contains_bytes(&raw, b"AgentMessage")
                        || contains_bytes(&raw, b"task_complete")
                        || contains_bytes(&raw, b"agent_message")
                } else {
                    contains_bytes(&raw, b"UserMessage") || contains_bytes(&raw, b"user_message")
                };
            if may_mirror {
                if let Ok(mut v) = serde_json::from_slice::<Value>(&raw) {
                    if j::s(&v, "type") == "event_msg" {
                        let pl = &v["payload"];
                        let item = &pl["item"];
                        let item_id = item["id"].clone();
                        let relevant = j::s(item, "type")
                            == if role == "user" {
                                "UserMessage"
                            } else {
                                "AgentMessage"
                            }
                            || if role == "assistant" {
                                matches!(j::s(pl, "type"), "task_complete" | "agent_message")
                            } else {
                                j::s(pl, "type") == "user_message"
                            };
                        if relevant && deep_replace(&mut v, old, text) {
                            if duplicates > 1 && (item_id.is_null() || item_id != id) {
                                bail!("Несколько сообщений имеют одинаковый текст; безопасно определить копию не удалось")
                            };
                            row = Some(v);
                        }
                    }
                }
            }
        }
        if let Some(v) = row {
            let ending = j::ending(&raw).to_vec();
            raw = serde_json::to_vec(&v)?;
            raw.extend(ending);
            changed.push(pos);
        }
        writer.write_all(&raw)?;
        pos += n as u64;
    }
    writer.flush()?;
    Ok(json!({"old":old,"new":text,"lines_changed":changed}))
}
fn attach_checked(c: &Connection, path: &Path) -> Result<()> {
    c.execute(
        "ATTACH DATABASE ? AS saved",
        [path.to_string_lossy().as_ref()],
    )?;
    let check: String = c.query_row("PRAGMA saved.quick_check", [], |r| r.get(0))?;
    if check != "ok" {
        bail!("Повреждена копия проекции")
    };
    for table in TABLES {
        let cols = |schema: &str| -> Result<Vec<String>> {
            let mut q = c.prepare(&format!("PRAGMA {schema}.table_info(\"{table}\")"))?;
            let rows = q
                .query_map([], |r| r.get::<_, String>(1))?
                .collect::<std::result::Result<Vec<_>, _>>()?;
            Ok(rows)
        };
        let live = cols("main")?;
        let saved = cols("saved")?;
        if live.is_empty() || live != saved || !live.iter().any(|c| c == "thread_id") {
            bail!("Не совпадает схема таблицы {table}")
        }
    }
    Ok(())
}
fn block_presets_ref() -> &'static Value {
    static PRESETS: OnceLock<Value> = OnceLock::new();
    PRESETS.get_or_init(|| {
        serde_json::from_str(include_str!("codex_block_presets.json"))
            .expect("embedded block presets")
    })
}
pub fn block_presets() -> Value {
    block_presets_ref().clone()
}
fn normalized_code(v: &Value) -> String {
    if let Some(o) = v.as_object() {
        o.keys().next().cloned().unwrap_or_else(|| "other".into())
    } else {
        let s = j::text(v);
        if s.trim().is_empty() {
            "failed".into()
        } else {
            s.trim().into()
        }
    }
}
fn code_variants(raw: &str) -> (String, String) {
    for p in block_presets_ref().as_array().unwrap() {
        if [j::s(p, "code"), j::s(p, "rollout_code"), j::s(p, "id")]
            .iter()
            .any(|s| s.eq_ignore_ascii_case(raw))
        {
            return (j::s(p, "code").into(), j::s(p, "rollout_code").into());
        }
    }
    if raw.contains('_') {
        let mut parts = raw.split('_').filter(|s| !s.is_empty());
        let mut camel = parts.next().unwrap_or("").to_lowercase();
        for p in parts {
            let mut chars = p.chars();
            if let Some(c) = chars.next() {
                camel.extend(c.to_uppercase());
                camel.extend(chars.flat_map(char::to_lowercase));
            }
        }
        (camel, raw.to_lowercase())
    } else {
        let mut snake = String::new();
        let mut prev = false;
        for c in raw.chars() {
            if c.is_ascii_uppercase() && prev {
                snake.push('_')
            }
            snake.extend(c.to_lowercase());
            prev = c.is_ascii_lowercase() || c.is_ascii_digit();
        }
        (raw.into(), snake)
    }
}
pub fn describe_error(raw_code: &Value, raw_message: &Value) -> Value {
    let (code, _) = code_variants(&normalized_code(raw_code));
    let mut msg = j::text(raw_message).trim().to_string();
    if let Ok(v) = serde_json::from_str::<Value>(&msg) {
        if !j::s(&v, "detail").is_empty() {
            msg = j::s(&v, "detail").into()
        }
    }
    let low = code.to_lowercase();
    let m = msg.to_lowercase();
    let (cat, prompt) = if low.contains("misalignment")
        || m.contains("blocked by our safety")
        || m.contains("safety system")
    {
        ("Блокировка Safety Guard","Этот диалог заблокирован системой безопасности (Safety Guard). Хотите снять блокировку?")
    } else if low.contains("cyber") || m.contains("cybersecurity") {
        (
            "Ограничение Cyber Policy",
            "В диалоге сработало ограничение Cyber Policy. Хотите снять его?",
        )
    } else if low.contains("usagelimit") || m.contains("usage limit") {
        (
            "Лимит использования",
            "Последний ход остановлен из-за лимита использования. Хотите очистить ограничение?",
        )
    } else if low.contains("overload") || m.contains("at capacity") {
        (
            "Перегрузка сервера",
            "Последний ход прерван из-за перегрузки сервера. Хотите очистить ошибку?",
        )
    } else if low.contains("unauth") || m.contains("refresh token") || m.contains("access token") {
        (
            "Ошибка авторизации",
            "Последний ход завершился ошибкой авторизации. Хотите очистить ошибку?",
        )
    } else if low.contains("connection")
        || low.contains("http")
        || m.contains("stream disconnected")
        || m.contains("connection failed")
    {
        (
            "Сбой соединения",
            "Последний ход прерван из-за обрыва соединения. Хотите очистить ошибку?",
        )
    } else if m.contains("bad request") || m.contains("not supported") {
        (
            "Ошибка запроса",
            "Последний ход завершился ошибкой запроса. Хотите очистить ошибку?",
        )
    } else {
        (
            "Ограничение Codex",
            "Этот диалог заблокирован или завершился с ошибкой. Хотите снять блокировку?",
        )
    };
    json!({"code":code,"category":cat,"prompt":prompt,"message":if msg.is_empty(){cat.into()}else{msg}})
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (tempfile::TempDir, CodexProvider, PathBuf, Vec<j::Line>) {
        let d = tempfile::tempdir().unwrap();
        let root = d.path().join("codex_sandbox");
        let path = root.join("sessions/2026/01/01/rollout-2026-01-01-thread-one.jsonl");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let rows = vec![
            json!({"ordinal":0,"type":"session_meta","payload":{"id":"thread-one","cwd":"/tmp/проект"}}),
            json!({"ordinal":1,"type":"response_item","payload":{"type":"message","role":"user","id":"u1","content":[{"type":"input_text","text":"Привет 🐈"}]}}),
            json!({"ordinal":2,"type":"event_msg","payload":{"type":"item_completed","item":{"id":"u1","type":"UserMessage","content":"Привет 🐈"}}}),
            json!({"ordinal":3,"type":"response_item","payload":{"type":"reasoning","summary":[{"type":"summary_text","text":"analysis"}],"encrypted_content":"keep"}}),
            json!({"ordinal":4,"type":"response_item","payload":{"type":"message","role":"assistant","id":"a1","content":[{"type":"output_text","text":"Ответ"},{"type":"image","data":"keep"}]},"unknown":17}),
            json!({"ordinal":5,"type":"event_msg","payload":{"type":"item_completed","item":{"id":"a1","type":"AgentMessage","text":"Ответ"}}}),
            json!({"ordinal":6,"type":"event_msg","payload":{"type":"task_complete","turn_id":"turn-one","last_agent_message":"Ответ"}}),
        ];
        let raw = rows
            .iter()
            .map(|v| serde_json::to_string(v).unwrap() + "\r\n")
            .collect::<String>();
        fs::write(&path, raw).unwrap();
        fs::write(
            root.join("session_index.jsonl"),
            "{\"id\":\"thread-one\",\"thread_name\":\"Тестовая сессия\"}\n",
        )
        .unwrap();
        let c = Connection::open(root.join("thread_history_1.sqlite")).unwrap();
        c.execute_batch("CREATE TABLE thread_items(thread_id TEXT, data TEXT);CREATE TABLE thread_turns(thread_id TEXT,turn_id TEXT,status TEXT,error_json TEXT,rollout_ordinal INTEGER,rollout_end_byte_offset INTEGER);CREATE TABLE thread_realtime_items(thread_id TEXT,data TEXT);CREATE TABLE thread_history_projection_state(thread_id TEXT,next_rollout_byte_offset INTEGER);INSERT INTO thread_items VALUES('thread-one','ours'),('another','theirs');INSERT INTO thread_turns VALUES('thread-one','turn-one','completed',NULL,6,1000);INSERT INTO thread_history_projection_state VALUES('thread-one',1000);").unwrap();
        let lines = j::read_lines(&path).unwrap();
        let p = CodexProvider::new(ProviderPaths {
            id: "codex".into(),
            name: "Codex".into(),
            short: "CX".into(),
            root,
            backup_root: d.path().join("backups"),
        });
        (d, p, path, lines)
    }
    #[test]
    fn streaming_edit_mirrors_projection_and_restores_exactly() {
        let (_d, p, path, lines) = fixture();
        let original = fs::read(&path).unwrap();
        let key = format!("0:b{}", lines[4].offset);
        let pre = p
            .call(
                "edit_preview",
                &json!({"cid":"thread-one","key":key}),
                &OpContext::default(),
            )
            .unwrap();
        let r=p.call("apply_edit",&json!({"cid":"thread-one","key":key,"revision":pre["revision"],"text":"Новый ответ ✓"}),&OpContext::default()).unwrap();
        let current = j::read_lines(&path).unwrap();
        assert_eq!(current[0].raw, lines[0].raw);
        assert_eq!(current[3].raw, lines[3].raw);
        let v = current[4].obj().unwrap();
        assert_eq!(v["unknown"], 17);
        assert_eq!(v["payload"]["content"][1]["data"], "keep");
        assert_eq!(
            current[5].obj().unwrap()["payload"]["item"]["text"],
            "Новый ответ ✓"
        );
        assert_eq!(
            current[6].obj().unwrap()["payload"]["last_agent_message"],
            "Новый ответ ✓"
        );
        let c = Connection::open(p.projection()).unwrap();
        let n: i64 = c
            .query_row(
                "SELECT COUNT(*) FROM thread_items WHERE thread_id='thread-one'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(n, 0);
        let n: i64 = c
            .query_row(
                "SELECT COUNT(*) FROM thread_items WHERE thread_id='another'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(n, 1);
        drop(c);
        p.call(
            "restore",
            &json!({"backup_dir":r["backup"]}),
            &OpContext::default(),
        )
        .unwrap();
        assert_eq!(fs::read(&path).unwrap(), original);
        let c = Connection::open(p.projection()).unwrap();
        let n: i64 = c
            .query_row("SELECT COUNT(*) FROM thread_items", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 2);
    }
    #[test]
    fn head_cut_renumbers_and_preserves_meta() {
        let (_d, p, path, lines) = fixture();
        let key = format!("0:b{}", lines[4].offset);
        let pre = p
            .call(
                "preview_cut",
                &json!({"cid":"thread-one","key":key,"mode":"head"}),
                &OpContext::default(),
            )
            .unwrap();
        p.call(
            "apply_cut",
            &json!({"cid":"thread-one","key":key,"mode":"head","revision":pre["revision"]}),
            &OpContext::default(),
        )
        .unwrap();
        let rows = j::read_lines(&path).unwrap();
        assert_eq!(rows.len(), 4);
        assert_eq!(rows[0].raw, lines[0].raw);
        for (i, row) in rows.iter().enumerate() {
            assert_eq!(row.obj().unwrap()["ordinal"], i);
        }
        assert_eq!(
            rows[1].obj().unwrap()["payload"]["content"][0]["text"],
            "Ответ"
        );
    }
    #[test]
    fn invalid_ordinal_rejects_before_mutation() {
        let (_d, p, path, mut lines) = fixture();
        let mut v = lines[3].obj().unwrap();
        v["ordinal"] = json!(100);
        lines[3].set(&v).unwrap();
        let bytes = j::write_lines(&lines);
        fs::write(&path, &bytes).unwrap();
        let key = format!("0:b{}", j::read_lines(&path).unwrap()[4].offset);
        let rev = p.revision("thread-one").unwrap();
        assert!(p
            .call(
                "apply_edit",
                &json!({"cid":"thread-one","key":key,"revision":rev,"text":"bad"}),
                &OpContext::default()
            )
            .is_err());
        assert_eq!(fs::read(path).unwrap(), bytes);
        assert!(!p.paths.backup_root.exists());
    }
    #[test]
    fn lock_unlock_updates_rollout_and_projection() {
        let (_d, p, path, lines) = fixture();
        let r = p
            .call(
                "lock_chat",
                &json!({"cid":"thread-one","message":"A test block","code":"cyber_policy"}),
                &OpContext::default(),
            )
            .unwrap();
        assert_eq!(r["turns_locked"], 1);
        let block = p.detect_block("thread-one").unwrap().unwrap();
        assert_eq!(block["code"], "cyberPolicy");
        let r = p
            .call(
                "unlock_chat",
                &json!({"cid":"thread-one"}),
                &OpContext::default(),
            )
            .unwrap();
        assert_eq!(r["turns_unlocked"], 1);
        assert!(p.detect_block("thread-one").unwrap().is_none());
        let current = j::read_lines(&path).unwrap();
        for n in 0..6 {
            assert_eq!(current[n].raw, lines[n].raw);
        }
        assert!(current[6].obj().unwrap()["payload"].get("error").is_none());
    }
    #[test]
    fn delete_and_restore_index_and_projection() {
        let (_d, p, path, _) = fixture();
        let original = fs::read(&path).unwrap();
        let r = p
            .call(
                "delete_conv",
                &json!({"cid":"thread-one"}),
                &OpContext::default(),
            )
            .unwrap();
        assert!(!path.exists());
        assert!(fs::read_to_string(p.index()).unwrap().is_empty());
        p.call(
            "restore",
            &json!({"backup_dir":r["backup"]}),
            &OpContext::default(),
        )
        .unwrap();
        assert_eq!(fs::read(path).unwrap(), original);
        assert!(fs::read_to_string(p.index())
            .unwrap()
            .contains("Тестовая сессия"));
    }
    #[test]
    fn byte_pagination_and_exports() {
        let (_d, p, _, _) = fixture();
        let r = p
            .call(
                "entries",
                &json!({"cid":"thread-one","limit":1,"include_tools":true}),
                &OpContext::default(),
            )
            .unwrap();
        assert_eq!(r["page_msgs"], 1);
        let before = p
            .call(
                "entries",
                &json!({"cid":"thread-one","limit":1,"include_tools":true,"before":r["cursor"]}),
                &OpContext::default(),
            )
            .unwrap();
        assert_eq!(before["turns"][0]["items"][0]["text"], "Привет 🐈");
        let doc = p
            .call(
                "export_chat",
                &json!({"cid":"thread-one","include_tools":true}),
                &OpContext::default(),
            )
            .unwrap();
        assert_eq!(doc["meta"]["title"], "Тестовая сессия");
        assert_eq!(doc["entries"].as_array().unwrap().len(), 3);
        assert_eq!(doc["entries"][1]["subtype"], "thinking");
    }
}
