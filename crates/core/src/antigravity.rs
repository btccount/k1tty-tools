use crate::pbwire::{self, bytes_at, encode_field, parse_fields, string_at};
use crate::provider::{OpContext, Provider, ProviderPaths};
use anyhow::{anyhow, bail, ensure, Context, Result};
use chrono::{DateTime, Local, Utc};
#[cfg(unix)]
use fs2::FileExt;
use regex::Regex;
use rusqlite::{params, types::Value as SqlValue, Connection, OpenFlags, OptionalExtension};
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File, FileTimes},
    io::Write,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{Duration, UNIX_EPOCH},
};

pub struct AntigravityProvider {
    paths: ProviderPaths,
    turns_cache: Mutex<BTreeMap<String, (Vec<(u128, u64)>, Arc<Vec<Value>>, usize)>>,
    count_cache: Mutex<BTreeMap<String, (Vec<(u128, u64)>, Option<i64>)>>,
    live_unlocked: Mutex<BTreeSet<String>>,
}

fn txt<'a>(value: &'a Value, key: &str) -> &'a str {
    value.get(key).and_then(Value::as_str).unwrap_or("")
}
fn num(value: &Value) -> i64 {
    value
        .as_i64()
        .or_else(|| value.as_f64().map(|n| n as i64))
        .or_else(|| value.as_str().and_then(|s| s.parse().ok()))
        .unwrap_or(-1_000_000_000)
}
fn safe_id(cid: &str) -> Result<()> {
    ensure!(
        !cid.is_empty()
            && cid != "."
            && cid != ".."
            && !cid.contains(['/', '\\', '\0', ':'])
            && Path::new(cid).file_name().and_then(|v| v.to_str()) == Some(cid),
        "Недопустимый ID диалога."
    );
    Ok(())
}
fn required<'a>(d: &'a Value, key: &str) -> Result<&'a str> {
    d.get(key)
        .and_then(Value::as_str)
        .filter(|v| !v.is_empty())
        .ok_or_else(|| anyhow!("Требуется {key}."))
}
fn key_idx(d: &Value) -> Result<i64> {
    if let Some(i) = d["key"].as_i64() {
        return Ok(i);
    }
    required(d, "key")?
        .parse()
        .context("Недопустимый номер шага")
}
fn ro(db: &Path) -> Result<Connection> {
    let con = Connection::open_with_flags(
        db,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    con.busy_timeout(Duration::from_secs(30))?;
    Ok(con)
}
fn rw(db: &Path) -> Result<Connection> {
    let con = Connection::open_with_flags(
        db,
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    con.busy_timeout(Duration::from_secs(30))?;
    Ok(con)
}
fn db_probe_with_timeout(db: &Path, timeout: Duration) -> Result<()> {
    if !db.is_file() {
        return Ok(());
    }
    let con = Connection::open_with_flags(
        db,
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    con.busy_timeout(timeout)?;
    con.execute_batch("BEGIN IMMEDIATE; ROLLBACK;")?;
    Ok(())
}
fn db_probe(db: &Path) -> Result<()> {
    db_probe_with_timeout(db, Duration::from_millis(120))
}
fn wal_checkpoint_passive(con: &Connection) {
    let _ = con.execute_batch("PRAGMA wal_checkpoint(PASSIVE);");
}
fn sql_bytes(v: &SqlValue) -> Vec<u8> {
    match v {
        SqlValue::Text(s) => s.as_bytes().to_vec(),
        SqlValue::Blob(b) => b.clone(),
        SqlValue::Null => Vec::new(),
        x => format!("{x:?}").into_bytes(),
    }
}
fn sql_text(v: &SqlValue) -> String {
    String::from_utf8_lossy(&sql_bytes(v)).into_owned()
}
fn sql_num(v: &SqlValue) -> i64 {
    match v {
        SqlValue::Integer(n) => *n,
        SqlValue::Real(n) => *n as i64,
        _ => sql_text(v)
            .parse::<f64>()
            .map(|v| v as i64)
            .unwrap_or(-1_000_000_000),
    }
}
fn body_like(source: &SqlValue, bytes: Vec<u8>) -> SqlValue {
    if matches!(source, SqlValue::Blob(_)) {
        SqlValue::Blob(bytes)
    } else {
        SqlValue::Text(String::from_utf8_lossy(&bytes).into_owned())
    }
}
fn file_list(dir: &Path) -> Vec<PathBuf> {
    let mut out: Vec<_> = fs::read_dir(dir)
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .collect();
    out.sort();
    out
}
fn size(path: &Path) -> u64 {
    if path.is_file() {
        return fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    }
    walkdir::WalkDir::new(path)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
        .filter_map(|e| e.metadata().ok())
        .map(|m| m.len())
        .sum()
}
fn atomic(path: &Path, data: &[u8]) -> Result<()> {
    let parent = path.parent().ok_or_else(|| anyhow!("Path has no parent"))?;
    fs::create_dir_all(parent)?;
    let mut temp = tempfile::NamedTempFile::new_in(parent)?;
    if let Ok(meta) = fs::metadata(path) {
        temp.as_file().set_permissions(meta.permissions())?;
    }
    temp.write_all(data)?;
    temp.as_file().sync_all()?;
    temp.persist(path).map_err(|e| e.error)?;
    Ok(())
}
fn preserve_times(src: &Path, dst: &Path) -> Result<()> {
    let meta = fs::metadata(src)?;
    let mut times = FileTimes::new();
    if let Ok(t) = meta.modified() {
        times = times.set_modified(t);
    }
    if let Ok(t) = meta.accessed() {
        times = times.set_accessed(t);
    }
    File::options().write(true).open(dst)?.set_times(times)?;
    Ok(())
}
fn copy_tree(src: &Path, dst: &Path) -> Result<()> {
    if !src.exists() {
        return Ok(());
    }
    for e in walkdir::WalkDir::new(src).follow_links(false) {
        let e = e?;
        let rel = e.path().strip_prefix(src)?;
        if e.file_type().is_symlink() {
            bail!(
                "Символические ссылки в данных диалога не копируются: {}",
                e.path().display()
            );
        }
        if e.file_type().is_dir() {
            fs::create_dir_all(dst.join(rel))?;
        } else if e.file_type().is_file()
            && e.path().extension().and_then(|s| s.to_str()) != Some("tmp")
        {
            let target = dst.join(rel);
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(e.path(), &target)?;
            preserve_times(e.path(), &target)?;
        }
    }
    Ok(())
}
fn copy_db(src: &Path, dst: &Path) -> Result<()> {
    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent)?;
    }
    let src_con = ro(src)?;
    let mut temp = tempfile::NamedTempFile::new_in(dst.parent().unwrap())?;
    {
        let mut dest = Connection::open(temp.path())?;
        let backup = rusqlite::backup::Backup::new(&src_con, &mut dest)?;
        backup.run_to_completion(128, Duration::from_millis(5), None)?;
    }
    temp.flush()?;
    temp.as_file().sync_all()?;
    for suffix in ["-wal", "-shm"] {
        let p = PathBuf::from(format!("{}{suffix}", dst.display()));
        if p.exists() {
            fs::remove_file(p)?;
        }
    }
    temp.persist(dst).map_err(|e| e.error)?;
    preserve_times(src, dst)?;
    Ok(())
}
fn restore_db_safe(src: &Path, dst: &Path) -> Result<()> {
    if !dst.is_file() {
        copy_db(src, dst)?;
        if let Ok(con) = rw(dst) {
            let _ = con.execute_batch("PRAGMA journal_mode=WAL;");
        }
        return Ok(());
    }
    let src_con = ro(src)?;
    let mut dst_con = rw(dst)?;
    {
        let backup = rusqlite::backup::Backup::new(&src_con, &mut dst_con)?;
        backup.run_to_completion(128, Duration::from_millis(5), None)?;
    }
    let check: String = dst_con.query_row("PRAGMA quick_check", [], |r| r.get(0))?;
    ensure!(check == "ok", "quick_check после восстановления: {check}");
    wal_checkpoint_passive(&dst_con);
    Ok(())
}
fn read_lines(path: &Path) -> Vec<String> {
    fs::read(path)
        .map(|b| {
            String::from_utf8_lossy(&b)
                .lines()
                .filter(|l| !l.trim().is_empty())
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}
fn read_values(path: &Path) -> Vec<Value> {
    read_lines(path)
        .into_iter()
        .filter_map(|l| serde_json::from_str(&l).ok())
        .collect()
}
fn read_value(path: &Path) -> Value {
    fs::read(path)
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or(json!({}))
}
fn modified(path: &Path) -> f64 {
    fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.)
}
fn stat_signature(path: &Path) -> (u128, u64) {
    fs::metadata(path)
        .map(|m| {
            (
                m.modified()
                    .ok()
                    .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                    .map(|d| d.as_nanos())
                    .unwrap_or(0),
                m.len(),
            )
        })
        .unwrap_or((0, 0))
}
fn local_time(ts: f64) -> String {
    DateTime::from_timestamp(ts as i64, 0)
        .map(|d| d.with_timezone(&Local).format("%d.%m.%Y %H:%M").to_string())
        .unwrap_or_default()
}
fn kind(t: &Value) -> &'static str {
    match t.as_str() {
        Some("USER_INPUT") => "user",
        Some("PLANNER_RESPONSE") => "assistant",
        Some("GENERIC") => "tool",
        Some("CHECKPOINT") => "compact",
        Some("SYSTEM_MESSAGE") => "system",
        Some("ERROR" | "ERROR_MESSAGE") => "error",
        Some(_) => "other",
        None => kind_num(num(t)),
    }
}
fn kind_num(t: i64) -> &'static str {
    match t {
        14 => "user",
        15 => "assistant",
        132 => "tool",
        23 => "compact",
        101 => "system",
        17 => "error",
        _ => "other",
    }
}
fn step_name(t: i64) -> &'static str {
    match t {
        14 => "USER_INPUT",
        15 => "PLANNER_RESPONSE",
        132 => "GENERIC",
        23 => "CHECKPOINT",
        101 => "SYSTEM_MESSAGE",
        17 => "ERROR_MESSAGE",
        _ => "OTHER",
    }
}
fn service_spec(t: i64) -> Option<(&'static str, Vec<Vec<u32>>)> {
    match t {
        101 => Some(("Системное сообщение", vec![vec![114, 1]])),
        23 => Some(("Компакция (сводка)", vec![vec![30, 5]])),
        17 => Some(("Сообщение об ошибке", vec![vec![24, 3, 2], vec![24, 3, 1]])),
        132 => Some(("Вывод инструмента", vec![vec![140, 2, 1]])),
        _ => None,
    }
}
fn service_text(t: i64, p: &[u8]) -> Option<(Vec<u32>, String)> {
    service_spec(t).and_then(|(_, paths)| {
        paths
            .into_iter()
            .find_map(|path| string_at(p, &path).map(|s| (path, s)))
    })
}
fn extract_request(s: &str) -> String {
    Regex::new(r"(?s)<USER_REQUEST>(.*?)</USER_REQUEST>")
        .unwrap()
        .captures(s)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().trim().to_owned())
        .unwrap_or_else(|| s.trim().to_owned())
}
fn clean_user(s: &str) -> String {
    if s.contains("<USER_REQUEST>") {
        return extract_request(s);
    }
    let cleaned = Regex::new(r"(?s)<ADDITIONAL_METADATA>.*?</ADDITIONAL_METADATA>\s*")
        .unwrap()
        .replace_all(s, "")
        .trim()
        .to_owned();
    if cleaned.is_empty() {
        s.to_owned()
    } else {
        cleaned
    }
}
fn tool_names(e: &Value) -> Vec<String> {
    e["tool_calls"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|c| {
            c["name"]
                .as_str()
                .filter(|s| !s.is_empty())
                .map(str::to_owned)
        })
        .collect()
}
fn unquote_json_str(s: &str) -> String {
    if s.len() >= 2 && s.starts_with('"') && s.ends_with('"') {
        serde_json::from_str::<String>(s).unwrap_or_else(|_| s.to_owned())
    } else {
        s.to_owned()
    }
}
fn format_tool_call(call: &Value, include_name: bool) -> String {
    let name = txt(call, "name");
    let args = call
        .get("args")
        .filter(|v| !v.is_null())
        .or_else(|| call.get("arguments").filter(|v| !v.is_null()));
    let body = match args {
        Some(Value::Object(map)) => {
            if let Some(cmd) = map
                .get("CommandLine")
                .or_else(|| map.get("command"))
                .and_then(Value::as_str)
                .map(unquote_json_str)
                .filter(|s| !s.trim().is_empty())
            {
                let summary = map
                    .get("toolSummary")
                    .and_then(Value::as_str)
                    .map(unquote_json_str)
                    .unwrap_or_default();
                let summary = summary.trim();
                if !summary.is_empty() {
                    format!("{summary}\n{cmd}")
                } else {
                    cmd
                }
            } else if map.is_empty() {
                String::new()
            } else {
                serde_json::to_string_pretty(map).unwrap_or_default()
            }
        }
        Some(Value::String(s)) => unquote_json_str(s),
        Some(v) => serde_json::to_string_pretty(v).unwrap_or_default(),
        None => String::new(),
    };
    if include_name && !name.is_empty() {
        if body.is_empty() {
            name.to_owned()
        } else {
            format!("[{name}]\n{body}")
        }
    } else if !body.is_empty() {
        body
    } else {
        name.to_owned()
    }
}
fn format_tool_calls(e: &Value, always_name: bool) -> String {
    let Some(calls) = e["tool_calls"].as_array() else {
        return String::new();
    };
    let include_name = always_name || calls.len() > 1;
    calls
        .iter()
        .map(|c| format_tool_call(c, include_name))
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n")
}

impl AntigravityProvider {
    pub fn new(paths: ProviderPaths) -> Self {
        Self {
            paths,
            turns_cache: Mutex::new(BTreeMap::new()),
            count_cache: Mutex::new(BTreeMap::new()),
            live_unlocked: Mutex::new(BTreeSet::new()),
        }
    }
    fn invalidate(&self, cid: Option<&str>) {
        let mut turns = self.turns_cache.lock().unwrap_or_else(|e| e.into_inner());
        let mut counts = self.count_cache.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(cid) = cid {
            turns.remove(cid);
            counts.remove(cid);
        } else {
            turns.clear();
            counts.clear();
        }
    }
    fn conv_dir(&self) -> PathBuf {
        self.paths.root.join("conversations")
    }
    fn db(&self, cid: &str) -> PathBuf {
        self.conv_dir().join(format!("{cid}.db"))
    }
    fn summaries(&self) -> PathBuf {
        self.paths.root.join("conversation_summaries.db")
    }
    fn brain(&self, cid: &str) -> PathBuf {
        self.paths
            .root
            .join("brain")
            .join(cid)
            .join(".system_generated")
    }
    fn annotation(&self, cid: &str) -> PathBuf {
        self.paths
            .root
            .join("annotations")
            .join(format!("{cid}.pbtxt"))
    }
    fn test_mode(&self) -> bool {
        std::env::var("ANTIGRAVITY_WEB_TEST").as_deref() == Ok("1")
            && self
                .paths
                .root
                .to_string_lossy()
                .to_lowercase()
                .contains("sandbox")
    }
    fn running(&self) -> Vec<String> {
        crate::process::snapshot()
            .remove(&self.paths.id)
            .unwrap_or_default()
    }
    fn label(&self, k: &str) -> String {
        match k {
            "user" => "Вы",
            "assistant" => &self.paths.name,
            "tool" => "Инструмент",
            "compact" => "Компакция",
            "system" => "Система",
            "error" => "Ошибка",
            _ => "Прочее",
        }
        .to_owned()
    }
    fn history_path(&self, cid: &str) -> PathBuf {
        let logs = self.brain(cid).join("logs");
        let full = logs.join("transcript_full.jsonl");
        if full.is_file() {
            full
        } else {
            logs.join("transcript.jsonl")
        }
    }
    pub fn read_history(&self, cid: &str) -> Vec<Value> {
        read_values(&self.history_path(cid))
    }
    fn presence_guard(&self, cid: &str) -> Result<Option<File>> {
        safe_id(cid)?;
        #[cfg(unix)]
        {
            let p = self.paths.root.join("presence").join(format!("{cid}.lock"));
            if p.is_file() {
                let file = File::open(p)?;
                file.try_lock_exclusive().context("presence lock активен")?;
                return Ok(Some(file));
            }
        }
        Ok(None)
    }
    fn is_live_unlocked(&self, cid: &str) -> bool {
        self.live_unlocked
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .contains(cid)
    }
    fn is_presence_locked(&self, cid: &str) -> bool {
        self.presence_guard(cid).is_err()
    }
    fn presence_guard_for_write(&self, cid: &str) -> Result<(Option<File>, bool)> {
        match self.presence_guard(cid) {
            Ok(guard) => Ok((guard, false)),
            Err(_err) if self.is_live_unlocked(cid) => Ok((None, true)),
            Err(err) => Err(err),
        }
    }
    fn unlock_db(&self, cid: &str) -> Result<Value> {
        safe_id(cid)?;
        ensure!(self.db(cid).is_file(), "Диалог {cid} не найден.");
        db_probe_with_timeout(&self.db(cid), Duration::from_millis(800)).with_context(|| {
            format!(
                "SQLite-база {} сейчас физически занята. Подождите секунду или перезапустите Hub.",
                self.paths.name
            )
        })?;
        db_probe_with_timeout(&self.summaries(), Duration::from_millis(800)).with_context(
            || {
                format!(
                "База сводок {} сейчас физически занята. Подождите секунду или перезапустите Hub.",
                self.paths.name
            )
            },
        )?;
        self.live_unlocked
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(cid.to_owned());
        let presence_locked = self.is_presence_locked(cid);
        Ok(json!({
            "ok": true,
            "provider": self.paths.id,
            "cid": cid,
            "locked": false,
            "file_locked": false,
            "reason": "",
            "lock_reason": "",
            "live_unlocked": true,
            "presence_locked": presence_locked,
            "can_unlock_db": true,
            "can_restart_hub": true,
        }))
    }
    fn relock_db(&self, cid: &str) -> Result<Value> {
        safe_id(cid)?;
        self.live_unlocked
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(cid);
        let status = self.lock_status(Some(cid))?;
        Ok(json!({
            "ok": true,
            "provider": self.paths.id,
            "cid": cid,
            "locked": status["locked"],
            "file_locked": status["locked"],
            "reason": status["reason"],
            "lock_reason": status["reason"],
            "live_unlocked": status["live_unlocked"],
            "presence_locked": status["presence_locked"],
            "can_unlock_db": true,
            "can_restart_hub": true,
        }))
    }
    fn restart_hub(&self, cid: Option<&str>) -> Result<Value> {
        if let Some(cid) = cid {
            safe_id(cid)?;
        }
        let daemon_dir = self.paths.root.join("daemon");
        let pids = crate::process::stop_antigravity_hubs(&self.paths.id, &daemon_dir);
        if !pids.is_empty() {
            for _ in 0..30 {
                let alive = crate::process::any_pid_alive(&pids);
                let pres = cid.is_some_and(|c| self.is_presence_locked(c));
                if !alive && !pres {
                    break;
                }
                std::thread::sleep(Duration::from_millis(100));
            }
        }
        crate::process::snapshot_force(true);
        if let Some(c) = cid {
            if !self.is_presence_locked(c) {
                self.live_unlocked
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .remove(c);
            }
        }
        self.invalidate(cid);
        let status = self.lock_status(cid)?;
        Ok(json!({
            "ok": true,
            "provider": self.paths.id,
            "cid": cid,
            "pids": pids,
            "restarted": !pids.is_empty(),
            "locked": status["locked"],
            "file_locked": status["locked"],
            "reason": status["reason"],
            "lock_reason": status["reason"],
            "live_unlocked": status["live_unlocked"],
            "presence_locked": status["presence_locked"],
            "can_unlock_db": true,
            "can_restart_hub": true,
        }))
    }
    fn write_gate(&self, cid: &str) -> Result<(Option<File>, Vec<String>)> {
        let (guard, bypassed_presence) = self.presence_guard_for_write(cid)?;
        ensure!(self.db(cid).is_file(), "Диалог {cid} не найден.");
        db_probe_with_timeout(&self.db(cid), Duration::from_millis(800)).with_context(|| {
            format!(
                "Файл диалога {} занят. Закройте приложение и повторите.",
                self.paths.name
            )
        })?;
        db_probe_with_timeout(&self.summaries(), Duration::from_millis(800))?;
        let mut warns = Vec::new();
        if bypassed_presence {
            warns.push(format!(
                "Live Unlock: правка записана в WAL-БД {} при активном диалоге. Чтобы приложение подхватило изменения из БД, переоткройте диалог или перезапустите Hub.",
                self.paths.name
            ));
        } else if !self.running().is_empty() {
            warns.push(format!(
                "{} запущен, но файлы этого диалога свободны. Изменения появятся в приложении после его перезапуска.",
                self.paths.name
            ));
        }
        if self.test_mode() {
            warns.push("Тест-режим: данные пишутся в песочницу.".to_owned());
        }
        Ok((guard, warns))
    }
    fn state(&self) -> Value {
        let running = self.running();
        let avail = self.conv_dir().is_dir();
        let blocked = !self.test_mode()
            && !running.is_empty()
            && file_list(&self.conv_dir())
                .iter()
                .filter(|p| p.extension().and_then(|x| x.to_str()) == Some("db"))
                .any(|p| db_probe(p).is_err());
        let note = if !avail {
            format!("Каталог не найден: {}", self.paths.root.display())
        } else if self.test_mode() {
            "Тест-режим: изменения пишутся в песочницу.".into()
        } else if blocked {
            format!(
                "Файлы диалогов заняты ({}). Закройте приложение для правки.",
                crate::process::format_running(&running)
            )
        } else if !running.is_empty() {
            format!(
                "Запущено: {}, файлы свободны. Изменения применятся после перезапуска.",
                crate::process::format_running(&running)
            )
        } else {
            "Файлы свободны.".into()
        };
        json!({"id":self.paths.id,"name":self.paths.name,"short":self.paths.short,"available":avail,"writable":avail&&!blocked,"blocked":blocked,"running":running,"test":self.test_mode(),"appdata":self.paths.root,"backups_root":self.paths.backup_root,"note":note})
    }
    fn titles(&self) -> BTreeMap<String, String> {
        let mut titles = BTreeMap::new();
        if let Ok(con) = ro(&self.summaries()) {
            if let Ok(mut stmt) =
                con.prepare("SELECT conversation_id,title FROM conversation_summaries")
            {
                if let Ok(rows) = stmt.query_map([], |r| {
                    Ok((
                        sql_text(&r.get::<_, SqlValue>(0)?),
                        sql_text(&r.get::<_, SqlValue>(1)?),
                    ))
                }) {
                    for (id, title) in rows.flatten() {
                        titles.insert(id, title);
                    }
                }
            }
        }
        titles
    }
    fn convs(&self) -> Result<Value> {
        let titles = self.titles();
        let mut rows = Vec::new();
        for p in file_list(&self.conv_dir()) {
            if p.extension().and_then(|x| x.to_str()) != Some("db") {
                continue;
            }
            let Some(cid) = p.file_stem().and_then(|x| x.to_str()) else {
                continue;
            };
            let Ok(meta) = fs::metadata(&p) else {
                continue;
            };
            let sig = vec![
                stat_signature(&p),
                stat_signature(&PathBuf::from(format!("{}-wal", p.display()))),
            ];
            let cached = self
                .count_cache
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .get(cid)
                .cloned();
            let count = if let Some((old_sig, count)) = cached.filter(|(s, _)| s == &sig) {
                let _ = old_sig;
                count
            } else {
                let count = ro(&p).ok().and_then(|con| {
                    con.query_row("SELECT count(*) FROM steps", [], |r| r.get::<_, i64>(0))
                        .ok()
                });
                self.count_cache
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .insert(cid.to_owned(), (sig, count));
                count
            };
            let ts = modified(&p);
            rows.push(json!({"provider":self.paths.id,"id":cid,"title":titles.get(cid).filter(|s|!s.is_empty()).cloned().unwrap_or_else(||format!("Диалог {}",cid.chars().take(8).collect::<String>())),"subtitle":count.map(|n|format!("шагов: {n}")).unwrap_or_else(||"SQLite + transcript".into()),"badge":count.map(|n|format!("{n} ш.")).unwrap_or_else(||self.paths.short.clone()),"size":meta.len(),"mtime":local_time(ts),"mtime_ts":ts,"writable":true}));
        }
        rows.sort_by(|a, b| {
            b["mtime_ts"]
                .as_f64()
                .partial_cmp(&a["mtime_ts"].as_f64())
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        Ok(json!(rows))
    }
    fn db_types(&self, cid: &str) -> BTreeMap<i64, i64> {
        let mut map = BTreeMap::new();
        if let Ok(c) = ro(&self.db(cid)) {
            if let Ok(mut s) = c.prepare("SELECT idx,step_type FROM steps") {
                if let Ok(rows) = s.query_map([], |r| Ok((r.get(0)?, r.get(1)?))) {
                    for (idx, t) in rows.flatten() {
                        map.insert(idx, t);
                    }
                }
            }
        }
        map
    }
    fn entry(&self, e: &Value, types: &BTreeMap<i64, i64>, readonly: bool) -> Value {
        let orig_kind = kind(&e["type"]);
        let idx = num(&e["step_index"]);
        let stype = types.get(&idx);
        let tools = tool_names(e);
        let content = txt(e, "content");
        let thinking = txt(e, "thinking").trim();
        let mut actions = Vec::<&str>::new();
        if !readonly {
            if stype == Some(&14)
                || (stype == Some(&15)
                    && (!content.trim().is_empty() || !tools.is_empty() || !thinking.is_empty()))
                || (stype.is_some_and(|t| service_spec(*t).is_some()) && !content.trim().is_empty())
            {
                actions.push("edit");
            }
            if orig_kind == "user" {
                actions.push("cut_head");
            }
            actions.push("cut_tail");
        }
        let tool_only = orig_kind == "assistant" && content.trim().is_empty() && !tools.is_empty();
        let thinking_only = orig_kind == "assistant"
            && content.trim().is_empty()
            && tools.is_empty()
            && !thinking.is_empty();
        let (k, subtype, label, text, tool_field) = if tool_only {
            let head = tools.iter().take(3).cloned().collect::<Vec<_>>().join(", ");
            let extra = if tools.len() > 3 {
                format!(" (+{})", tools.len() - 3)
            } else {
                String::new()
            };
            let calls_text = format_tool_calls(e, false);
            let body = if !thinking.is_empty() && !calls_text.is_empty() {
                format!("{calls_text}\n\n[thinking]\n{thinking}")
            } else if !calls_text.is_empty() {
                calls_text
            } else {
                tools.join(", ")
            };
            (
                "tool",
                "tool_use".to_owned(),
                format!("Инструмент: {head}{extra}"),
                body,
                json!(tools.join(", ")),
            )
        } else if thinking_only {
            (
                "tool",
                "thinking".to_owned(),
                "Размышления".to_owned(),
                thinking.to_owned(),
                json!("thinking"),
            )
        } else if orig_kind == "tool" {
            let mut label = "Вывод инструмента".to_owned();
            if !tools.is_empty() {
                label.push_str(": ");
                label.push_str(&tools.iter().take(3).cloned().collect::<Vec<_>>().join(", "));
            }
            (
                "tool",
                "tool_result".to_owned(),
                label,
                content.to_owned(),
                if tools.is_empty() {
                    json!("tool_result")
                } else {
                    json!(tools.join(", "))
                },
            )
        } else {
            let mut label = self.label(orig_kind);
            if !tools.is_empty() && orig_kind != "user" {
                label.push_str(": ");
                label.push_str(&tools.iter().take(3).cloned().collect::<Vec<_>>().join(", "));
            }
            let body = if orig_kind == "assistant" && !tools.is_empty() {
                let calls_text = format_tool_calls(e, true);
                if calls_text.is_empty() {
                    content.to_owned()
                } else {
                    format!("{content}\n\n{calls_text}")
                }
            } else {
                content.to_owned()
            };
            (
                orig_kind,
                e["type"].as_str().unwrap_or(orig_kind).to_lowercase(),
                label,
                body,
                if tools.is_empty() {
                    Value::Null
                } else {
                    json!(tools.join(", "))
                },
            )
        };
        json!({"key":idx.to_string(),"step_index":e["step_index"],"kind":k,"subtype":subtype,"label":label,"text":text,"ts":txt(e,"created_at"),"tool":tool_field,"status":e["status"],"in_db":readonly||stype.is_some(),"actions":actions})
    }
    fn flat_entries(&self, cid: &str) -> (Arc<Vec<Value>>, usize) {
        let sig = vec![
            stat_signature(&self.history_path(cid)),
            stat_signature(&self.db(cid)),
            stat_signature(&PathBuf::from(format!("{}-wal", self.db(cid).display()))),
        ];
        if let Some((_, items, turns)) = self
            .turns_cache
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(cid)
            .filter(|(s, _, _)| s == &sig)
        {
            return (Arc::clone(items), *turns);
        }
        let types = self.db_types(cid);
        let history = self.read_history(cid);
        let mut turns = 0;
        let entries = history
            .iter()
            .enumerate()
            .map(|(i, e)| {
                if i == 0 || kind(&e["type"]) == "user" {
                    turns += 1;
                }
                self.entry(e, &types, false)
            })
            .collect();
        let entries = Arc::new(entries);
        let mut cache = self.turns_cache.lock().unwrap_or_else(|e| e.into_inner());
        if cache.len() >= 8 {
            if let Some(key) = cache.keys().next().cloned() {
                cache.remove(&key);
            }
        }
        cache.insert(cid.to_owned(), (sig, Arc::clone(&entries), turns));
        (entries, turns)
    }
    fn page(&self, flat: &[Value], d: &Value) -> (Vec<Value>, usize, usize, usize) {
        let tools = d["include_tools"].as_bool().unwrap_or(false);
        let flat: Vec<&Value> = flat
            .iter()
            .filter(|it| {
                tools
                    || (matches!(txt(it, "kind"), "user" | "assistant")
                        && !txt(it, "text").trim().is_empty())
            })
            .collect();
        let total = flat.len();
        let end = if d["before"].is_null() {
            total
        } else {
            num(&d["before"]).max(0) as usize
        }
        .min(total);
        let limit = d["limit"].as_u64().unwrap_or(80).clamp(1, 150) as usize;
        let max_items = if tools { 350 } else { limit * 2 };
        let mut start = end;
        let mut msgs = 0;
        while start > 0 && msgs < limit && end - start < max_items {
            start -= 1;
            if matches!(txt(&flat[start], "kind"), "user" | "assistant") {
                msgs += 1;
            }
        }
        (
            flat[start..end].iter().map(|v| (*v).clone()).collect(),
            msgs,
            total,
            start,
        )
    }
    fn entries(&self, cid: &str, d: &Value) -> Result<Value> {
        let (flat, turns) = self.flat_entries(cid);
        let (window, msgs, total, start) = self.page(&flat, d);
        let status = self.lock_status(Some(cid))?;
        Ok(json!({
            "provider": self.paths.id,
            "cid": cid,
            "turns": if window.is_empty() { json!([]) } else { json!([{ "items": window }]) },
            "page_msgs": msgs,
            "total_label": format!("ходов: {turns} · записей: {total}"),
            "cursor": if start > 0 { json!(start.to_string()) } else { Value::Null },
            "has_more": start > 0,
            "writable": true,
            "file_locked": status["locked"],
            "lock_reason": status["reason"],
            "live_unlocked": status["live_unlocked"],
            "presence_locked": status["presence_locked"],
            "can_unlock_db": status["can_unlock_db"],
            "can_restart_hub": status["can_restart_hub"],
        }))
    }
    fn step(&self, cid: &str, idx: i64) -> Result<(i64, Vec<u8>)> {
        let con = ro(&self.db(cid))?;
        con.query_row(
            "SELECT step_type,step_payload FROM steps WHERE idx=?",
            [idx],
            |r| {
                Ok((
                    r.get(0)?,
                    r.get::<_, Option<Vec<u8>>>(1)?.unwrap_or_default(),
                ))
            },
        )
        .optional()?
        .ok_or_else(|| {
            anyhow!(
                "Шага #{idx} нет в базе данных (существует только в транскрипте), править нечего."
            )
        })
    }
    fn edit_preview(&self, cid: &str, idx: i64) -> Result<Value> {
        let (stype, payload) = self.step(cid, idx)?;
        if stype == 14 {
            let text = self
                .read_history(cid)
                .into_iter()
                .find(|e| num(&e["step_index"]) == idx)
                .map(|e| txt(&e, "content").to_owned())
                .unwrap_or_default();
            let hint = "Показан полный текст сообщения включая блоки <ADDITIONAL_METADATA> и <USER_SETTINGS_CHANGE>. Блок <USER_REQUEST> должен остаться. В базу пишется текст запроса, метаданные в транскрипты.";
            return Ok(
                json!({"text":text,"kind":"Сообщение пользователя","hint":hint,"revision":idx.to_string(),"writable":true,"full_user":true}),
            );
        }
        if stype == 15 {
            let entry = self
                .read_history(cid)
                .into_iter()
                .find(|e| num(&e["step_index"]) == idx)
                .unwrap_or(Value::Null);
            let content = txt(&entry, "content").to_owned();
            if !content.trim().is_empty() {
                return Ok(
                    json!({"text":content,"kind":format!("Ответ {}",self.paths.name),"hint":"Текст обновится в SQLite (protobuf), обоих транскриптах, чанках и поисковом индексе FTS5.","revision":idx.to_string(),"writable":true,"full_user":false}),
                );
            }
            if let Some(calls) = entry["tool_calls"].as_array().filter(|a| !a.is_empty()) {
                let mut obj = serde_json::Map::new();
                let clean_calls: Vec<Value> = calls
                    .iter()
                    .map(|c| {
                        let name = c["name"].as_str().unwrap_or("");
                        let args = c
                            .get("args")
                            .or_else(|| c.get("arguments"))
                            .cloned()
                            .unwrap_or(json!({}));
                        json!({"name": name, "args": args})
                    })
                    .collect();
                obj.insert("tool_calls".into(), Value::Array(clean_calls));
                let th = txt(&entry, "thinking").trim();
                if !th.is_empty() {
                    obj.insert("thinking".into(), json!(th));
                }
                let text = serde_json::to_string_pretty(&Value::Object(obj))?;
                return Ok(
                    json!({"text":text,"kind":"Вызов инструмента","hint":"Отредактируйте JSON аргументов tool_calls (и thinking при наличии). Обновится в SQLite (protobuf) и транскриптах.","revision":idx.to_string(),"writable":true,"full_user":false}),
                );
            }
            let th = {
                let t = txt(&entry, "thinking");
                if !t.trim().is_empty() {
                    t.to_owned()
                } else {
                    string_at(&payload, &[20, 3]).unwrap_or_default()
                }
            };
            return Ok(
                json!({"text":th,"kind":"Размышления","hint":"Текст размышлений обновится в SQLite (protobuf) и транскриптах.","revision":idx.to_string(),"writable":true,"full_user":false}),
            );
        }
        let (label, _) = service_spec(stype)
            .ok_or_else(|| anyhow!("Шаг типа {stype} не поддерживает редактирование текста."))?;
        let(_,text)=service_text(stype,&payload).ok_or_else(||anyhow!("В этом служебном шаге нет редактируемого текстового поля в БД (возможно, бинарные данные или вложение)."))?;
        Ok(
            json!({"text":text,"kind":label,"hint":"Текст обновится в SQLite (protobuf) и транскриптах. Перед записью создаётся полная резервная копия.","revision":idx.to_string(),"writable":true,"full_user":false}),
        )
    }
    fn write_transcript(&self, cid: &str, name: &str, lines: &[String]) -> Result<()> {
        let path = self.brain(cid).join("logs").join(name);
        let data = if lines.is_empty() {
            String::new()
        } else {
            format!("{}\n", lines.join("\n"))
        };
        atomic(&path, data.as_bytes())?;
        let chunks = self
            .brain(cid)
            .join("logs/chunks")
            .join(name.trim_end_matches(".jsonl"));
        fs::create_dir_all(&chunks)?;
        for p in file_list(&chunks) {
            if p.extension().and_then(|x| x.to_str()) == Some("jsonl") {
                fs::remove_file(p)?;
            }
        }
        for (i, b) in data.as_bytes().chunks(102400).enumerate() {
            atomic(&chunks.join(format!("{i:08}.jsonl")), b)?;
        }
        Ok(())
    }
    fn full_backup(&self, cid: &str, tag: &str) -> Result<PathBuf> {
        let stamp = Local::now().format("%Y%m%d-%H%M%S-%6f");
        let dir = self
            .paths
            .backup_root
            .join(cid)
            .join(format!("{stamp}-{tag}"));
        fs::create_dir_all(&dir)?;
        copy_db(&self.db(cid), &dir.join(format!("{cid}.db")))?;
        copy_tree(
            &self.brain(cid),
            &dir.join("brain").join(cid).join(".system_generated"),
        )?;
        if self.summaries().is_file() {
            copy_db(&self.summaries(), &dir.join("conversation_summaries.db"))?;
        }
        if self.annotation(cid).is_file() {
            fs::create_dir_all(dir.join("annotations"))?;
            fs::copy(
                self.annotation(cid),
                dir.join("annotations").join(format!("{cid}.pbtxt")),
            )?;
        }
        atomic(
            &dir.join("backup.meta.json"),
            &serde_json::to_vec_pretty(
                &json!({"conversation_id":cid,"created":Local::now().to_rfc3339(),"original_db":self.db(cid),"appdata":self.paths.root}),
            )?,
        )?;
        Ok(dir)
    }
    fn validate_backup(&self, dir: &Path) -> Result<PathBuf> {
        let root = self
            .paths
            .backup_root
            .canonicalize()
            .context("Папка резервных копий не найдена")?;
        let p = dir.canonicalize()?;
        ensure!(
            p.is_dir() && p.starts_with(&root) && p != root && p.parent() != Some(root.as_path()),
            "Недопустимый путь резервной копии {}.",
            self.paths.name
        );
        Ok(p)
    }
    fn restore(&self, dir: &Path) -> Result<()> {
        let p = self.validate_backup(dir)?;
        let meta = read_value(&p.join("backup.meta.json"));
        let cid = required(&meta, "conversation_id")?;
        safe_id(cid)?;
        ensure!(
            p.join(format!("{cid}.db")).is_file(),
            "В копии отсутствует база диалога"
        );
        let (_presence, _) = self.presence_guard_for_write(cid)?;
        db_probe_with_timeout(&self.db(cid), Duration::from_millis(800))?;
        db_probe_with_timeout(&self.summaries(), Duration::from_millis(800))?;
        restore_db_safe(&p.join(format!("{cid}.db")), &self.db(cid))?;
        let source = p.join("brain").join(cid).join(".system_generated");
        if source.exists() {
            let dest = self.brain(cid);
            if dest.exists() {
                fs::remove_dir_all(&dest)?;
            }
            copy_tree(&source, &dest)?;
        }
        if p.join("conversation_summaries.db").is_file() {
            restore_db_safe(&p.join("conversation_summaries.db"), &self.summaries())?;
        }
        let ann = p.join("annotations").join(format!("{cid}.pbtxt"));
        if ann.is_file() && !self.annotation(cid).exists() {
            fs::create_dir_all(self.paths.root.join("annotations"))?;
            fs::copy(ann, self.annotation(cid))?;
        }
        self.invalidate(None);
        Ok(())
    }
    fn apply_edit(&self, cid: &str, idx: i64, text: &str, ctx: &OpContext) -> Result<Value> {
        ensure!(
            !text.trim().is_empty(),
            "Пустой текст, сохранение не выполнено."
        );
        let (guard, warns) = self.write_gate(cid)?;
        let (stype, _) = self.step(cid, idx)?;
        let text = if stype == 14 || stype == 15 {
            text.trim()
        } else {
            text
        };
        let backup = if ctx.auto_backup {
            ctx.phase("backup");
            Some(self.full_backup(cid, "webedit")?)
        } else {
            None
        };
        ctx.phase("edit");
        let result = self.edit_text(cid, idx, text);
        drop(guard);
        let mut rep = match result {
            Ok(r) => r,
            Err(error) => {
                if let Some(b) = &backup {
                    self.restore(b).with_context(|| {
                        format!(
                            "Ошибка правки: {error}; восстановление не удалось. Копия: {}",
                            b.display()
                        )
                    })?;
                }
                return Err(error);
            }
        };
        rep["backup"] = json!(backup);
        rep["warns"] = json!(warns);
        self.invalidate(Some(cid));
        ctx.phase("done");
        Ok(rep)
    }
    fn edit_text(&self, cid: &str, idx: i64, text: &str) -> Result<Value> {
        let (stype, payload) = self.step(cid, idx)?;
        let old_entry = self
            .read_history(cid)
            .into_iter()
            .find(|e| num(&e["step_index"]) == idx)
            .unwrap_or(Value::Null);
        let old_content = txt(&old_entry, "content").to_owned();
        let planner_tool_edit = stype == 15
            && old_content.trim().is_empty()
            && old_entry["tool_calls"]
                .as_array()
                .is_some_and(|a| !a.is_empty());
        let planner_thinking_edit =
            stype == 15 && old_content.trim().is_empty() && !planner_tool_edit;
        let mut parsed_calls: Option<Vec<Value>> = None;
        let mut parsed_thinking: Option<String> = None;
        let mut old_pb = None;
        let mut request = text.to_owned();
        let side;
        let new_payload = match stype {
            14 => {
                side = "user";
                ensure!(
                    text.contains("<USER_REQUEST>") && text.contains("</USER_REQUEST>"),
                    "В тексте должен остаться блок <USER_REQUEST>…</USER_REQUEST>"
                );
                request = extract_request(text);
                patch_user_payload(&payload, &request, &extract_request(&old_content))?
            }
            15 if planner_tool_edit => {
                side = "tool";
                let parsed: Value =
                    serde_json::from_str(text).context("Некорректный JSON для tool_calls")?;
                let calls = if let Some(arr) = parsed.as_array() {
                    arr.clone()
                } else if let Some(arr) = parsed["tool_calls"].as_array() {
                    arr.clone()
                } else {
                    bail!("Ожидался JSON-объект с полем tool_calls или массив вызовов");
                };
                let th = parsed["thinking"].as_str().map(str::to_owned);
                let patched = patch_planner_tool_calls(&payload, &calls, th.as_deref())?;
                parsed_calls = Some(calls);
                parsed_thinking = th;
                patched
            }
            15 if planner_thinking_edit => {
                side = "thinking";
                pbwire::replace_nested(&payload, &[20, 3], text.as_bytes(), true, true)?
            }
            15 => {
                side = "agent";
                let p = pbwire::replace_nested(&payload, &[20, 1], text.as_bytes(), true, true)?;
                pbwire::replace_nested(&p, &[20, 8], text.as_bytes(), true, false)?
            }
            _ => {
                side = step_name(stype);
                let (path, old) = service_text(stype, &payload).ok_or_else(|| {
                    anyhow!("В этом служебном шаге не найдено текстовое поле для замены")
                })?;
                old_pb = Some(old);
                pbwire::replace_nested(&payload, &path, text.as_bytes(), false, true)?
            }
        };
        let mut con = rw(&self.db(cid))?;
        let tx = con.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let current: Vec<u8> =
            tx.query_row("SELECT step_payload FROM steps WHERE idx=?", [idx], |r| {
                r.get(0)
            })?;
        ensure!(
            current == payload,
            "Диалог изменился во время редактирования. Откройте сообщение заново."
        );
        tx.execute(
            "UPDATE steps SET step_payload=? WHERE idx=?",
            params![new_payload, idx],
        )?;
        let integrity: String = tx.query_row("PRAGMA integrity_check", [], |r| r.get(0))?;
        ensure!(integrity == "ok", "integrity_check fail: {integrity}");
        tx.commit()?;
        wal_checkpoint_passive(&con);
        let mut rep =
            json!({"cid":cid,"step_index":idx,"side":side.to_lowercase(),"integrity_check":"ok"});
        let mut updated = false;
        for name in ["transcript_full.jsonl", "transcript.jsonl"] {
            let path = self.brain(cid).join("logs").join(name);
            if !path.exists() {
                continue;
            }
            let mut lines = read_lines(&path);
            let mut changed = false;
            for line in &mut lines {
                let Ok(mut obj) = serde_json::from_str::<Value>(line) else {
                    continue;
                };
                if num(&obj["step_index"]) == idx
                    && (txt(&obj, "type") == step_name(stype) || service_spec(stype).is_some())
                {
                    if planner_tool_edit {
                        if let Some(new_calls) = &parsed_calls {
                            let existing =
                                obj["tool_calls"].as_array().cloned().unwrap_or_default();
                            let merged: Vec<Value> = new_calls
                                .iter()
                                .enumerate()
                                .map(|(i, c)| {
                                    let mut item =
                                        existing.get(i).cloned().unwrap_or_else(|| json!({}));
                                    if let Some(n) = c["name"].as_str().filter(|s| !s.is_empty()) {
                                        item["name"] = json!(n);
                                    }
                                    if let Some(a) = c.get("args") {
                                        item["args"] = a.clone();
                                    }
                                    item
                                })
                                .collect();
                            obj["tool_calls"] = Value::Array(merged);
                        }
                        if let Some(th) = &parsed_thinking {
                            obj["thinking"] = json!(th);
                        }
                    } else if planner_thinking_edit {
                        obj["thinking"] = json!(text);
                    } else {
                        let content = if stype == 14 || stype == 15 {
                            text.to_owned()
                        } else {
                            let cur = txt(&obj, "content");
                            old_pb
                                .as_deref()
                                .filter(|p| !p.is_empty() && cur.contains(p))
                                .map(|p| cur.replacen(p, text, 1))
                                .unwrap_or_else(|| text.to_owned())
                        };
                        obj["content"] = json!(content);
                    }
                    *line = serde_json::to_string(&obj)?;
                    changed = true;
                    break;
                }
            }
            if changed {
                self.write_transcript(cid, name, &lines)?;
                updated = true;
            }
        }
        rep["transcripts_updated"] = json!(updated);
        if stype == 132 {
            let out = self
                .brain(cid)
                .join("steps")
                .join(idx.to_string())
                .join("output.txt");
            if out.is_file() {
                let old = String::from_utf8_lossy(&fs::read(&out)?).into_owned();
                let new = old_pb
                    .as_deref()
                    .filter(|p| !p.is_empty() && old.contains(p))
                    .map(|p| old.replacen(p, text, 1))
                    .unwrap_or_else(|| text.to_owned());
                atomic(&out, new.as_bytes())?;
                rep["step_output_updated"] = json!(true);
            }
        }
        if stype == 14 || (stype == 15 && !planner_tool_edit && !planner_thinking_edit) {
            let new = if stype == 14 {
                format!("{request}\n")
            } else {
                text.to_owned()
            };
            let report = self.fts_replace(cid, idx, side, new.as_bytes())?;
            if let Some(m) = report.as_object() {
                rep.as_object_mut().unwrap().extend(m.clone());
            }
        }
        Ok(rep)
    }
}

fn patch_planner_tool_calls(
    payload: &[u8],
    calls: &[Value],
    thinking: Option<&str>,
) -> Result<Vec<u8>> {
    let mut out = Vec::with_capacity(payload.len());
    let mut patched_outer = false;
    for field in pbwire::try_parse_fields(payload)? {
        if field.number == 20 && field.wire_type == 2 {
            let inner = field.bytes().unwrap_or_default();
            let mut inner_out = Vec::with_capacity(inner.len());
            let mut call_idx = 0usize;
            let mut patched_thinking = false;
            for sub in pbwire::try_parse_fields(inner)? {
                if sub.number == 7 && sub.wire_type == 2 {
                    if let Some(c) = calls.get(call_idx) {
                        let mut cbytes = sub.bytes().unwrap_or_default().to_vec();
                        if let Some(name) = c["name"].as_str().filter(|s| !s.is_empty()) {
                            cbytes = pbwire::replace_nested(
                                &cbytes,
                                &[2],
                                name.as_bytes(),
                                true,
                                false,
                            )?;
                        }
                        if let Some(args_val) = c.get("args") {
                            let args_str = if let Some(s) = args_val.as_str() {
                                s.to_owned()
                            } else {
                                serde_json::to_string(args_val)?
                            };
                            cbytes = pbwire::replace_nested(
                                &cbytes,
                                &[3],
                                args_str.as_bytes(),
                                true,
                                false,
                            )?;
                        }
                        inner_out.extend(encode_field(7, 2, &cbytes));
                    } else {
                        inner_out.extend(sub.raw);
                    }
                    call_idx += 1;
                } else if sub.number == 3 && sub.wire_type == 2 {
                    if let Some(th) = thinking {
                        inner_out.extend(encode_field(3, 2, th.as_bytes()));
                        patched_thinking = true;
                    } else {
                        inner_out.extend(sub.raw);
                    }
                } else {
                    inner_out.extend(sub.raw);
                }
            }
            if let Some(th) = thinking {
                if !patched_thinking && !th.is_empty() {
                    inner_out.extend(encode_field(3, 2, th.as_bytes()));
                }
            }
            out.extend(encode_field(20, 2, &inner_out));
            patched_outer = true;
        } else {
            out.extend(field.raw);
        }
    }
    ensure!(
        patched_outer,
        "В шаге не найден блок PLANNER_RESPONSE (field 20)"
    );
    Ok(out)
}

fn patch_user_payload(payload: &[u8], new_text: &str, old_request: &str) -> Result<Vec<u8>> {
    let mut changed = false;
    let mut result = Vec::new();
    let mut consumed = 0;
    for field in parse_fields(payload) {
        consumed += field.raw.len();
        if field.number == 19 && field.wire_type == 2 {
            let bytes = field.bytes().unwrap();
            let mut inner = Vec::new();
            let mut used = 0;
            for f in parse_fields(bytes) {
                used += f.raw.len();
                if f.number == 2 && f.wire_type == 2 {
                    inner.extend(encode_field(2, 2, new_text.as_bytes()));
                    changed = true;
                } else if f.number == 3 && f.wire_type == 2 {
                    let b = f.bytes().unwrap();
                    let mut part = Vec::new();
                    let mut part_used = 0;
                    for p in parse_fields(b) {
                        part_used += p.raw.len();
                        if p.number == 1
                            && p.wire_type == 2
                            && String::from_utf8_lossy(p.bytes().unwrap()).trim()
                                == old_request.trim()
                        {
                            part.extend(encode_field(1, 2, new_text.as_bytes()));
                            changed = true;
                        } else {
                            part.extend(p.raw);
                        }
                    }
                    part.extend_from_slice(&b[part_used..]);
                    inner.extend(encode_field(3, 2, &part));
                } else {
                    inner.extend(f.raw);
                }
            }
            inner.extend_from_slice(&bytes[used..]);
            result.extend(encode_field(19, 2, &inner));
        } else {
            result.extend(field.raw);
        }
    }
    result.extend_from_slice(&payload[consumed..]);
    ensure!(
        changed,
        "В этом сообщении не найден текст запроса для замены (возможно, только вложение)"
    );
    Ok(result)
}

impl AntigravityProvider {
    fn fts_replace(&self, cid: &str, idx: i64, side: &str, new_bytes: &[u8]) -> Result<Value> {
        if !self.summaries().is_file() {
            return Ok(json!({"fts_updated":false,"fts_reason":"no summaries db"}));
        }
        ensure!(matches!(side, "user" | "agent"), "Invalid FTS side");
        let mut con = rw(&self.summaries())?;
        if !table_exists(&con, "conversations_fts") {
            return Ok(json!({"fts_updated":false,"fts_reason":"no fts table"}));
        }
        let tx = con.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let sql=format!("SELECT rowid,chunk_first_step,chunk_last_step,{side}_step_byte_offsets,{side}_body FROM conversations_fts WHERE conversation_id=?");
        let rows = {
            let mut stmt = tx.prepare(&sql)?;
            let rows = stmt
                .query_map([cid], |r| {
                    Ok((
                        r.get::<_, i64>(0)?,
                        r.get::<_, SqlValue>(1)?,
                        r.get::<_, SqlValue>(2)?,
                        r.get::<_, SqlValue>(3)?,
                        r.get::<_, SqlValue>(4)?,
                    ))
                })?
                .collect::<std::result::Result<Vec<_>, _>>()?;
            rows
        };
        let mut done = None;
        for (rowid, first, last, offsets, body) in rows {
            let Ok(offs) = serde_json::from_slice::<Vec<Vec<Value>>>(&sql_bytes(&offsets)) else {
                continue;
            };
            let Some(mine) = offs.iter().find(|o| o.len() >= 2 && num(&o[0]) == idx) else {
                continue;
            };
            let body_bytes = sql_bytes(&body);
            let start = num(&mine[1]).max(0) as usize;
            ensure!(start <= body_bytes.len(), "Повреждены смещения FTS");
            let end = offs
                .iter()
                .filter(|o| o.len() >= 2)
                .map(|o| num(&o[1]))
                .filter(|o| *o > start as i64)
                .min()
                .map(|n| n.max(0) as usize)
                .unwrap_or(body_bytes.len())
                .min(body_bytes.len());
            let old_len = end - start;
            let delta = new_bytes.len() as i64 - old_len as i64;
            let mut new_body = body_bytes[..start].to_vec();
            new_body.extend_from_slice(new_bytes);
            new_body.extend_from_slice(&body_bytes[end..]);
            let mut remapped = 0;
            let new_offsets: Vec<_> = offs
                .into_iter()
                .map(|o| {
                    if o.len() < 2 {
                        return o;
                    }
                    let step = num(&o[0]);
                    let off = num(&o[1]);
                    let new = if step == idx {
                        start as i64
                    } else if off >= end as i64 {
                        remapped += 1;
                        off + delta
                    } else {
                        off
                    };
                    vec![o[0].clone(), json!(new)]
                })
                .collect();
            tx.execute(&format!("UPDATE conversations_fts SET {side}_step_byte_offsets=?,{side}_body=? WHERE rowid=?"),params![serde_json::to_string(&new_offsets)?,body_like(&body,new_body),rowid])?;
            done = Some(
                json!({"fts_updated":true,"fts_rowid":rowid,"chunk_range":[sql_num(&first),sql_num(&last)],"old_seg_len":old_len,"new_seg_len":new_bytes.len(),"offsets_remapped":remapped,"chunk_first_step":sql_num(&first),"chunk_last_step":sql_num(&last)}),
            );
            break;
        }
        tx.commit()?;
        wal_checkpoint_passive(&con);
        Ok(done.unwrap_or_else(||json!({"fts_updated":false,"fts_reason":format!("step {idx} not found in FTS {side} body")})))
    }
    fn cut_summaries(
        &self,
        cid: &str,
        keep: i64,
        mode: &str,
        kept: &[Value],
        db_count: i64,
    ) -> Result<()> {
        if !self.summaries().exists() {
            return Ok(());
        }
        let mut con = rw(&self.summaries())?;
        let tx = con.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let last_user = kept
            .iter()
            .filter(|e| txt(e, "type") == "USER_INPUT")
            .map(|e| num(&e["step_index"]))
            .max()
            .unwrap_or(0)
            .max(0);
        let rows = {
            let mut s = tx.prepare(
                "SELECT rowid,raw_summary FROM conversation_summaries WHERE conversation_id=?",
            )?;
            let v = s
                .query_map([cid], |r| {
                    Ok((
                        r.get::<_, i64>(0)?,
                        r.get::<_, Option<Vec<u8>>>(1)?.unwrap_or_default(),
                    ))
                })?
                .collect::<std::result::Result<Vec<_>, _>>()?;
            v
        };
        for (rowid, raw) in rows {
            let mut changes = BTreeMap::from([(2, db_count.max(0) as u64)]);
            if mode == "tail" {
                changes.insert(16, last_user as u64);
                tx.execute("UPDATE conversation_summaries SET step_count=?,last_user_input_step_index=? WHERE rowid=?",params![db_count,last_user,rowid])?;
            } else {
                tx.execute(
                    "UPDATE conversation_summaries SET step_count=? WHERE rowid=?",
                    params![db_count, rowid],
                )?;
            }
            tx.execute(
                "UPDATE conversation_summaries SET raw_summary=? WHERE rowid=?",
                params![pbwire::patch_varints(&raw, &changes), rowid],
            )?;
        }
        if table_exists(&tx, "conversations_fts") {
            let rows = {
                let mut s=tx.prepare("SELECT rowid,chunk_first_step,chunk_last_step,user_step_byte_offsets,agent_step_byte_offsets,user_body,agent_body FROM conversations_fts WHERE conversation_id=?")?;
                let v = s
                    .query_map([cid], |r| {
                        Ok((
                            r.get::<_, i64>(0)?,
                            r.get::<_, SqlValue>(1)?,
                            r.get::<_, SqlValue>(2)?,
                            r.get::<_, SqlValue>(3)?,
                            r.get::<_, SqlValue>(4)?,
                            r.get::<_, SqlValue>(5)?,
                            r.get::<_, SqlValue>(6)?,
                        ))
                    })?
                    .collect::<std::result::Result<Vec<_>, _>>()?;
                v
            };
            for (rowid, first, last, uoff, aoff, ubody, abody) in rows {
                let first = sql_num(&first);
                let last = sql_num(&last);
                if (mode == "tail" && first > keep) || (mode == "head" && last < keep) {
                    tx.execute("DELETE FROM conversations_fts WHERE rowid=?", [rowid])?;
                    continue;
                }
                if (mode == "tail" && last <= keep) || (mode == "head" && first >= keep) {
                    continue;
                }
                let (uoff, ubody) = cut_fts_body(&uoff, &ubody, keep, mode)?;
                let (aoff, abody) = cut_fts_body(&aoff, &abody, keep, mode)?;
                if mode == "tail" {
                    tx.execute("UPDATE conversations_fts SET chunk_last_step=?,user_step_byte_offsets=?,agent_step_byte_offsets=?,user_body=?,agent_body=? WHERE rowid=?",params![keep,uoff,aoff,ubody,abody,rowid])?;
                } else {
                    tx.execute("UPDATE conversations_fts SET chunk_first_step=?,user_step_byte_offsets=?,agent_step_byte_offsets=?,user_body=?,agent_body=? WHERE rowid=?",params![first.max(keep),uoff,aoff,ubody,abody,rowid])?;
                }
            }
        }
        tx.commit()?;
        wal_checkpoint_passive(&con);
        Ok(())
    }
    fn remove_counts(&self, cid: &str, keep: i64, mode: &str) -> (usize, usize, usize) {
        let b = self.brain(cid);
        let messages = file_list(&b.join("messages"))
            .iter()
            .filter(|p| {
                p.extension().and_then(|e| e.to_str()) == Some("json")
                    && p.file_name().and_then(|n| n.to_str()) != Some("read.json")
            })
            .filter(|p| msg_step(&read_value(p)).is_some_and(|i| is_removed(i, keep, mode)))
            .count();
        let steps = file_list(&b.join("steps"))
            .iter()
            .filter(|p| p.is_dir())
            .filter_map(|p| {
                p.file_name()
                    .and_then(|s| s.to_str())
                    .and_then(|s| s.parse::<i64>().ok())
            })
            .filter(|i| is_removed(*i, keep, mode))
            .count();
        let tasks = file_list(&b.join("tasks"))
            .iter()
            .filter_map(|p| task_idx(p))
            .filter(|i| is_removed(*i, keep, mode))
            .count();
        (messages, steps, tasks)
    }
    fn rollback(&self, cid: &str, keep: i64, mode: &str, preview: bool) -> Result<Value> {
        ensure!(
            matches!(mode, "head" | "tail"),
            "Недопустимый режим обрезки."
        );
        let entries = self.read_history(cid);
        let removed: Vec<_> = entries
            .iter()
            .filter(|e| is_removed(num(&e["step_index"]), keep, mode))
            .cloned()
            .collect();
        let kept: Vec<_> = entries
            .iter()
            .filter(|e| !is_removed(num(&e["step_index"]), keep, mode))
            .cloned()
            .collect();
        let con = ro(&self.db(cid))?;
        let total: i64 = con.query_row("SELECT count(*) FROM steps", [], |r| r.get(0))?;
        let compare = if mode == "tail" { ">" } else { "<" };
        let to_delete: i64 = con.query_row(
            &format!("SELECT count(*) FROM steps WHERE idx {compare} ?"),
            [keep],
            |r| r.get(0),
        )?;
        drop(con);
        let mut rep = json!({"cid":cid,"keep_step":keep,"mode":mode,"preview_only":preview,"transcript_lines_before":entries.len(),"transcript_lines_removed":removed.len(),"transcript_lines_kept":kept.len(),"db_rows_before":total});
        if preview {
            let (m, s, t) = self.remove_counts(cid, keep, mode);
            rep["will_delete"] = json!({"db_rows":to_delete,"transcript_lines":removed.len(),"messages_files":m,"steps_output_dirs":s,"task_logs":t});
            return Ok(rep);
        }
        let mut con = rw(&self.db(cid))?;
        let tx = con.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        tx.execute(&format!("DELETE FROM steps WHERE idx {compare} ?"), [keep])?;
        let mut gen_deleted = 0;
        if table_exists(&tx, "gen_metadata") {
            let rows = {
                let mut s = tx.prepare("SELECT idx,data FROM gen_metadata")?;
                let r = s
                    .query_map([], |r| {
                        Ok((
                            r.get::<_, i64>(0)?,
                            r.get::<_, Option<Vec<u8>>>(1)?.unwrap_or_default(),
                        ))
                    })?
                    .collect::<std::result::Result<Vec<_>, _>>()?;
                r
            };
            for (idx, data) in rows {
                let attrs = gen_attrs(&data);
                let last = attrs
                    .get("last_step_index")
                    .and_then(|s| s.parse::<i64>().ok())
                    .unwrap_or(-1);
                if is_removed(last, keep, mode) {
                    gen_deleted += tx.execute("DELETE FROM gen_metadata WHERE idx=?", [idx])?;
                }
            }
        }
        let integrity: String = tx.query_row("PRAGMA integrity_check", [], |r| r.get(0))?;
        ensure!(integrity == "ok", "integrity_check fail: {integrity}");
        let (count, max_idx): (i64, Option<i64>) =
            tx.query_row("SELECT count(*),max(idx) FROM steps", [], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })?;
        tx.commit()?;
        wal_checkpoint_passive(&con);
        drop(con);
        rep["gen_metadata_removed"] = json!(gen_deleted);
        rep["integrity_check"] = json!(["ok"]);
        rep["db_rows_after"] = json!(count);
        rep["db_max_idx_after"] = json!(max_idx);
        for name in ["transcript.jsonl", "transcript_full.jsonl"] {
            let p = self.brain(cid).join("logs").join(name);
            if !p.exists() {
                continue;
            }
            let lines: Vec<_> = read_lines(&p)
                .into_iter()
                .filter(|l| {
                    let idx = serde_json::from_str::<Value>(l)
                        .map(|v| num(&v["step_index"]))
                        .unwrap_or(-1_000_000_000);
                    !is_removed(idx, keep, mode)
                })
                .collect();
            self.write_transcript(cid, name, &lines)?;
        }
        let mdir = self.brain(cid).join("messages");
        let mut removed_msgs = Vec::new();
        let mut kept_msgs = Vec::new();
        for path in file_list(&mdir) {
            if path.extension().and_then(|e| e.to_str()) != Some("json")
                || path.file_name().and_then(|e| e.to_str()) == Some("read.json")
            {
                continue;
            }
            let Ok(bytes) = fs::read(&path) else {
                continue;
            };
            let Ok(msg) = serde_json::from_slice::<Value>(&bytes) else {
                continue;
            };
            if msg_step(&msg).is_some_and(|i| is_removed(i, keep, mode)) {
                fs::remove_file(path)?;
                removed_msgs.push(msg);
            } else {
                kept_msgs.push(msg);
            }
        }
        let read_path = mdir.join("read.json");
        if read_path.is_file() {
            let mut obj = read_value(&read_path);
            if let Some(map) = obj.as_object_mut() {
                for m in &removed_msgs {
                    if let Some(id) = m["id"].as_str() {
                        map.remove(id);
                    }
                }
                atomic(&read_path, &serde_json::to_vec(&obj)?)?;
            }
        }
        rep["messages_removed"] = json!(removed_msgs.len());
        let protected = protected_refs(&kept, &kept_msgs);
        let mut outs = 0;
        let mut tasks = 0;
        for path in file_list(&self.brain(cid).join("steps")) {
            if !path.is_dir() {
                continue;
            }
            let Some(idx) = path
                .file_name()
                .and_then(|n| n.to_str())
                .and_then(|s| s.parse::<i64>().ok())
            else {
                continue;
            };
            if is_removed(idx, keep, mode) && !protected.contains(&idx) {
                fs::remove_dir_all(path)?;
                outs += 1;
            }
        }
        for path in file_list(&self.brain(cid).join("tasks")) {
            if let Some(idx) = task_idx(&path) {
                if is_removed(idx, keep, mode) && !protected.contains(&idx) {
                    fs::remove_file(path)?;
                    tasks += 1;
                }
            }
        }
        rep["steps_outputs_removed"] = json!(outs);
        rep["task_logs_removed"] = json!(tasks);
        match self.cut_summaries(cid, keep, mode, &kept, count) {
            Ok(()) => rep["summaries_updated"] = json!(true),
            Err(e) => {
                rep["summaries_updated"] = json!(false);
                rep["summaries_error"] = json!(e.to_string());
            }
        }
        Ok(rep)
    }
    fn preview_cut(&self, cid: &str, idx: i64, mode: &str) -> Result<Value> {
        let keep = if mode == "tail" { idx - 1 } else { idx };
        let raw = self.rollback(cid, keep, mode, true)?;
        let (flat, _) = self.flat_entries(cid);
        let removed: Vec<_> = flat
            .iter()
            .filter(|e| is_removed(num(&e["step_index"]), keep, mode))
            .collect();
        let list:Vec<_>=removed.iter().take(25).map(|e|json!({"key":format!("#{}",num(&e["step_index"])),"label":e["label"],"text":txt(e,"text").replace('\n'," ").chars().take(140).collect::<String>()})).collect();
        Ok(
            json!({"report":{"provider":self.paths.id,"mode":mode,"steps_count":removed.len(),"raw_count":raw["will_delete"]["db_rows"],"raw_label":"шагов в БД","bytes":null,"extra_label":format!("транскрипт: -{} / -0",raw["will_delete"]["transcript_lines"]),"steps_preview":list,"raw":raw},"revision":format!("{mode}:{keep}"),"writable":true,"hint":"Будут очищены SQLite-база steps, транскрипты, чанки, очередь сообщений, выводы инструментов и FTS5-индекс. Создаётся полная резервная копия."}),
        )
    }
    fn apply_cut(&self, cid: &str, idx: i64, mode: &str, ctx: &OpContext) -> Result<Value> {
        let keep = if mode == "tail" { idx - 1 } else { idx };
        let (guard, warns) = self.write_gate(cid)?;
        let original = self.read_history(cid);
        let backup = if ctx.auto_backup {
            ctx.phase("backup");
            Some(self.full_backup(cid, "web")?)
        } else {
            None
        };
        ctx.phase("cut");
        let result = self.rollback(cid, keep, mode, false);
        drop(guard);
        let mut rep = match result {
            Ok(r) => r,
            Err(error) => {
                if let Some(b) = &backup {
                    self.restore(b).with_context(|| {
                        format!(
                            "Ошибка обрезки: {error}; восстановление не удалось. Копия: {}",
                            b.display()
                        )
                    })?;
                }
                return Err(error);
            }
        };
        rep["backup"] = json!(backup);
        rep["leaks"] = if backup.is_some() {
            ctx.phase("leak");
            let removed: Vec<_> = original
                .iter()
                .filter(|e| is_removed(num(&e["step_index"]), keep, mode))
                .cloned()
                .collect();
            let kept: Vec<_> = original
                .iter()
                .filter(|e| !is_removed(num(&e["step_index"]), keep, mode))
                .cloned()
                .collect();
            json!(self.check_leak(cid, &removed, &kept))
        } else {
            json!([])
        };
        rep["warns"] = json!(warns);
        self.invalidate(Some(cid));
        ctx.phase("done");
        Ok(rep)
    }
    fn check_leak(&self, cid: &str, removed: &[Value], kept: &[Value]) -> Vec<Value> {
        let kept_text = kept
            .iter()
            .map(|e| txt(e, "content").replace('\n', " "))
            .collect::<Vec<_>>()
            .join("\n")
            .to_lowercase();
        let mut samples = BTreeSet::new();
        for e in removed {
            let text = txt(e, "content");
            if text.chars().count() > 120 {
                let frag = text
                    .chars()
                    .skip(40)
                    .take(80)
                    .collect::<String>()
                    .replace('\n', " ");
                if !kept_text.contains(&frag.to_lowercase()) {
                    samples.insert(frag);
                }
            }
        }
        if samples.is_empty() {
            return Vec::new();
        }
        let mut targets = vec![
            self.brain(cid).join("logs/transcript.jsonl"),
            self.brain(cid).join("logs/transcript_full.jsonl"),
        ];
        targets.extend(
            file_list(&self.brain(cid).join("messages"))
                .into_iter()
                .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("json")),
        );
        let mut leaks = Vec::new();
        for path in targets {
            if let Ok(bytes) = fs::read(&path) {
                for s in &samples {
                    if bytes.windows(s.len()).any(|w| w == s.as_bytes()) {
                        leaks.push(json!([path, s]));
                    }
                }
            }
        }
        if let Ok(con) = ro(&self.summaries()) {
            if table_exists(&con, "conversations_fts") {
                let token_re = Regex::new(r"[A-Za-zА-Яа-я0-9]+").unwrap();
                for sample in samples {
                    let words: Vec<_> = token_re
                        .find_iter(&sample)
                        .map(|m| m.as_str())
                        .filter(|s| s.chars().count() > 3)
                        .take(10)
                        .collect();
                    if words.len() < 3 {
                        continue;
                    }
                    let phrase = format!("\"{}\"", words.join(" "));
                    match con.query_row("SELECT count(*) FROM conversations_fts WHERE conversations_fts MATCH ? AND conversation_id=?",params![phrase,cid],|r|r.get::<_,i64>(0)){Ok(n)if n>0=>leaks.push(json!([format!("{} (FTS MATCH)",self.summaries().display()),sample.chars().take(60).collect::<String>()])),Err(e)=>{leaks.push(json!([format!("{} (проверка FTS не удалась)",self.summaries().display()),e.to_string()]));break;},_=>{}}
                }
            }
        }
        leaks
    }
    fn delete_conv(&self, cid: &str, ctx: &OpContext) -> Result<Value> {
        let (guard, warns) = self.write_gate(cid)?;
        let backup = if ctx.auto_backup {
            ctx.phase("backup");
            Some(self.full_backup(cid, "delete_conv")?)
        } else {
            None
        };
        ctx.phase("cut");
        let result = (|| -> Result<()> {
            for suffix in ["", "-wal", "-shm"] {
                let p = PathBuf::from(format!("{}{suffix}", self.db(cid).display()));
                if p.exists() {
                    fs::remove_file(p)?;
                }
            }
            let brain = self.paths.root.join("brain").join(cid);
            if brain.exists() {
                fs::remove_dir_all(brain)?;
            }
            if self.annotation(cid).exists() {
                fs::remove_file(self.annotation(cid))?;
            }
            if self.summaries().is_file() {
                let mut con = rw(&self.summaries())?;
                let tx = con.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
                tx.execute(
                    "DELETE FROM conversation_summaries WHERE conversation_id=?",
                    [cid],
                )?;
                if table_exists(&tx, "conversations_fts") {
                    tx.execute(
                        "DELETE FROM conversations_fts WHERE conversation_id=?",
                        [cid],
                    )?;
                }
                if table_exists(&tx, "conversation_parents") {
                    tx.execute(
                        "DELETE FROM conversation_parents WHERE conversation_id=? OR parent_id=?",
                        params![cid, cid],
                    )?;
                }
                tx.commit()?;
                wal_checkpoint_passive(&con);
            }
            Ok(())
        })();
        drop(guard);
        if let Err(error) = result {
            if let Some(b) = &backup {
                self.restore(b).with_context(|| {
                    format!(
                        "Ошибка удаления: {error}; восстановление не удалось. Копия: {}",
                        b.display()
                    )
                })?;
            }
            return Err(error);
        }
        let presence = self.paths.root.join("presence").join(format!("{cid}.lock"));
        if presence.exists() {
            let _ = fs::remove_file(presence);
        }
        self.live_unlocked
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(cid);
        self.invalidate(Some(cid));
        ctx.phase("done");
        Ok(json!({"deleted":cid,"provider":self.paths.id,"backup":backup,"warns":warns}))
    }
}

fn table_exists(con: &Connection, name: &str) -> bool {
    con.query_row(
        "SELECT 1 FROM sqlite_master WHERE type='table' AND name=?",
        [name],
        |_| Ok(()),
    )
    .optional()
    .ok()
    .flatten()
    .is_some()
}
fn gen_attrs(data: &[u8]) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    for f in parse_fields(data) {
        if f.number != 1 || f.wire_type != 2 {
            continue;
        }
        for attr in parse_fields(f.bytes().unwrap()) {
            if attr.number != 20 || attr.wire_type != 2 {
                continue;
            }
            let pair = parse_fields(attr.bytes().unwrap());
            if pair.len() >= 2 {
                if let (Some(k), Some(v)) = (pair[0].bytes(), pair[1].bytes()) {
                    map.insert(
                        String::from_utf8_lossy(k).into_owned(),
                        String::from_utf8_lossy(v).into_owned(),
                    );
                }
            }
        }
    }
    map
}
fn is_removed(idx: i64, keep: i64, mode: &str) -> bool {
    if mode == "tail" {
        idx > keep
    } else {
        idx < keep
    }
}
fn msg_step(v: &Value) -> Option<i64> {
    let n = &v["sourceMetadata"]["tool"]["stepIndex"];
    if n.is_null() {
        None
    } else {
        Some(num(n))
    }
}
fn task_idx(p: &Path) -> Option<i64> {
    p.file_name()?
        .to_str()?
        .strip_prefix("task-")?
        .strip_suffix(".log")?
        .parse()
        .ok()
}
fn protected_refs(kept: &[Value], messages: &[Value]) -> BTreeSet<i64> {
    let step = Regex::new(r"steps/(\d+)/output\.txt").unwrap();
    let task = Regex::new(r"task-(\d+)\.log").unwrap();
    let mut out = BTreeSet::new();
    for e in kept.iter().chain(messages) {
        let text = format!(
            "{}\n{}",
            txt(e, "content"),
            e.get("tool_calls")
                .map(Value::to_string)
                .unwrap_or_default()
        );
        for re in [&step, &task] {
            for c in re.captures_iter(&text) {
                if let Ok(n) = c[1].parse() {
                    out.insert(n);
                }
            }
        }
    }
    out
}
fn cut_fts_body(
    offsets: &SqlValue,
    body: &SqlValue,
    keep: i64,
    mode: &str,
) -> Result<(SqlValue, SqlValue)> {
    let Ok(offs) = serde_json::from_slice::<Vec<Vec<Value>>>(&sql_bytes(offsets)) else {
        return Ok((offsets.clone(), body.clone()));
    };
    let bytes = sql_bytes(body);
    let retained: Vec<_> = offs
        .iter()
        .filter(|o| o.len() >= 2 && !is_removed(num(&o[0]), keep, mode))
        .cloned()
        .collect();
    if retained.len() == offs.len() {
        return Ok((offsets.clone(), body.clone()));
    }
    let (new_offsets, new_body) = if mode == "tail" {
        let end = offs
            .iter()
            .filter(|o| o.len() >= 2 && is_removed(num(&o[0]), keep, mode))
            .map(|o| num(&o[1]).max(0) as usize)
            .min()
            .unwrap_or(bytes.len())
            .min(bytes.len());
        (
            retained
                .into_iter()
                .filter(|o| num(&o[1]) < end as i64)
                .collect::<Vec<_>>(),
            bytes[..end].to_vec(),
        )
    } else {
        let start = retained
            .iter()
            .map(|o| num(&o[1]).max(0) as usize)
            .min()
            .unwrap_or(bytes.len())
            .min(bytes.len());
        (
            retained
                .into_iter()
                .map(|o| vec![o[0].clone(), json!(num(&o[1]) - start as i64)])
                .collect(),
            bytes[start..].to_vec(),
        )
    };
    Ok((
        SqlValue::Text(serde_json::to_string(&new_offsets)?),
        body_like(body, new_body),
    ))
}

impl AntigravityProvider {
    fn backup_title(&self, dir: &Path, cid: &str, live: &BTreeMap<String, String>) -> String {
        if let Some(title) = live.get(cid).filter(|s| !s.trim().is_empty()) {
            return title.clone();
        }
        ro(&dir.join("conversation_summaries.db"))
            .ok()
            .and_then(|con| {
                con.query_row(
                    "SELECT title FROM conversation_summaries WHERE conversation_id=?",
                    [cid],
                    |r| r.get::<_, SqlValue>(0),
                )
                .ok()
            })
            .map(|v| sql_text(&v).trim().to_owned())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| cid.to_owned())
    }
    fn backup_transcript(dir: &Path, cid: &str) -> Option<PathBuf> {
        let logs = dir.join("brain").join(cid).join(".system_generated/logs");
        ["transcript_full.jsonl", "transcript.jsonl"]
            .iter()
            .map(|s| logs.join(s))
            .find(|p| p.is_file())
    }
    fn backups(&self, cid: Option<&str>) -> Result<Value> {
        let titles = self.titles();
        let mut groups = Vec::new();
        let mut total_count = 0;
        let mut total_bytes = 0;
        let dirs = if let Some(id) = cid {
            safe_id(id)?;
            vec![self.paths.backup_root.join(id)]
        } else {
            let mut d = file_list(&self.paths.backup_root);
            d.reverse();
            d
        };
        for cdir in dirs {
            if !cdir.is_dir() {
                continue;
            }
            let Some(id) = cdir.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            let mut rows = Vec::new();
            let mut title = titles.get(id).cloned().unwrap_or_default();
            let mut snapshots = file_list(&cdir);
            snapshots.reverse();
            for dir in snapshots {
                if !dir.is_dir() {
                    continue;
                }
                let bytes = size(&dir);
                total_count += 1;
                total_bytes += bytes;
                if title.is_empty() {
                    let t = self.backup_title(&dir, id, &BTreeMap::new());
                    if t != id {
                        title = t;
                    }
                }
                rows.push(json!({"provider":self.paths.id,"cid":id,"dir":dir,"name":dir.file_name().and_then(|n|n.to_str()).unwrap_or_default(),"size":bytes,"meta":read_value(&dir.join("backup.meta.json")),"restorable":true,"inspectable":Self::backup_transcript(&dir,id).is_some()||dir.join(format!("{id}.db")).is_file(),"reason":""}));
            }
            if !rows.is_empty() {
                groups.push(json!({"provider":self.paths.id,"cid":id,"title":if title.is_empty(){id.to_owned()}else{title},"backups":rows}));
            }
        }
        Ok(
            json!({"groups":groups,"root":self.paths.backup_root,"total_count":total_count,"total_bytes":total_bytes}),
        )
    }
    fn backup_entries(&self, dir: &Path, d: &Value) -> Result<Value> {
        let p = self.validate_backup(dir)?;
        let meta = read_value(&p.join("backup.meta.json"));
        let cid = meta["conversation_id"]
            .as_str()
            .filter(|s| !s.is_empty())
            .or_else(|| {
                p.parent()
                    .and_then(Path::file_name)
                    .and_then(|n| n.to_str())
            })
            .ok_or_else(|| anyhow!("В копии не найден ID диалога"))?;
        safe_id(cid)?;
        let mut flat = Vec::new();
        if let Some(path) = Self::backup_transcript(&p, cid) {
            flat = read_values(&path)
                .iter()
                .map(|e| self.entry(e, &BTreeMap::new(), true))
                .collect();
        } else if p.join(format!("{cid}.db")).is_file() {
            let con = ro(&p.join(format!("{cid}.db")))?;
            let mut stmt =
                con.prepare("SELECT idx,step_type,step_payload FROM steps ORDER BY idx")?;
            for row in stmt.query_map([], |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, i64>(1)?,
                    r.get::<_, Option<Vec<u8>>>(2)?.unwrap_or_default(),
                ))
            })? {
                let (idx, stype, payload) = row?;
                let k = kind_num(stype);
                let summary = payload_summary(stype, &payload);
                let tool = txt(&summary, "tool");
                flat.push(json!({"key":idx.to_string(),"step_index":idx,"kind":k,"subtype":k,"label":self.label(k),"text":if txt(&summary,"text").is_empty(){txt(&summary,"args")}else{txt(&summary,"text")},"ts":"","tool":if tool.is_empty(){Value::Null}else{json!(tool)},"status":null,"in_db":true,"actions":[]}));
            }
        }
        let (window, msgs, total, start) = self.page(&flat, d);
        Ok(
            json!({"provider":self.paths.id,"cid":cid,"backup_dir":p,"backup_name":p.file_name().and_then(|n|n.to_str()),"title":self.backup_title(&p,cid,&self.titles()),"readonly":true,"turns":if window.is_empty(){json!([])}else{json!([{ "items":window }])},"page_msgs":msgs,"total_label":format!("записей: {total}"),"cursor":if start>0{json!(start.to_string())}else{Value::Null},"has_more":start>0,"writable":false}),
        )
    }
    fn db_history_fallback(&self, cid: &str) -> Vec<Value> {
        let Ok(con) = ro(&self.db(cid)) else {
            return Vec::new();
        };
        let Ok(mut stmt) =
            con.prepare("SELECT idx,step_type,metadata,step_payload FROM steps ORDER BY idx")
        else {
            return Vec::new();
        };
        let Ok(rows) = stmt.query_map([], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, Option<Vec<u8>>>(2)?.unwrap_or_default(),
                r.get::<_, Option<Vec<u8>>>(3)?.unwrap_or_default(),
            ))
        }) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for (idx, stype, metadata, payload) in rows.flatten() {
            let seconds = bytes_at(&metadata, &[1])
                .and_then(|b| {
                    parse_fields(&b)
                        .iter()
                        .find(|f| f.number == 1 && f.wire_type == 0)
                        .and_then(|f| f.varint())
                })
                .unwrap_or(0);
            let iso = if seconds > 946_684_800 {
                DateTime::from_timestamp(seconds as i64, 0)
                    .map(|d| d.to_rfc3339_opts(chrono::SecondsFormat::AutoSi, true))
                    .unwrap_or_default()
            } else {
                String::new()
            };
            if stype == 14 {
                if let Some(text) = string_at(&payload, &[19, 2]).filter(|s| !s.trim().is_empty()) {
                    out.push(json!({"step_index":idx,"type":"USER_INPUT","content":text,"created_at":iso,"status":"DONE"}));
                }
            } else if stype == 15 {
                let text = string_at(&payload, &[20, 8])
                    .filter(|s| !s.is_empty())
                    .or_else(|| string_at(&payload, &[20, 1]))
                    .unwrap_or_default();
                let think = string_at(&payload, &[20, 3]).unwrap_or_default();
                if !text.trim().is_empty() || !think.trim().is_empty() {
                    out.push(json!({"step_index":idx,"type":"PLANNER_RESPONSE","content":text,"thinking":think,"created_at":iso,"status":"DONE"}));
                }
            }
        }
        out
    }
    fn export_chat(&self, cid: &str, include_tools: bool) -> Result<Value> {
        let mut title = String::new();
        let mut directory = String::new();
        let mut raw_size = size(&self.db(cid));
        let source = self.history_path(cid);
        if source.is_file() {
            raw_size += size(&source);
        }
        if let Ok(con) = ro(&self.summaries()) {
            if let Ok((t, w)) = con.query_row(
                "SELECT title,workspace_uris FROM conversation_summaries WHERE conversation_id=?",
                [cid],
                |r| Ok((r.get::<_, SqlValue>(0)?, r.get::<_, SqlValue>(1)?)),
            ) {
                title = sql_text(&t).trim().to_owned();
                if let Ok(uris) = serde_json::from_str::<Vec<String>>(&sql_text(&w)) {
                    if let Some(first) = uris.first() {
                        let uri = percent_decode(first);
                        if uri.starts_with("file:///") {
                            directory = if uri.as_bytes().get(9) == Some(&b':') {
                                uri[8..].to_owned()
                            } else {
                                uri[7..].to_owned()
                            };
                        }
                    }
                }
            }
        }
        let mut history = self.read_history(cid);
        if history.is_empty() && self.db(cid).is_file() {
            history = self.db_history_fallback(cid);
        }
        let mut entries = Vec::new();
        for e in history {
            let k = kind(&e["type"]);
            let mut text = txt(&e, "content").to_owned();
            if k == "user" {
                text = clean_user(&text);
            }
            let thinking = txt(&e, "thinking").trim();
            if k == "assistant" && !thinking.is_empty() && include_tools {
                entries.push(json!({"kind":"tool","subtype":"thinking","label":"thinking","text":thinking,"ts":txt(&e,"created_at"),"tool":"thinking","status":"ok","code":""}));
            }
            let tools = tool_names(&e);
            if text.trim().is_empty() && !tools.is_empty() {
                text = e["tool_calls"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .map(|call| {
                        let args = call
                            .get("args")
                            .filter(|v| !v.is_null())
                            .or_else(|| call.get("arguments"))
                            .cloned()
                            .unwrap_or(json!({}));
                        format!("{}({})", txt(call, "name"), python_json(&args))
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
            }
            if text.trim().is_empty() && tools.is_empty() {
                continue;
            }
            entries.push(json!({"kind":k,"subtype":e["type"].as_str().filter(|s|!s.is_empty()).unwrap_or(k).to_lowercase(),"label":if tools.is_empty(){k.to_owned()}else{tools.iter().take(3).cloned().collect::<Vec<_>>().join(", ")},"text":text,"ts":txt(&e,"created_at"),"tool":if tools.is_empty(){Value::Null}else{json!(tools.join(", "))},"status":e["status"],"code":""}));
        }
        entries.retain(|e| {
            if include_tools {
                !txt(e, "text").trim().is_empty() || !e["tool"].is_null()
            } else {
                matches!(txt(e, "kind"), "user" | "assistant" | "compact")
                    && !txt(e, "text").trim().is_empty()
            }
        });
        let title = if !title.trim().is_empty() {
            title.trim().chars().take(140).collect::<String>()
        } else {
            entries
                .iter()
                .find(|e| txt(e, "kind") == "user" && !txt(e, "text").trim().is_empty())
                .map(|e| {
                    txt(e, "text")
                        .split_whitespace()
                        .collect::<Vec<_>>()
                        .join(" ")
                        .chars()
                        .take(90)
                        .collect()
                })
                .unwrap_or_else(|| "Imported chat".to_owned())
        };
        let timestamps: Vec<_> = entries
            .iter()
            .map(|e| txt(e, "ts"))
            .filter(|s| !s.is_empty())
            .collect();
        let now = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::AutoSi, true);
        let created = timestamps.first().copied().unwrap_or(&now).to_owned();
        let updated = timestamps.last().copied().unwrap_or(&now).to_owned();
        Ok(
            json!({"meta":{"source_provider":self.paths.id,"source_id":cid,"title":title,"directory":directory,"created_at":created,"updated_at":updated,"raw_size":raw_size},"entries":entries}),
        )
    }
}

impl Provider for AntigravityProvider {
    fn paths(&self) -> &ProviderPaths {
        &self.paths
    }
    fn probe_lock(&self, cid: Option<&str>) -> Result<Option<String>> {
        if let Some(cid) = cid {
            safe_id(cid)?;
            if !self.is_live_unlocked(cid) {
                if let Err(error) = self.presence_guard(cid) {
                    return Ok(Some(format!(
                        "Файл диалога {} занят ({error}). Закройте {} и повторите.",
                        self.paths.name, self.paths.name
                    )));
                }
            }
            if let Err(error) = db_probe(&self.db(cid)) {
                return Ok(Some(format!(
                    "Файл диалога {} занят ({error}). Закройте {} и повторите.",
                    self.paths.name, self.paths.name
                )));
            }
        }
        if let Err(error) = db_probe(&self.summaries()) {
            return Ok(Some(format!(
                "База {} занята ({error}). Закройте {} и повторите.",
                self.paths.name, self.paths.name
            )));
        }
        Ok(None)
    }
    fn lock_status(&self, cid: Option<&str>) -> Result<Value> {
        let reason = self.probe_lock(cid)?;
        let live_unlocked = cid.is_some_and(|c| self.is_live_unlocked(c));
        let presence_locked = cid.is_some_and(|c| self.is_presence_locked(c));
        Ok(json!({
            "locked": reason.is_some(),
            "reason": reason.unwrap_or_default(),
            "live_unlocked": live_unlocked,
            "presence_locked": presence_locked,
            "can_unlock_db": true,
            "can_restart_hub": true,
        }))
    }
    fn call(&self, op: &str, d: &Value, ctx: &OpContext) -> Result<Value> {
        match op {
            "state" => Ok(self.state()),
            "convs" => {
                if d["refresh"].as_bool().unwrap_or(false) {
                    self.invalidate(None);
                }
                self.convs()
            }
            "backups" => self.backups(d["cid"].as_str().filter(|s| !s.is_empty())),
            "backup_entries" => self.backup_entries(Path::new(required(d, "backup_dir")?), d),
            "restore" => {
                let path = Path::new(required(d, "backup_dir")?);
                ctx.phase("restore");
                self.restore(path)?;
                ctx.phase("done");
                Ok(json!({"restored":path,"provider":self.paths.id}))
            }
            "restart_hub" => {
                let cid = d["cid"].as_str().filter(|s| !s.is_empty());
                self.restart_hub(cid)
            }
            "entries" | "edit_preview" | "apply_edit" | "preview_cut" | "apply_cut"
            | "delete_conv" | "export_chat" | "unlock_db" | "relock_db" => {
                let cid = required(d, "cid")?;
                safe_id(cid)?;
                match op {
                    "entries" => self.entries(cid, d),
                    "edit_preview" => self.edit_preview(cid, key_idx(d)?),
                    "apply_edit" => self.apply_edit(cid, key_idx(d)?, txt(d, "text"), ctx),
                    "preview_cut" => {
                        self.preview_cut(cid, key_idx(d)?, d["mode"].as_str().unwrap_or("tail"))
                    }
                    "apply_cut" => {
                        self.apply_cut(cid, key_idx(d)?, d["mode"].as_str().unwrap_or("tail"), ctx)
                    }
                    "delete_conv" => self.delete_conv(cid, ctx),
                    "export_chat" => {
                        self.export_chat(cid, d["include_tools"].as_bool().unwrap_or(true))
                    }
                    "unlock_db" => self.unlock_db(cid),
                    "relock_db" => self.relock_db(cid),
                    _ => unreachable!(),
                }
            }
            "delete_single" => {
                bail!("Удаление одиночной записи не поддерживается этим провайдером")
            }
            _ => bail!("Неизвестная операция {op} для {}", self.paths.name),
        }
    }
}

pub fn payload_summary(stype: i64, payload: &[u8]) -> Value {
    let mut text = String::new();
    let mut tool = String::new();
    let mut args = String::new();
    match stype {
        14 => {
            if let Some(part) = bytes_at(payload, &[19, 3]) {
                let texts: Vec<_> = parse_fields(&part)
                    .iter()
                    .filter(|f| f.wire_type == 2)
                    .filter_map(|f| f.bytes())
                    .filter_map(|v| string_at(v, &[1]))
                    .filter(|s| !s.is_empty() && !s.starts_with('\n'))
                    .collect();
                text = texts.join("\n");
            }
        }
        15 => text = string_at(payload, &[20, 1]).unwrap_or_default(),
        132 => {
            tool = string_at(payload, &[5, 4, 2]).unwrap_or_default();
            args = string_at(payload, &[5, 4, 3]).unwrap_or_default();
        }
        _ => {}
    }
    json!({"text":text,"tool":tool,"args":args})
}
fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    fn hex(c: u8) -> Option<u8> {
        match c {
            b'0'..=b'9' => Some(c - b'0'),
            b'a'..=b'f' => Some(c - b'a' + 10),
            b'A'..=b'F' => Some(c - b'A' + 10),
            _ => None,
        }
    }
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            if let (Some(a), Some(c)) = (hex(b[i + 1]), hex(b[i + 2])) {
                out.push(a * 16 + c);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}
fn python_json(v: &Value) -> String {
    match v {
        Value::Array(a) => format!(
            "[{}]",
            a.iter().map(python_json).collect::<Vec<_>>().join(", ")
        ),
        Value::Object(o) => format!(
            "{{{}}}",
            o.iter()
                .map(|(k, v)| format!(
                    "{}: {}",
                    serde_json::to_string(k).unwrap_or_default(),
                    python_json(v)
                ))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        _ => v.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pbwire::encode_varint;
    const CID: &str = "12345678-abcd-4567-8123-123456789abc";
    fn nested(path: &[u32], leaf: &[u8]) -> Vec<u8> {
        path.iter()
            .rev()
            .fold(leaf.to_vec(), |b, n| encode_field(*n, 2, &b))
    }
    fn fixture() -> (tempfile::TempDir, AntigravityProvider) {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("antigravity sandbox");
        fs::create_dir_all(root.join("conversations")).unwrap();
        let p = AntigravityProvider::new(ProviderPaths {
            id: "antigravity".into(),
            name: "Antigravity".into(),
            short: "AG".into(),
            root: root.clone(),
            backup_root: tmp.path().join("backups"),
        });
        let con = Connection::open(p.db(CID)).unwrap();
        con.execute_batch("CREATE TABLE steps(idx INTEGER PRIMARY KEY,step_type INTEGER,metadata BLOB,step_payload BLOB); CREATE TABLE gen_metadata(idx INTEGER PRIMARY KEY,data BLOB); CREATE TABLE opaque_extension(idx INTEGER PRIMARY KEY,data BLOB); INSERT INTO opaque_extension VALUES(7,x'ff00deadbeef');").unwrap();
        let mut hist = Vec::new();
        for (idx, text) in ["first", "one", "second", "two"].iter().enumerate() {
            let user = idx % 2 == 0;
            let stype = if user { 14 } else { 15 };
            let mut payload = if user {
                let mut inner = encode_field(2, 2, text.as_bytes());
                inner.extend(nested(&[3, 1], text.as_bytes()));
                encode_field(19, 2, &inner)
            } else {
                let mut inner = encode_field(1, 2, text.as_bytes());
                inner.extend(encode_field(8, 2, text.as_bytes()));
                inner.extend(encode_field(3, 2, b"reasoning"));
                encode_field(20, 2, &inner)
            };
            payload.extend(encode_field(777, 0, &encode_varint(12345)));
            let meta = encode_field(
                1,
                2,
                &encode_field(1, 0, &encode_varint(1_785_000_000 + idx as u64)),
            );
            con.execute(
                "INSERT INTO steps VALUES(?,?,?,?)",
                params![idx as i64, stype, meta, payload],
            )
            .unwrap();
            hist.push(json!({"step_index":idx,"type":step_name(stype),"content":if user{format!("<ADDITIONAL_METADATA>keep me</ADDITIONAL_METADATA>\n<USER_REQUEST>\n{text}\n</USER_REQUEST>")}else{text.to_string()},"created_at":format!("2026-07-25T12:00:0{idx}Z"),"status":"DONE","custom":{"preserve":true},"thinking":if user{""}else{"reasoning"}}));
        }
        for (idx, last) in [(0, "1"), (1, "3")] {
            let mut pair = encode_field(1, 2, b"last_step_index");
            pair.extend(encode_field(2, 2, last.as_bytes()));
            con.execute(
                "INSERT INTO gen_metadata VALUES(?,?)",
                params![idx, nested(&[1, 20], &pair)],
            )
            .unwrap();
        }
        drop(con);
        let sum = Connection::open(p.summaries()).unwrap();
        sum.execute_batch("CREATE TABLE conversation_summaries(conversation_id TEXT PRIMARY KEY,title TEXT,step_count INTEGER,last_user_input_step_index INTEGER,raw_summary BLOB,workspace_uris TEXT);CREATE TABLE conversation_parents(conversation_id TEXT PRIMARY KEY,parent_id TEXT);CREATE VIRTUAL TABLE conversations_fts USING fts5(conversation_id UNINDEXED,chunk_index UNINDEXED,chunk_first_step UNINDEXED,chunk_last_step UNINDEXED,user_step_byte_offsets UNINDEXED,agent_step_byte_offsets UNINDEXED,user_body,agent_body);").unwrap();
        let mut raw = encode_field(2, 0, &encode_varint(4));
        raw.extend(encode_field(16, 0, &encode_varint(2)));
        raw.extend(encode_field(999, 2, b"unknown summary"));
        sum.execute(
            "INSERT INTO conversation_summaries VALUES(?,?,4,2,?,?)",
            params![
                CID,
                "Проверка миграции",
                raw,
                "[\"file:///tmp/Project%20Folder\"]"
            ],
        )
        .unwrap();
        sum.execute(
            "INSERT INTO conversations_fts VALUES(?,0,0,3,?, ?, ?, ?)",
            params![
                CID,
                "[[0,0],[2,6]]",
                "[[1,0],[3,3]]",
                b"first\nsecond\n".to_vec(),
                b"onetwo".to_vec()
            ],
        )
        .unwrap();
        drop(sum);
        for name in ["transcript_full.jsonl", "transcript.jsonl"] {
            p.write_transcript(
                CID,
                name,
                &hist.iter().map(Value::to_string).collect::<Vec<_>>(),
            )
            .unwrap();
        }
        let b = p.brain(CID);
        fs::create_dir_all(b.join("messages")).unwrap();
        for idx in 0..4 {
            atomic(&b.join("messages").join(format!("m{idx}.json")),&serde_json::to_vec(&json!({"id":format!("m{idx}"),"sourceMetadata":{"tool":{"stepIndex":idx}},"content":format!("message {idx}")})).unwrap()).unwrap();
            atomic(
                &b.join("steps").join(idx.to_string()).join("output.txt"),
                b"output",
            )
            .unwrap();
            atomic(&b.join("tasks").join(format!("task-{idx}.log")), b"log").unwrap();
        }
        atomic(
            &b.join("messages/read.json"),
            b"{\"m0\":true,\"m1\":true,\"m2\":true,\"m3\":true}",
        )
        .unwrap();
        atomic(&p.annotation(CID), b"title: \"test\"\n").unwrap();
        (tmp, p)
    }
    #[test]
    fn edit_updates_both_protobuf_fields_transcripts_chunks_and_fts_without_touching_unknowns() {
        let (_tmp, p) = fixture();
        let old = p.step(CID, 1).unwrap().1;
        let rep = p
            .call(
                "apply_edit",
                &json!({"cid":CID,"key":"1","text":"Новый ответ 🐈","confirm":true}),
                &OpContext::default(),
            )
            .unwrap();
        assert_eq!(rep["integrity_check"], "ok");
        assert_eq!(rep["fts_updated"], true);
        let new = p.step(CID, 1).unwrap().1;
        assert_eq!(string_at(&new, &[20, 1]).unwrap(), "Новый ответ 🐈");
        assert_eq!(string_at(&new, &[20, 8]).unwrap(), "Новый ответ 🐈");
        assert_eq!(string_at(&new, &[20, 3]), string_at(&old, &[20, 3]));
        assert_eq!(
            parse_fields(&new).last().unwrap().raw,
            parse_fields(&old).last().unwrap().raw
        );
        for name in ["transcript_full.jsonl", "transcript.jsonl"] {
            let path = p.brain(CID).join("logs").join(name);
            let vals = read_values(&path);
            assert_eq!(vals[1]["content"], "Новый ответ 🐈");
            assert_eq!(vals[1]["custom"]["preserve"], true);
            let bytes = fs::read(path).unwrap();
            let chunks = p
                .brain(CID)
                .join("logs/chunks")
                .join(name.trim_end_matches(".jsonl"));
            let joined: Vec<u8> = file_list(&chunks)
                .into_iter()
                .flat_map(|p| fs::read(p).unwrap())
                .collect();
            assert_eq!(bytes, joined);
        }
        let con = ro(&p.summaries()).unwrap();
        let (body, offs): (Vec<u8>, String) = con
            .query_row(
                "SELECT agent_body,agent_step_byte_offsets FROM conversations_fts",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(body, "Новый ответ 🐈two".as_bytes());
        assert_eq!(
            serde_json::from_str::<Value>(&offs).unwrap()[1][1],
            json!("Новый ответ 🐈".len())
        );
        drop(con);
        let backup = Path::new(rep["backup"].as_str().unwrap());
        p.call(
            "restore",
            &json!({"backup_dir":backup}),
            &OpContext::default(),
        )
        .unwrap();
        assert_eq!(p.step(CID, 1).unwrap().1, old);
    }
    #[test]
    fn full_user_edit_preserves_transcript_metadata_and_fts_newlines() {
        let (_tmp, p) = fixture();
        let text="<ADDITIONAL_METADATA>new metadata</ADDITIONAL_METADATA>\n<USER_REQUEST>\nновый запрос\n</USER_REQUEST>";
        p.call(
            "apply_edit",
            &json!({"cid":CID,"key":"0","text":text}),
            &OpContext::default(),
        )
        .unwrap();
        let payload = p.step(CID, 0).unwrap().1;
        assert_eq!(
            string_at(&payload, &[19, 2]).as_deref(),
            Some("новый запрос")
        );
        assert_eq!(
            string_at(&payload, &[19, 3, 1]).as_deref(),
            Some("новый запрос")
        );
        assert_eq!(p.read_history(CID)[0]["content"], text);
        let con = ro(&p.summaries()).unwrap();
        let body: Vec<u8> = con
            .query_row("SELECT user_body FROM conversations_fts", [], |r| r.get(0))
            .unwrap();
        assert_eq!(body, "новый запрос\nsecond\n".as_bytes());
        assert!(p
            .call(
                "apply_edit",
                &json!({"cid":CID,"key":"0","text":"lost wrapper"}),
                &OpContext::default()
            )
            .is_err());
    }
    #[test]
    fn cuts_remove_all_related_state_but_keep_referenced_output_and_restore() {
        let (_tmp, p) = fixture();
        let mut history = p.read_history(CID);
        history[0]["content"] = json!(format!(
            "{} see steps/3/output.txt and task-3.log",
            txt(&history[0], "content")
        ));
        for name in ["transcript_full.jsonl", "transcript.jsonl"] {
            p.write_transcript(
                CID,
                name,
                &history.iter().map(Value::to_string).collect::<Vec<_>>(),
            )
            .unwrap();
        }
        let preview = p
            .call(
                "preview_cut",
                &json!({"cid":CID,"key":"2","mode":"tail"}),
                &OpContext::default(),
            )
            .unwrap();
        assert_eq!(preview["report"]["raw_count"], 2);
        let rep = p
            .call(
                "apply_cut",
                &json!({"cid":CID,"key":"2","mode":"tail"}),
                &OpContext::default(),
            )
            .unwrap();
        assert_eq!(rep["db_rows_after"], 2);
        assert_eq!(rep["gen_metadata_removed"], 1);
        assert_eq!(rep["messages_removed"], 2);
        assert_eq!(rep["steps_outputs_removed"], 1);
        assert!(p.brain(CID).join("steps/3/output.txt").is_file());
        assert!(p.brain(CID).join("tasks/task-3.log").is_file());
        assert!(!p.brain(CID).join("steps/2").exists());
        assert!(read_value(&p.brain(CID).join("messages/read.json"))
            .get("m2")
            .is_none());
        let con = ro(&p.summaries()).unwrap();
        let body: Vec<u8> = con
            .query_row("SELECT user_body FROM conversations_fts", [], |r| r.get(0))
            .unwrap();
        assert_eq!(body, b"first\n");
        drop(con);
        p.call(
            "restore",
            &json!({"backup_dir":rep["backup"]}),
            &OpContext::default(),
        )
        .unwrap();
        assert_eq!(p.read_history(CID).len(), 4);
        assert!(p.brain(CID).join("steps/2/output.txt").is_file());
        assert_eq!(p.db_types(CID).len(), 4);
    }
    #[test]
    fn head_cut_keeps_original_indices_and_utf8_fts_offsets() {
        let (_tmp, p) = fixture();
        let rep = p
            .call(
                "apply_cut",
                &json!({"cid":CID,"key":"2","mode":"head"}),
                &OpContext {
                    auto_backup: false,
                    ..Default::default()
                },
            )
            .unwrap();
        assert_eq!(rep["db_rows_after"], 2);
        assert_eq!(rep["backup"], Value::Null);
        assert_eq!(
            p.db_types(CID).keys().copied().collect::<Vec<_>>(),
            vec![2, 3]
        );
        let con = ro(&p.summaries()).unwrap();
        let (body, offs): (Vec<u8>, String) = con
            .query_row(
                "SELECT user_body,user_step_byte_offsets FROM conversations_fts",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(body, b"second\n");
        assert_eq!(
            serde_json::from_str::<Value>(&offs).unwrap(),
            json!([[2, 0]])
        );
    }
    #[test]
    fn backups_are_inspectable_after_delete_and_paths_cannot_escape() {
        let (_tmp, p) = fixture();
        let rep = p
            .call("delete_conv", &json!({"cid":CID}), &OpContext::default())
            .unwrap();
        assert!(!p.db(CID).exists());
        let list = p
            .call("backups", &json!({}), &OpContext::default())
            .unwrap();
        assert_eq!(list["total_count"], 1);
        let view = p
            .call(
                "backup_entries",
                &json!({"backup_dir":rep["backup"],"limit":2}),
                &OpContext::default(),
            )
            .unwrap();
        assert_eq!(view["readonly"], true);
        assert_eq!(view["page_msgs"], 2);
        assert_eq!(view["has_more"], true);
        assert_eq!(view["turns"][0]["items"][0]["actions"], json!([]));
        assert!(p
            .call(
                "backup_entries",
                &json!({"backup_dir":p.paths.root}),
                &OpContext::default()
            )
            .is_err());
        assert!(p
            .call(
                "entries",
                &json!({"cid":"../outside"}),
                &OpContext::default()
            )
            .is_err());
        p.call(
            "restore",
            &json!({"backup_dir":rep["backup"]}),
            &OpContext::default(),
        )
        .unwrap();
        assert_eq!(p.db_types(CID).len(), 4);
    }
    #[test]
    fn export_matches_normalized_contract_and_db_fallback() {
        let (_tmp, p) = fixture();
        let doc = p
            .call(
                "export_chat",
                &json!({"cid":CID,"include_tools":true}),
                &OpContext::default(),
            )
            .unwrap();
        assert_eq!(doc["meta"]["title"], "Проверка миграции");
        assert_eq!(doc["meta"]["directory"], "/tmp/Project Folder");
        assert_eq!(doc["entries"][0]["text"], "first");
        assert_eq!(doc["entries"].as_array().unwrap().len(), 6);
        assert_eq!(doc["entries"][1]["subtype"], "thinking");
        let simple = p
            .call(
                "export_chat",
                &json!({"cid":CID,"include_tools":false}),
                &OpContext::default(),
            )
            .unwrap();
        assert_eq!(simple["entries"].as_array().unwrap().len(), 4);
        fs::remove_dir_all(p.brain(CID).join("logs")).unwrap();
        let fallback = p
            .call(
                "export_chat",
                &json!({"cid":CID,"include_tools":true}),
                &OpContext::default(),
            )
            .unwrap();
        assert_eq!(fallback["entries"].as_array().unwrap().len(), 6);
        assert_eq!(fallback["entries"][0]["text"], "first");
    }
    #[test]
    fn sqlite_lock_is_reported_without_creating_missing_databases() {
        let (_tmp, p) = fixture();
        assert!(p.probe_lock(Some("missing")).unwrap().is_none());
        assert!(!p.db("missing").exists());
        let con = rw(&p.db(CID)).unwrap();
        con.execute_batch("BEGIN IMMEDIATE").unwrap();
        assert!(p.probe_lock(Some(CID)).unwrap().is_some());
        assert!(p
            .call(
                "apply_edit",
                &json!({"cid":CID,"key":"1","text":"blocked"}),
                &OpContext::default()
            )
            .is_err());
        con.execute_batch("ROLLBACK").unwrap();
    }
    #[test]
    fn tool_call_planner_steps_render_as_tools_without_empty_cards() {
        let (_tmp, p) = fixture();
        let mut hist = p.read_history(CID);
        hist.push(json!({
            "step_index": 4,
            "type": "PLANNER_RESPONSE",
            "status": "DONE",
            "created_at": "2026-07-25T12:00:04Z",
            "thinking": "Need to inspect directory",
            "tool_calls": [{
                "name": "run_command",
                "args": {
                    "CommandLine": "Get-ChildItem",
                    "toolSummary": "List files"
                }
            }]
        }));
        hist.push(json!({
            "step_index": 5,
            "type": "GENERIC",
            "status": "DONE",
            "created_at": "2026-07-25T12:00:05Z",
            "content": "file1.txt\nfile2.txt"
        }));
        hist.push(json!({
            "step_index": 6,
            "type": "PLANNER_RESPONSE",
            "status": "DONE",
            "created_at": "2026-07-25T12:00:06Z",
            "thinking": "Only thinking step"
        }));
        for name in ["transcript_full.jsonl", "transcript.jsonl"] {
            p.write_transcript(
                CID,
                name,
                &hist.iter().map(Value::to_string).collect::<Vec<_>>(),
            )
            .unwrap();
        }
        p.invalidate(Some(CID));

        let clean = p
            .call(
                "entries",
                &json!({"cid": CID, "include_tools": false}),
                &OpContext::default(),
            )
            .unwrap();
        let clean_items = clean["turns"][0]["items"].as_array().unwrap();
        assert_eq!(clean_items.len(), 4);
        assert!(clean_items
            .iter()
            .all(|it| !txt(it, "text").trim().is_empty()));

        let full = p
            .call(
                "entries",
                &json!({"cid": CID, "include_tools": true}),
                &OpContext::default(),
            )
            .unwrap();
        let full_items = full["turns"][0]["items"].as_array().unwrap();
        assert_eq!(full_items.len(), 7);
        let call_it = &full_items[4];
        assert_eq!(call_it["kind"], "tool");
        assert_eq!(call_it["subtype"], "tool_use");
        assert_eq!(call_it["label"], "Инструмент: run_command");
        assert_eq!(call_it["tool"], "run_command");
        assert!(txt(call_it, "text").contains("List files"));
        assert!(txt(call_it, "text").contains("Get-ChildItem"));
        assert!(txt(call_it, "text").contains("Need to inspect directory"));
        assert!(!call_it["actions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|a| a == "edit"));

        let res_it = &full_items[5];
        assert_eq!(res_it["kind"], "tool");
        assert_eq!(res_it["subtype"], "tool_result");
        assert_eq!(res_it["label"], "Вывод инструмента");
        assert_eq!(res_it["text"], "file1.txt\nfile2.txt");

        let think_it = &full_items[6];
        assert_eq!(think_it["kind"], "tool");
        assert_eq!(think_it["subtype"], "thinking");
        assert_eq!(think_it["label"], "Размышления");
        assert_eq!(think_it["text"], "Only thinking step");
    }

    #[test]
    fn live_unlock_allows_edit_and_wal_safe_restore_while_db_is_open() {
        let (_tmp, p) = fixture();
        // Switch DBs to WAL mode and keep concurrent open reader handles like language_server does.
        let db_con = rw(&p.db(CID)).unwrap();
        db_con.execute_batch("PRAGMA journal_mode=WAL;").unwrap();
        let sum_con = rw(&p.summaries()).unwrap();
        sum_con.execute_batch("PRAGMA journal_mode=WAL;").unwrap();

        let live_reader = ro(&p.db(CID)).unwrap();
        let live_sum_reader = ro(&p.summaries()).unwrap();
        let initial_rows: i64 = live_reader
            .query_row("SELECT count(*) FROM steps", [], |r| r.get(0))
            .unwrap();
        assert_eq!(initial_rows, 4);

        #[cfg(unix)]
        let (db_ino_before, sum_ino_before, _presence_holder) = {
            use std::os::unix::fs::MetadataExt;
            let pdir = p.paths.root.join("presence");
            fs::create_dir_all(&pdir).unwrap();
            let lock_path = pdir.join(format!("{CID}.lock"));
            let holder = File::create(&lock_path).unwrap();
            holder.lock_exclusive().unwrap();
            let db_ino = fs::metadata(p.db(CID)).unwrap().ino();
            let sum_ino = fs::metadata(p.summaries()).unwrap().ino();
            (db_ino, sum_ino, holder)
        };

        #[cfg(unix)]
        {
            assert!(p.probe_lock(Some(CID)).unwrap().is_some());
            assert!(p
                .apply_edit(CID, 1, "should fail while locked", &OpContext::default())
                .is_err());
        }

        let unlock_res = p.unlock_db(CID).unwrap();
        assert_eq!(unlock_res["live_unlocked"], true);
        assert_eq!(unlock_res["locked"], false);
        assert!(p.probe_lock(Some(CID)).unwrap().is_none());

        let rep = p
            .apply_edit(CID, 1, "edited live in WAL", &OpContext::default())
            .unwrap();
        let backup_dir = rep["backup"].as_str().unwrap().to_owned();
        #[cfg(unix)]
        assert!(rep["warns"]
            .as_array()
            .unwrap()
            .iter()
            .any(|w| w.as_str().unwrap_or("").contains("Live Unlock")));

        // Existing open reader immediately sees the WAL edit without corruption.
        let edited_payload: Vec<u8> = live_reader
            .query_row("SELECT step_payload FROM steps WHERE idx=1", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(
            string_at(&edited_payload, &[20, 1]).as_deref(),
            Some("edited live in WAL")
        );

        // Restore in-place while reader handles and presence lock remain active.
        p.restore(Path::new(&backup_dir)).unwrap();
        let restored_payload: Vec<u8> = live_reader
            .query_row("SELECT step_payload FROM steps WHERE idx=1", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(
            string_at(&restored_payload, &[20, 1]).as_deref(),
            Some("one")
        );
        let sum_check: String = live_sum_reader
            .query_row("PRAGMA quick_check", [], |r| r.get(0))
            .unwrap();
        assert_eq!(sum_check, "ok");

        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            assert_eq!(fs::metadata(p.db(CID)).unwrap().ino(), db_ino_before);
            assert_eq!(fs::metadata(p.summaries()).unwrap().ino(), sum_ino_before);

            let relock = p.relock_db(CID).unwrap();
            assert_eq!(relock["live_unlocked"], false);
            assert_eq!(relock["locked"], true);
            assert!(p.probe_lock(Some(CID)).unwrap().is_some());
        }
    }
}
