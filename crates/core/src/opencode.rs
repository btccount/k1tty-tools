use crate::{
    jsonl::json_serialized_len,
    process,
    provider::{
        required, truncate_preview_item, valid_id, EntryRef, OpContext, Provider, ProviderPaths,
    },
    storage,
};
use anyhow::{bail, Context, Result};
use chrono::{Local, TimeZone, Utc};
use rusqlite::{
    params,
    types::{Value as SqlValue, ValueRef},
    Connection, OpenFlags, OptionalExtension, TransactionBehavior,
};
use serde_json::{json, Map, Value};
use std::{
    collections::{BTreeMap, HashMap},
    fs,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
    time::{Duration, UNIX_EPOCH},
};

const BACKUP_TABLES: [(&str, &str); 9] = [
    ("session_v2", "id"),
    ("session_message", "session_id"),
    ("session_part", "session_id"),
    ("session_inbox", "session_id"),
    ("instruction_state", "session_id"),
    ("instruction_entry", "session_id"),
    ("event_sequence", "aggregate_id"),
    ("blob_ref", "session_id"),
    ("schedule", "session_id"),
];

fn s(v: &Value) -> &str {
    v.as_str().unwrap_or("")
}
fn text(v: &Value) -> String {
    match v {
        Value::String(x) => x.clone(),
        Value::Null => String::new(),
        _ => v.to_string(),
    }
}
fn truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|n| n != 0.0),
        Value::String(s) => !s.is_empty(),
        Value::Array(a) => !a.is_empty(),
        Value::Object(o) => !o.is_empty(),
    }
}
fn chars(value: &str, n: usize) -> String {
    value.chars().take(n).collect()
}
fn iso(ms: i64) -> String {
    Utc.timestamp_millis_opt(ms)
        .single()
        .filter(|_| ms != 0)
        .map(|d| {
            d.to_rfc3339_opts(
                if ms % 1000 == 0 {
                    chrono::SecondsFormat::Secs
                } else {
                    chrono::SecondsFormat::Micros
                },
                false,
            )
        })
        .unwrap_or_default()
}

struct CachedSessionItems {
    sig: (i64, i64),
    items: Arc<Vec<Value>>,
    last_used: u64,
}

pub struct OpenCodeProvider {
    paths: ProviderPaths,
    items_cache: Mutex<HashMap<(String, bool), CachedSessionItems>>,
    cache_tick: AtomicU64,
}
#[derive(Clone, Debug)]
struct Message {
    id: String,
    kind: String,
    seq: i64,
    created: i64,
    data: Value,
}

impl OpenCodeProvider {
    pub fn new(paths: ProviderPaths) -> Self {
        Self {
            paths,
            items_cache: Mutex::new(HashMap::new()),
            cache_tick: AtomicU64::new(0),
        }
    }
    fn invalidate(&self, sid: Option<&str>) {
        let mut cache = self.items_cache.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(sid) = sid {
            cache.remove(&(sid.to_owned(), false));
            cache.remove(&(sid.to_owned(), true));
        } else {
            cache.clear();
        }
    }
    fn db(&self) -> PathBuf {
        self.paths.root.join("opencode.db")
    }
    fn ro(&self) -> Result<Connection> {
        let c = Connection::open_with_flags(self.db(), OpenFlags::SQLITE_OPEN_READ_ONLY)?;
        c.busy_timeout(Duration::from_secs(10))?;
        Ok(c)
    }
    fn rw(&self) -> Result<Connection> {
        if !self.db().is_file() {
            bail!("База данных OpenCode не найдена: {}", self.db().display());
        }
        let c = Connection::open_with_flags(self.db(), OpenFlags::SQLITE_OPEN_READ_WRITE)?;
        c.busy_timeout(Duration::from_secs(10))?;
        c.execute_batch("PRAGMA foreign_keys=ON")?;
        Ok(c)
    }
    fn gate(&self) -> Result<Vec<String>> {
        self.assert_writable(None)?;
        Ok(
            if process::snapshot()
                .get("opencode")
                .is_some_and(|p| !p.is_empty())
            {
                vec!["OpenCode запущен, но база доступна в WAL-режиме. Переоткройте диалог в OpenCode после правки.".into()]
            } else {
                vec![]
            },
        )
    }
    fn sessions(&self) -> Result<Vec<Value>> {
        if !self.db().is_file() {
            return Ok(vec![]);
        }
        let c = self.ro()?;
        let mut stat=c.prepare("SELECT session_id,COUNT(*),COALESCE(SUM(OCTET_LENGTH(data)),0) FROM session_message GROUP BY session_id")?;
        let stats = stat
            .query_map([], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    (r.get::<_, i64>(1)?, r.get::<_, i64>(2)?),
                ))
            })?
            .collect::<rusqlite::Result<BTreeMap<_, _>>>()?;
        let mut query=c.prepare("SELECT id,parent_id,slug,directory,title,time_created,time_updated,time_archived FROM session_v2 ORDER BY time_updated DESC")?;
        let records = query
            .query_map([], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, Option<String>>(1)?,
                    r.get::<_, Option<String>>(2)?,
                    r.get::<_, Option<String>>(3)?,
                    r.get::<_, Option<String>>(4)?,
                    r.get::<_, Option<i64>>(5)?,
                    r.get::<_, Option<i64>>(6)?,
                    r.get::<_, Option<i64>>(7)?,
                ))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        let mut out = vec![];
        for (id, parent, slug, directory, title, created, updated, archived) in records {
            let slug = slug.unwrap_or_default();
            let directory = directory.unwrap_or_default();
            let mut title = title.unwrap_or_default().trim().to_owned();
            if title.is_empty() {
                let raw:Option<String>=c.query_row("SELECT data FROM session_message WHERE session_id=? AND type='user' ORDER BY seq ASC LIMIT 1",[&id],|r|r.get(0)).optional()?;
                if let Some(v) = raw.and_then(|v| serde_json::from_str::<Value>(&v).ok()) {
                    let value = if !s(&v["text"]).is_empty() {
                        s(&v["text"])
                    } else {
                        s(&v["metadata"]["displayText"])
                    };
                    title = chars(&value.split_whitespace().collect::<Vec<_>>().join(" "), 72);
                }
                if title.is_empty() {
                    title = if !slug.is_empty() {
                        slug.clone()
                    } else {
                        format!("Диалог {}", chars(&id, 10))
                    };
                }
            }
            let ts = updated.filter(|t| *t != 0).or(created).unwrap_or(0);
            let (count, size) = stats.get(&id).cloned().unwrap_or((0, 0));
            out.push(json!({"id":id,"parent_id":parent,"slug":slug,"directory":directory,"title":title,"msg_count":count,"size":size,"mtime_ts":ts as f64/1000.0,"mtime":if ts!=0{Local.timestamp_millis_opt(ts).single().map(|d|d.format("%d.%m.%Y %H:%M").to_string()).unwrap_or_default()}else{String::new()},"archived":archived.unwrap_or(0)!=0}));
        }
        Ok(out)
    }
    fn messages(&self, sid: &str) -> Result<Vec<Message>> {
        if !self.db().is_file() {
            return Ok(vec![]);
        }
        let c = self.ro()?;
        let mut q=c.prepare("SELECT id,type,seq,time_created,data FROM session_message WHERE session_id=? ORDER BY seq,time_created,id")?;
        let rows = q
            .query_map([sid], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, i64>(2)?,
                    r.get::<_, i64>(3)?,
                    r.get::<_, String>(4)?,
                ))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows
            .into_iter()
            .filter_map(|(id, kind, seq, created, raw)| {
                serde_json::from_str::<Value>(&raw)
                    .ok()
                    .filter(Value::is_object)
                    .map(|data| Message {
                        id,
                        kind,
                        seq,
                        created,
                        data,
                    })
            })
            .collect())
    }
    fn items(&self, sid: &str) -> Result<Vec<Value>> {
        Ok(items_from_messages(&self.messages(sid)?, false, true))
    }
    fn cached_preview_items(&self, sid: &str, include_tools: bool) -> Result<Arc<Vec<Value>>> {
        if !self.db().is_file() {
            return Ok(Arc::new(Vec::new()));
        }
        let c = self.ro()?;
        let sig: (i64, i64) = c.query_row(
            "SELECT COALESCE(MAX(time_updated),0),COUNT(*) FROM session_message WHERE session_id=?",
            [sid],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;
        let cache_key = (sid.to_owned(), include_tools);
        {
            let mut cache = self.items_cache.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(entry) = cache.get_mut(&cache_key).filter(|e| e.sig == sig) {
                entry.last_used = self.cache_tick.fetch_add(1, Ordering::Relaxed) + 1;
                return Ok(Arc::clone(&entry.items));
            }
        }
        let mut q = c.prepare("SELECT id,type,seq,time_created,data FROM session_message WHERE session_id=? ORDER BY seq,time_created,id")?;
        let mut rows = q.query([sid])?;
        let mut out = Vec::new();
        while let Some(r) = rows.next()? {
            let id: String = r.get(0)?;
            let kind: String = r.get(1)?;
            let seq: i64 = r.get(2)?;
            let created: i64 = r.get(3)?;
            let raw: String = r.get(4)?;
            let Ok(data) = serde_json::from_str::<Value>(&raw) else {
                continue;
            };
            if !data.is_object() {
                continue;
            }
            push_message_items(
                &id,
                &kind,
                seq,
                created,
                &data,
                false,
                include_tools,
                true,
                &mut out,
            );
        }
        let items = Arc::new(out);
        let tick = self.cache_tick.fetch_add(1, Ordering::Relaxed) + 1;
        let mut cache = self.items_cache.lock().unwrap_or_else(|e| e.into_inner());
        while cache.len() >= 4 {
            if let Some(old_key) = cache
                .iter()
                .min_by_key(|(_, v)| v.last_used)
                .map(|(k, _)| k.clone())
            {
                cache.remove(&old_key);
            } else {
                break;
            }
        }
        cache.insert(
            cache_key,
            CachedSessionItems {
                sig,
                items: Arc::clone(&items),
                last_used: tick,
            },
        );
        Ok(items)
    }
    fn load(c: &Connection, sid: &str, mid: &str) -> Result<Message> {
        let(id,kind,seq,created,raw):(String,String,i64,i64,String)=c.query_row("SELECT id,type,seq,time_created,data FROM session_message WHERE id=? AND session_id=?",params![mid,sid],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?))).with_context(||format!("Сообщение {mid} не найдено в сессии {sid}"))?;
        let data: Value = serde_json::from_str(&raw)?;
        if !data.is_object() {
            bail!("Некорректное сообщение OpenCode {mid}: ожидается JSON-объект");
        }
        Ok(Message {
            id,
            kind,
            seq,
            created,
            data,
        })
    }
    fn preview_edit(&self, sid: &str, key: &str) -> Result<Value> {
        let (k, mid, part) = parse_key(key)?;
        let m = Self::load(&self.ro()?, sid, mid)?;
        let d = &m.data;
        let (body, kind) = match k {
            "m" => match m.kind.as_str() {
                "user" => (
                    text(
                        d.get("text")
                            .filter(|v| !v.is_null())
                            .unwrap_or(&d["metadata"]["displayText"]),
                    ),
                    "Сообщение пользователя".into(),
                ),
                "compaction" => (text(&d["summary"]), "Компакция".into()),
                "system" | "synthetic" => (
                    text(&d["text"]),
                    if !s(&d["description"]).is_empty() {
                        text(&d["description"])
                    } else if m.kind == "system" {
                        "Системное сообщение".into()
                    } else {
                        "Служебное сообщение".into()
                    },
                ),
                _ => bail!("Тип сообщения {} не поддерживает редактирование", m.kind),
            },
            "e" => (
                if d["error"]["message"].is_string() {
                    text(&d["error"]["message"])
                } else {
                    text(&d["error"])
                },
                "Ошибка OpenCode".into(),
            ),
            _ => {
                let i = part.unwrap();
                let p = d["content"]
                    .get(i)
                    .filter(|p| p.is_object())
                    .context("Фрагмент не найден")?;
                match s(&p["type"]) {
                    "text" => (text(&p["text"]), "Ответ OpenCode".into()),
                    "reasoning" => (text(&p["text"]), "Размышления".into()),
                    "tool" => (
                        tool_text(p).0,
                        format!("Инструмент: {}", p["name"].as_str().unwrap_or("Инструмент")),
                    ),
                    other => bail!("Фрагмент типа {other} не поддерживает редактирование"),
                }
            }
        };
        Ok(json!({"text":body,"kind":kind,"hint":"","revision":key,"writable":true}))
    }
    fn edit(&self, sid: &str, key: &str, body: &str) -> Result<Value> {
        let (k, mid, part) = parse_key(key)?;
        let mut c = self.rw()?;
        let tx = c.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let mut m = Self::load(&tx, sid, mid)?;
        let d = &mut m.data;
        match k {
            "m" => match m.kind.as_str() {
                "user" => {
                    d["text"] = json!(body);
                    if d["metadata"].is_object() && d["metadata"].get("displayText").is_some() {
                        d["metadata"]["displayText"] = json!(body);
                    }
                }
                "compaction" => d["summary"] = json!(body),
                "system" | "synthetic" => d["text"] = json!(body),
                _ => bail!("Тип сообщения {} не поддерживает редактирование", m.kind),
            },
            "e" => {
                if d["error"].is_object() && d["error"].get("message").is_some() {
                    d["error"]["message"] = json!(body);
                } else {
                    d["error"] = json!(body);
                }
            }
            _ => {
                let i = part.unwrap();
                let p = d["content"]
                    .get_mut(i)
                    .filter(|p| p.is_object())
                    .context("Фрагмент не найден")?;
                match s(&p["type"]) {
                    "text" | "reasoning" => p["text"] = json!(body),
                    "tool" => {
                        let mode = tool_text(p).1;
                        if !p["state"].is_object() {
                            p["state"] = json!({});
                        }
                        let st = &mut p["state"];
                        match mode {
                            "state_content" => st["content"] = json!([{"type":"text","text":body}]),
                            "state_error_msg" => st["error"]["message"] = json!(body),
                            "state_error_str" => st["error"] = json!(body),
                            "state_input" => {
                                st["input"] = serde_json::from_str(body)
                                    .unwrap_or_else(|_| json!({"value":body}))
                            }
                            _ => st["output"] = json!(body),
                        }
                    }
                    other => bail!("Фрагмент типа {other} не поддерживает редактирование"),
                }
            }
        }
        let now = Utc::now().timestamp_millis();
        update_message(&tx, sid, &m, now)?;
        tx.execute(
            "UPDATE session_v2 SET time_updated=? WHERE id=?",
            params![now, sid],
        )?;
        tx.commit()?;
        wal_checkpoint(&c, false);
        self.invalidate(Some(sid));
        Ok(json!({"session_id":sid,"key":key,"seq":m.seq,"integrity_check":"ok"}))
    }
    fn delete_single(&self, sid: &str, key: &str) -> Result<Value> {
        let (k, mid, part) = parse_key(key)?;
        let mut c = self.rw()?;
        let tx = c.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let mut m = Self::load(&tx, sid, mid)?;
        let deleted = match k {
            "m" => true,
            "e" => {
                m.data
                    .as_object_mut()
                    .context("Некорректное сообщение")?
                    .remove("error");
                m.data["content"].as_array().is_none_or(Vec::is_empty)
            }
            _ => {
                let content = m.data["content"]
                    .as_array_mut()
                    .context("Нет фрагментов сообщения")?;
                let i = part.unwrap();
                if i >= content.len() {
                    bail!("Фрагмент #{i} не найден в сообщении {mid}");
                }
                content.remove(i);
                content.is_empty() && !truthy(&m.data["error"])
            }
        };
        let now = Utc::now().timestamp_millis();
        if deleted {
            tx.execute(
                "DELETE FROM session_message WHERE id=? AND session_id=?",
                params![mid, sid],
            )?;
        } else {
            update_message(&tx, sid, &m, now)?;
        }
        tx.execute(
            "UPDATE session_v2 SET time_updated=? WHERE id=?",
            params![now, sid],
        )?;
        tx.commit()?;
        wal_checkpoint(&c, false);
        self.invalidate(Some(sid));
        Ok(json!({"session_id":sid,"key":key,"seq":m.seq,"deleted_row":deleted}))
    }
    fn preview_cut(&self, sid: &str, key: &str, mode: &str) -> Result<Value> {
        let items = self.items(sid)?;
        let pos = items
            .iter()
            .position(|i| i["key"] == key)
            .context("Запись не найдена")?;
        let will = if mode == "tail" {
            &items[pos..]
        } else {
            &items[..pos]
        };
        let seq = items[pos]["seq"].as_i64().unwrap();
        let (k, _, part) = parse_key(key)?;
        let comparator = if mode == "head" {
            "<"
        } else if k == "p" && part.unwrap_or(0) > 0 {
            ">"
        } else {
            ">="
        };
        let c = self.ro()?;
        let(mut rows,bytes):(i64,i64)=c.query_row(&format!("SELECT COUNT(*),COALESCE(SUM(LENGTH(data)),0) FROM session_message WHERE session_id=? AND seq {comparator} ?"),params![sid,seq],|r|Ok((r.get(0)?,r.get(1)?)))?;
        if mode == "tail" && k == "p" && part.unwrap_or(0) > 0 {
            rows += 1;
        }
        let preview=will.iter().take(25).map(|i|json!({"key":format!("#{}",i["seq"]),"label":i["label"],"text":chars(&s(&i["text"]).replace('\n'," "),140)})).collect::<Vec<_>>();
        Ok(
            json!({"report":{"provider":"opencode","mode":mode,"steps_count":will.len(),"raw_count":rows,"raw_label":"сообщений в БД","bytes":bytes,"steps_preview":preview},"revision":format!("{mode}:{key}"),"writable":true,"hint":""}),
        )
    }
    fn cut(&self, sid: &str, key: &str, mode: &str) -> Result<Value> {
        let (k, mid, part) = parse_key(key)?;
        let mut c = self.rw()?;
        let tx = c.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let mut m = Self::load(&tx, sid, mid)?;
        let now = Utc::now().timestamp_millis();
        let comparator = if mode == "head" {
            "<"
        } else if k == "p" && part.unwrap_or(0) > 0 {
            let content = m.data["content"]
                .as_array_mut()
                .context("Нет фрагментов сообщения")?;
            content.truncate(part.unwrap());
            m.data.as_object_mut().unwrap().remove("error");
            update_message(&tx, sid, &m, now)?;
            ">"
        } else {
            ">="
        };
        let removed = tx.execute(
            &format!("DELETE FROM session_message WHERE session_id=? AND seq {comparator} ?"),
            params![sid, m.seq],
        )?;
        tx.execute(
            "UPDATE session_v2 SET time_updated=?,revert=NULL WHERE id=?",
            params![now, sid],
        )?;
        let left: i64 = tx.query_row(
            "SELECT COUNT(*) FROM session_message WHERE session_id=?",
            [sid],
            |r| r.get(0),
        )?;
        tx.commit()?;
        wal_checkpoint(&c, false);
        self.invalidate(Some(sid));
        Ok(
            json!({"session_id":sid,"mode":mode,"removed_rows":removed,"rows_after":left,"integrity_check":"ok"}),
        )
    }
    fn backup(&self, sid: &str, tag: &str) -> Result<PathBuf> {
        valid_id(sid)?;
        let c = self.ro()?;
        c.execute_batch("BEGIN")?;
        let tables = table_names(&c)?;
        let mut snap = Map::new();
        for (name, id_col) in BACKUP_TABLES {
            if !tables.iter().any(|t| t == name) {
                continue;
            }
            let cols = columns(&c, name)?;
            let mut q = c.prepare(&format!("SELECT * FROM \"{name}\" WHERE \"{id_col}\"=?"))?;
            let count = cols.len();
            let rows = q
                .query_map([sid], |r| {
                    (0..count)
                        .map(|i| r.get_ref(i).map(sql_json))
                        .collect::<rusqlite::Result<Vec<_>>>()
                })?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            snap.insert(name.into(), json!({"columns":cols,"rows":rows}));
        }
        c.execute_batch("COMMIT")?;
        if snap
            .get("session_v2")
            .and_then(|v| v["rows"].as_array())
            .is_none_or(Vec::is_empty)
        {
            bail!("Сессия OpenCode {sid} не найдена");
        }
        let title = snapshot_title_map(&snap);
        let stamp = Local::now().format("%Y%m%d-%H%M%S-%6f").to_string();
        let dir = self
            .paths
            .backup_root
            .join(sid)
            .join(format!("{stamp}-{tag}"));
        fs::create_dir_all(&dir)?;
        storage::atomic_write(
            &dir.join("session_backup.json"),
            &serde_json::to_vec(&json!({"session_id":sid,"tables":snap}))?,
        )?;
        storage::atomic_write(
            &dir.join("backup.meta.json"),
            &serde_json::to_vec_pretty(
                &json!({"session_id":sid,"title":title,"created":Local::now().to_rfc3339(),"tag":tag,"db":self.db()}),
            )?,
        )?;
        Ok(dir)
    }
    fn restore(&self, bdir: &Path) -> Result<Value> {
        let snap = storage::read_json(&bdir.join("session_backup.json"))?;
        let sid = required(&snap, "session_id")?;
        valid_id(sid)?;
        let validated = validate_snapshot(&snap, sid)?;
        let mut c = self.rw()?;
        let tables = table_names(&c)?;
        if !["session_v2", "session_message"]
            .iter()
            .all(|required| tables.iter().any(|t| t == required))
        {
            bail!("В базе OpenCode отсутствуют обязательные таблицы сессии");
        }
        let tx = c.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let cur_seq = if tables.iter().any(|s| s == "event_sequence") {
            tx.query_row(
                "SELECT seq FROM event_sequence WHERE aggregate_id=?",
                [sid],
                |r| r.get::<_, i64>(0),
            )
            .optional()?
            .unwrap_or(0)
        } else {
            0
        };
        for (name, id_col) in BACKUP_TABLES.into_iter().rev() {
            if name != "session_v2" && tables.iter().any(|t| t == name) {
                tx.execute(
                    &format!("DELETE FROM \"{name}\" WHERE \"{id_col}\"=?"),
                    [sid],
                )?;
            }
        }
        let mut counts = Map::new();
        for (name, _) in BACKUP_TABLES {
            if !tables.iter().any(|t| t == name) {
                continue;
            }
            let Some(data) = validated.get(name) else {
                continue;
            };
            let rows = &data.rows;
            let valid = columns(&tx, name)?;
            let keep = data
                .columns
                .iter()
                .enumerate()
                .filter(|(_, c)| valid.iter().any(|v| v == **c))
                .map(|(i, c)| (i, (*c).to_owned()))
                .collect::<Vec<_>>();
            if keep.is_empty() {
                continue;
            }
            let quote = |c: &str| format!("\"{}\"", c.replace('"', "\"\""));
            let mut sql = format!(
                "INSERT INTO \"{name}\" ({}) VALUES ({})",
                keep.iter()
                    .map(|(_, c)| quote(c))
                    .collect::<Vec<_>>()
                    .join(","),
                vec!["?"; keep.len()].join(",")
            );
            if name == "session_v2" {
                let updates = keep
                    .iter()
                    .filter(|(_, c)| c != "id")
                    .map(|(_, c)| format!("{}=excluded.{}", quote(c), quote(c)))
                    .collect::<Vec<_>>();
                sql.push_str(" ON CONFLICT(id) DO ");
                if updates.is_empty() {
                    sql.push_str("NOTHING");
                } else {
                    sql.push_str("UPDATE SET ");
                    sql.push_str(&updates.join(","));
                }
            }
            for row in rows {
                let mut values = keep
                    .iter()
                    .map(|(i, _)| json_sql(&row[*i]))
                    .collect::<Result<Vec<_>>>()?;
                if name == "event_sequence" {
                    if let Some(i) = keep.iter().position(|(_, c)| c == "seq") {
                        let old = match values[i] {
                            SqlValue::Integer(x) => x,
                            _ => 0,
                        };
                        values[i] = SqlValue::Integer(old.max(cur_seq));
                    }
                }
                tx.execute(&sql, rusqlite::params_from_iter(values))?;
            }
            counts.insert(name.into(), json!(rows.len()));
        }
        quick_check(&tx)?;
        tx.commit()?;
        wal_checkpoint(&c, true);
        self.invalidate(Some(sid));
        Ok(json!({"session_id":sid,"restored_tables":counts,"integrity_check":"ok"}))
    }
    fn backups(&self, cid: Option<&str>) -> Result<Value> {
        let titles = self
            .sessions()
            .unwrap_or_default()
            .into_iter()
            .map(|v| (text(&v["id"]), text(&v["title"])))
            .collect::<BTreeMap<_, _>>();
        let mut groups = Vec::new();
        let mut count = 0;
        let mut bytes = 0u64;
        let mut roots = if let Some(id) = cid {
            valid_id(id)?;
            vec![self.paths.backup_root.join(id)]
        } else {
            dirs(&self.paths.backup_root)
        };
        roots.sort();
        roots.reverse();
        for root in roots {
            let id = root
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();
            let mut backups = Vec::new();
            let mut meta_title = String::new();
            let mut entries = dirs(&root);
            entries.sort();
            entries.reverse();
            for dir in entries {
                if storage::backup_path(&self.paths.backup_root, &dir.to_string_lossy()).is_err() {
                    continue;
                }
                let meta =
                    storage::read_json(&dir.join("backup.meta.json")).unwrap_or_else(|_| json!({}));
                if meta_title.is_empty() {
                    meta_title = text(&meta["title"]);
                }
                let size = storage::dir_size(&dir);
                bytes += size;
                count += 1;
                let has = dir.join("session_backup.json").is_file();
                backups.push(json!({"provider":"opencode","cid":id,"dir":dir,"name":dir.file_name().unwrap_or_default().to_string_lossy(),"size":size,"meta":meta,"restorable":has,"inspectable":has,"reason":""}));
            }
            if !backups.is_empty() {
                let title = titles
                    .get(&id)
                    .filter(|s| !s.is_empty())
                    .cloned()
                    .unwrap_or_else(|| {
                        if meta_title.is_empty() {
                            id.clone()
                        } else {
                            meta_title
                        }
                    });
                groups
                    .push(json!({"provider":"opencode","cid":id,"title":title,"backups":backups}));
            }
        }
        Ok(
            json!({"groups":groups,"root":self.paths.backup_root,"total_count":count,"total_bytes":bytes}),
        )
    }
    fn backup_entries(&self, data: &Value) -> Result<Value> {
        let bdir = storage::backup_path(&self.paths.backup_root, required(data, "backup_dir")?)?;
        let snap = storage::read_json(&bdir.join("session_backup.json"))?;
        let sid = snap["session_id"]
            .as_str()
            .map(str::to_owned)
            .unwrap_or_else(|| {
                bdir.parent()
                    .and_then(Path::file_name)
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned()
            });
        let sm = &snap["tables"]["session_message"];
        let cols = sm["columns"].as_array().or_else(|| sm["cols"].as_array());
        let mut messages = Vec::new();
        if let (Some(cols), Some(rows)) = (cols, sm["rows"].as_array()) {
            let names = ["id", "type", "seq", "time_created", "data"];
            let indices = names
                .iter()
                .map(|n| cols.iter().position(|c| c == n))
                .collect::<Option<Vec<_>>>();
            if let Some(i) = indices {
                for row in rows {
                    if let Ok(d) = serde_json::from_str::<Value>(s(&row[i[4]])) {
                        if d.is_object() {
                            messages.push(Message {
                                id: text(&row[i[0]]),
                                kind: text(&row[i[1]]),
                                seq: row[i[2]].as_i64().unwrap_or(0),
                                created: row[i[3]].as_i64().unwrap_or(0),
                                data: d,
                            });
                        }
                    }
                }
            }
        }
        messages.sort_by(|a, b| (a.seq, a.created, &a.id).cmp(&(b.seq, b.created, &b.id)));
        let include_tools = data["include_tools"].as_bool().unwrap_or(false);
        let mut built = items_from_messages(&messages, true, include_tools);
        for item in &mut built {
            truncate_preview_item(item);
        }
        let mut result = page_slice(&built, data, &sid);
        let mut title = snapshot_title(&snap["tables"]);
        if title.is_empty() {
            title = storage::read_json(&bdir.join("backup.meta.json"))
                .ok()
                .map(|v| text(&v["title"]))
                .unwrap_or_default();
        }
        if title.is_empty() {
            title = sid;
        }
        result["backup_dir"] = json!(bdir);
        result["backup_name"] = json!(bdir.file_name().unwrap_or_default().to_string_lossy());
        result["title"] = json!(title);
        result["readonly"] = json!(true);
        result["writable"] = json!(false);
        Ok(result)
    }
    fn export(&self, sid: &str, include_tools: bool) -> Result<Value> {
        let session = self
            .sessions()?
            .into_iter()
            .find(|v| v["id"] == sid)
            .unwrap_or(Value::Null);
        let mut entries = Vec::new();
        for m in self.messages(sid)? {
            let ts = iso(m.created);
            let d = &m.data;
            match m.kind.as_str() {
                "user" => {
                    let body = text(
                        d.get("text")
                            .filter(|v| !v.is_null())
                            .unwrap_or(&d["metadata"]["displayText"]),
                    );
                    if !body.trim().is_empty() {
                        entries.push(json!({"kind":"user","subtype":"user","label":"Вы","text":body,"ts":ts,"tool":Value::Null,"status":Value::Null,"code":""}));
                    }
                }
                "assistant" => {
                    if let Some(parts) = d["content"].as_array() {
                        for p in parts {
                            if !p.is_object() {
                                continue;
                            }
                            let ptype = s(&p["type"]);
                            if !include_tools && ptype != "text" {
                                continue;
                            }
                            let pts = iso(p["time"]["created"]
                                .as_i64()
                                .filter(|t| *t != 0)
                                .unwrap_or(m.created));
                            match ptype {
                                "text" => {
                                    let body = text(&p["text"]);
                                    if body.trim().is_empty() && parts.len() > 1 {
                                        continue;
                                    }
                                    if !body.trim().is_empty() {
                                        entries.push(json!({"kind":"assistant","subtype":"text","label":"OpenCode","text":body,"ts":pts,"tool":Value::Null,"status":Value::Null,"code":""}));
                                    }
                                }
                                "reasoning" => {
                                    let body = text(&p["text"]);
                                    if !body.trim().is_empty() {
                                        entries.push(json!({"kind":"tool","subtype":"thinking","label":"Размышления","text":body,"ts":pts,"tool":"reasoning","status":"ok","code":""}));
                                    }
                                }
                                "tool" => {
                                    let name = p["name"]
                                        .as_str()
                                        .filter(|s| !s.is_empty())
                                        .unwrap_or("tool");
                                    let status = if p["state"]["status"] == "error" {
                                        "error"
                                    } else {
                                        "ok"
                                    };
                                    let has_input = p["state"]["input"]
                                        .as_object()
                                        .is_some_and(|o| !o.is_empty());
                                    let (out_body, out_mode) = tool_text(p);
                                    let has_output =
                                        out_mode != "none" && out_mode != "state_input";
                                    if has_input && has_output {
                                        let args_str = serde_json::to_string(&p["state"]["input"])
                                            .unwrap_or_else(|_| "{}".into());
                                        entries.push(json!({"kind":"tool","subtype":"tool_use","label":name,"text":args_str,"ts":pts,"tool":name,"status":"ok","code":""}));
                                        entries.push(json!({"kind":"tool","subtype":"tool_result","label":name,"text":out_body,"ts":pts,"tool":name,"status":status,"code":""}));
                                    } else {
                                        let body = tool_display(p);
                                        if body.trim().is_empty() && name.is_empty() {
                                            continue;
                                        }
                                        let subtype = if !has_input && has_output {
                                            "tool_output"
                                        } else {
                                            "tool_use"
                                        };
                                        entries.push(json!({"kind":"tool","subtype":subtype,"label":name,"text":body,"ts":pts,"tool":name,"status":status,"code":""}));
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                    if include_tools && truthy(&d["error"]) {
                        let e = &d["error"];
                        let code = if e.is_object() {
                            text(&e["type"])
                        } else {
                            String::new()
                        };
                        let body = if e.is_object() {
                            if s(&e["message"]).is_empty() {
                                e.to_string()
                            } else {
                                text(&e["message"])
                            }
                        } else {
                            text(e)
                        };
                        if !body.trim().is_empty() {
                            entries.push(json!({"kind":"error","subtype":"error","label":"Ошибка OpenCode","text":body,"ts":ts,"tool":Value::Null,"status":"error","code":code}));
                        }
                    }
                }
                "compaction" => {
                    let body = text(&d["summary"]);
                    if !body.trim().is_empty() {
                        entries.push(json!({"kind":"compact","subtype":"compaction","label":"Компакция","text":body,"ts":ts,"tool":Value::Null,"status":Value::Null,"code":""}));
                    }
                }
                "system" | "synthetic" if include_tools => {
                    let body = text(&d["text"]);
                    if !body.trim().is_empty() {
                        let label = d["description"]
                            .as_str()
                            .filter(|s| !s.is_empty())
                            .unwrap_or(if m.kind == "system" {
                                "Система"
                            } else {
                                "Служебное"
                            });
                        entries.push(json!({"kind":"system","subtype":m.kind,"label":label,"text":body,"ts":ts,"tool":Value::Null,"status":Value::Null,"code":""}));
                    }
                }
                _ => {}
            }
        }
        let mut title = text(&session["title"]);
        if title.trim().is_empty() {
            title = entries
                .iter()
                .find(|e| e["kind"] == "user")
                .map(|e| {
                    chars(
                        &s(&e["text"])
                            .split_whitespace()
                            .collect::<Vec<_>>()
                            .join(" "),
                        90,
                    )
                })
                .unwrap_or_else(|| "Imported chat".into());
        } else {
            title = chars(title.trim(), 140);
        }
        let mut directory = text(&session["directory"]);
        if directory.starts_with("\\\\?\\UNC\\") {
            directory = format!("\\\\{}", &directory[8..]);
        } else if directory.starts_with("\\\\?\\") {
            directory = directory[4..].into();
        }
        let timestamps = entries
            .iter()
            .filter_map(|e| e["ts"].as_str().filter(|s| !s.is_empty()))
            .collect::<Vec<_>>();
        let now = storage::timestamp();
        Ok(
            json!({"meta":{"source_provider":"opencode","source_id":sid,"title":title,"directory":directory,"created_at":timestamps.first().unwrap_or(&now.as_str()),"updated_at":timestamps.last().unwrap_or(&now.as_str()),"raw_size":session["size"].as_u64().unwrap_or(0)},"entries":entries}),
        )
    }
    fn preview_slim(&self, sid: &str, data: &Value) -> Result<Value> {
        let threshold = (data["tool_threshold_kb"]
            .as_u64()
            .unwrap_or(4)
            .clamp(1, 512)
            * 1024) as usize;
        let mut thinking_count = 0usize;
        let mut thinking_bytes = 0usize;
        let mut heavy_tool_count = 0usize;
        let mut heavy_tool_bytes = 0usize;
        if self.db().is_file() {
            let c = self.ro()?;
            let mut q = c.prepare(
                "SELECT data FROM session_message WHERE session_id=? ORDER BY seq,time_created,id",
            )?;
            let mut rows = q.query([sid])?;
            while let Some(r) = rows.next()? {
                let raw: String = r.get(0)?;
                let Ok(mdata) = serde_json::from_str::<Value>(&raw) else {
                    continue;
                };
                if let Some(parts) = mdata["content"].as_array() {
                    for p in parts {
                        let typ = s(&p["type"]);
                        if matches!(typ, "reasoning" | "thinking") {
                            let sz = json_serialized_len(p);
                            thinking_count += 1;
                            thinking_bytes += sz;
                        } else if typ == "tool" {
                            let sz = p["state"]["output"].as_str().map(str::len).unwrap_or(0);
                            if sz > threshold {
                                heavy_tool_count += 1;
                                heavy_tool_bytes += sz.saturating_sub(48);
                            }
                        }
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
            "can_slim": true,
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
    fn apply_slim(&self, sid: &str, data: &Value) -> Result<Value> {
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
        let mut c = self.rw()?;
        let tx = c.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let rows: Vec<(String, String)> = {
            let mut q = tx.prepare("SELECT id,data FROM session_message WHERE session_id=? ORDER BY seq,time_created,id")?;
            let list = q
                .query_map([sid], |r| {
                    Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
                })?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            list
        };
        let now = Utc::now().timestamp_millis();
        let mut thinking_removed = 0usize;
        let mut tools_trimmed = 0usize;
        let mut freed_bytes = 0usize;
        for (mid, raw) in rows {
            let Ok(mut mdata) = serde_json::from_str::<Value>(&raw) else {
                continue;
            };
            if !mdata.is_object() {
                continue;
            }
            let has_error = truthy(&mdata["error"]);
            let mut changed = false;
            let mut drop_row = false;
            if let Some(parts) = mdata.get_mut("content").and_then(Value::as_array_mut) {
                let mut next = Vec::with_capacity(parts.len());
                for mut p in parts.drain(..) {
                    let typ = s(&p["type"]).to_owned();
                    if remove_thinking && matches!(typ.as_str(), "reasoning" | "thinking") {
                        thinking_removed += 1;
                        changed = true;
                        continue;
                    }
                    if trim_heavy_tools && typ == "tool" {
                        let sz = p["state"]["output"].as_str().map(str::len).unwrap_or(0);
                        if sz > threshold {
                            if let Some(st) = p.get_mut("state").and_then(Value::as_object_mut) {
                                st.insert(
                                    "output".into(),
                                    json!(format!(
                                        "[trimmed by K1tty Tools: {:.1} KB]",
                                        sz as f64 / 1024.0
                                    )),
                                );
                                tools_trimmed += 1;
                                changed = true;
                            }
                        }
                    }
                    next.push(p);
                }
                if changed && next.is_empty() && !has_error {
                    drop_row = true;
                } else {
                    *parts = next;
                }
            }
            if !changed {
                continue;
            }
            let before = raw.len();
            if drop_row {
                freed_bytes += before;
                tx.execute(
                    "DELETE FROM session_message WHERE id=? AND session_id=?",
                    params![mid, sid],
                )?;
            } else {
                let serialized = serde_json::to_string(&mdata)?;
                freed_bytes += before.saturating_sub(serialized.len());
                tx.execute(
                    "UPDATE session_message SET data=?,time_updated=? WHERE id=? AND session_id=?",
                    params![serialized, now, mid, sid],
                )?;
            }
        }
        if thinking_removed == 0 && tools_trimmed == 0 {
            tx.rollback()?;
            return Ok(json!({
                "provider": "opencode",
                "cid": sid,
                "changed": false,
                "thinking_removed": 0,
                "tools_trimmed": 0,
                "freed_bytes": 0,
                "freed_tokens": 0
            }));
        }
        tx.execute(
            "UPDATE session_v2 SET time_updated=? WHERE id=?",
            params![now, sid],
        )?;
        tx.commit()?;
        wal_checkpoint(&c, false);
        self.invalidate(Some(sid));
        Ok(json!({
            "provider": "opencode",
            "cid": sid,
            "changed": true,
            "thinking_removed": thinking_removed,
            "tools_trimmed": tools_trimmed,
            "freed_bytes": freed_bytes,
            "freed_tokens": (freed_bytes + 3) / 4
        }))
    }
}

impl Provider for OpenCodeProvider {
    fn paths(&self) -> &ProviderPaths {
        &self.paths
    }
    fn content_signature(&self, _cid: &str) -> Option<String> {
        let db = self.db();
        let (db_len, db_mtime) = file_stat_sig(&db)?;
        let wal = PathBuf::from(format!("{}-wal", db.display()));
        let (wal_len, wal_mtime) = file_stat_sig(&wal).unwrap_or((0, 0));
        Some(format!("{db_len}:{db_mtime}:{wal_len}:{wal_mtime}"))
    }
    fn visit_entries(&self, cid: &str, f: &mut dyn FnMut(EntryRef<'_>) -> bool) -> Result<()> {
        valid_id(cid)?;
        if !self.db().is_file() {
            return Ok(());
        }
        let c = self.ro()?;
        let mut q = c.prepare(
            "SELECT id,type,time_created,data FROM session_message WHERE session_id=? ORDER BY seq,time_created,id",
        )?;
        let mut rows = q.query([cid])?;
        while let Some(r) = rows.next()? {
            let id: String = r.get(0)?;
            let kind: String = r.get(1)?;
            let created: i64 = r.get(2)?;
            let raw: String = r.get(3)?;
            let Ok(data) = serde_json::from_str::<Value>(&raw) else {
                continue;
            };
            if !data.is_object() {
                continue;
            }
            if !visit_message_entries(&id, &kind, created, &data, f) {
                break;
            }
        }
        Ok(())
    }
    fn read_entry_text(&self, cid: &str, key: &str) -> Result<Option<String>> {
        valid_id(cid)?;
        let Ok((k, mid, part)) = parse_key(key) else {
            return Ok(None);
        };
        if !self.db().is_file() {
            return Ok(None);
        }
        let m = Self::load(&self.ro()?, cid, mid)?;
        let d = &m.data;
        let out = match k {
            "m" => match m.kind.as_str() {
                "user" => Some(text(
                    d.get("text")
                        .filter(|v| !v.is_null())
                        .unwrap_or(&d["metadata"]["displayText"]),
                )),
                "compaction" => Some(text(&d["summary"])),
                "system" | "synthetic" => Some(text(&d["text"])),
                _ => None,
            },
            "e" => {
                if truthy(&d["error"]) {
                    let e = &d["error"];
                    let body = if e.is_object() {
                        if s(&e["message"]).is_empty() {
                            e.to_string()
                        } else {
                            text(&e["message"])
                        }
                    } else {
                        text(e)
                    };
                    Some(body)
                } else {
                    None
                }
            }
            _ => {
                let i = part.unwrap();
                match d["content"].get(i).filter(|p| p.is_object()) {
                    Some(p) => match s(&p["type"]) {
                        "text" | "reasoning" => Some(text(&p["text"])),
                        "tool" => Some(tool_display(p)),
                        _ => None,
                    },
                    None => None,
                }
            }
        };
        Ok(out)
    }
    fn probe_lock(&self, _cid: Option<&str>) -> Result<Option<String>> {
        if !self.db().is_file() {
            return Ok(None);
        }
        let result = (|| -> Result<()> {
            let c = self.rw()?;
            c.busy_timeout(Duration::from_millis(250))?;
            c.execute_batch("BEGIN IMMEDIATE;ROLLBACK")?;
            Ok(())
        })();
        Ok(result
            .err()
            .map(|e| format!("База данных OpenCode занята ({e}). Закройте OpenCode и повторите.")))
    }
    fn call(&self, op: &str, data: &Value, ctx: &OpContext) -> Result<Value> {
        if let Some(id) = data["cid"].as_str() {
            valid_id(id)?;
        }
        match op {
            "state" => {
                let running = process::snapshot().remove("opencode").unwrap_or_default();
                let available = self.db().is_file();
                let blocked = self.probe_lock(None)?.is_some();
                let note = if !available {
                    format!("База данных не найдена: {}", self.db().display())
                } else if blocked {
                    format!(
                        "База занята ({}). Закройте OpenCode для правки.",
                        process::format_running(&running)
                    )
                } else if !running.is_empty() {
                    format!(
                        "Запущено: {}. Переоткройте диалог в OpenCode после правки.",
                        process::format_running(&running)
                    )
                } else {
                    "Файлы свободны.".into()
                };
                Ok(
                    json!({"id":"opencode","name":"OpenCode","short":"OC","available":available,"writable":available&&!blocked,"blocked":blocked,"running":running,"test":false,"appdata":self.paths.root,"backups_root":self.paths.backup_root,"note":note}),
                )
            }
            "convs" => {
                if data["refresh"].as_bool().unwrap_or(false) {
                    self.invalidate(None);
                }
                Ok(Value::Array(self.sessions()?.into_iter().map(|v|{
                    let count=v["msg_count"].as_i64().unwrap_or(0);let dname=Path::new(s(&v["directory"])).file_name().unwrap_or_default().to_string_lossy();
                    json!({"provider":"opencode","id":v["id"],"title":v["title"],"subtitle":if s(&v["directory"]).is_empty(){&v["slug"]}else{&v["directory"]},"badge":if count>0{format!("{count} ш.")}else if !dname.is_empty(){chars(&dname,16)}else{"OC".into()},"size":v["size"],"mtime":v["mtime"],"mtime_ts":v["mtime_ts"],"writable":true})
                }).collect()))
            }
            "entries" => {
                let sid = required(data, "cid")?;
                let include_tools = data["include_tools"].as_bool().unwrap_or(false);
                let items = self.cached_preview_items(sid, include_tools)?;
                let mut result = page_slice(&items, data, sid);
                let reason = self.probe_lock(Some(sid))?;
                result["file_locked"] = json!(reason.is_some());
                result["lock_reason"] = json!(reason.unwrap_or_default());
                Ok(result)
            }
            "edit_preview" => self.preview_edit(required(data, "cid")?, required(data, "key")?),
            "preview_cut" => self.preview_cut(
                required(data, "cid")?,
                required(data, "key")?,
                data["mode"].as_str().unwrap_or("tail"),
            ),
            "preview_slim" => self.preview_slim(required(data, "cid")?, data),
            "backups" => self.backups(data["cid"].as_str().filter(|s| !s.is_empty())),
            "backup_entries" => self.backup_entries(data),
            "export_chat" => self.export(
                required(data, "cid")?,
                data["include_tools"].as_bool().unwrap_or(true),
            ),
            "restore" => {
                let dir =
                    storage::backup_path(&self.paths.backup_root, required(data, "backup_dir")?)?;
                self.gate()?;
                ctx.phase("restore");
                self.restore(&dir)?;
                ctx.phase("done");
                Ok(json!({"restored":dir,"provider":"opencode"}))
            }
            "apply_edit" | "apply_cut" | "apply_slim" | "delete_single" | "delete_conv" => {
                let sid = required(data, "cid")?;
                let warns = self.gate()?;
                let mode = data["mode"].as_str().unwrap_or("tail");
                if op == "apply_cut" && !matches!(mode, "head" | "tail") {
                    bail!("Недопустимый режим обрезки.");
                }
                let tag = match op {
                    "apply_edit" => "edit".into(),
                    "apply_cut" => format!("cut_{mode}"),
                    "apply_slim" => "slim".into(),
                    "delete_single" => "del_single".into(),
                    _ => "delete_conv".into(),
                };
                if op == "apply_edit" && s(&data["text"]).trim().is_empty() {
                    bail!("Пустой текст, сохранение отменено.");
                }
                let backup = if ctx.auto_backup {
                    ctx.phase("backup");
                    Some(self.backup(sid, &tag)?)
                } else {
                    None
                };
                ctx.phase(if op == "apply_edit" { "edit" } else { "cut" });
                let result = match op {
                    "apply_edit" => self.edit(sid, required(data, "key")?, s(&data["text"])),
                    "apply_cut" => self.cut(sid, required(data, "key")?, mode),
                    "apply_slim" => self.apply_slim(sid, data),
                    "delete_single" => self.delete_single(sid, required(data, "key")?),
                    _ => (|| -> Result<Value> {
                        let mut c = self.rw()?;
                        let tx = c.transaction_with_behavior(TransactionBehavior::Immediate)?;
                        let tables = table_names(&tx)?;
                        for (name, id_col) in BACKUP_TABLES.into_iter().rev() {
                            if name == "session_v2" || !tables.iter().any(|t| t == name) {
                                continue;
                            }
                            tx.execute(
                                &format!("DELETE FROM \"{name}\" WHERE \"{id_col}\"=?"),
                                [sid],
                            )?;
                        }
                        if tx.execute("DELETE FROM session_v2 WHERE id=?", [sid])? == 0 {
                            bail!("Сессия OpenCode {sid} не найдена");
                        }
                        tx.commit()?;
                        wal_checkpoint(&c, true);
                        self.invalidate(Some(sid));
                        Ok(json!({"deleted":sid,"provider":"opencode"}))
                    })(),
                };
                let mut result = result?;
                result["backup"] = json!(backup);
                result["warns"] = json!(warns);
                ctx.phase("done");
                Ok(result)
            }
            _ => bail!("Неизвестная операция OpenCode: {op}"),
        }
    }
}

fn dirs(path: &Path) -> Vec<PathBuf> {
    fs::read_dir(path)
        .map(|it| {
            it.flatten()
                .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
                .map(|e| e.path())
                .collect()
        })
        .unwrap_or_default()
}
fn parse_key(key: &str) -> Result<(&str, &str, Option<usize>)> {
    let p = key.split(':').collect::<Vec<_>>();
    match p.as_slice() {
        [k, mid] if matches!(*k, "m" | "e") && !mid.is_empty() => Ok((k, mid, None)),
        ["p", mid, part] if !mid.is_empty() => Ok((
            "p",
            mid,
            Some(part.parse().context("Некорректный индекс фрагмента")?),
        )),
        _ => bail!("Некорректный ключ записи OpenCode: {key}"),
    }
}
fn tool_text(p: &Value) -> (String, &'static str) {
    let st = &p["state"];
    if !st.is_object() {
        return (String::new(), "none");
    }
    if let Some(parts) = st["content"].as_array() {
        let chunks = parts
            .iter()
            .filter_map(|c| {
                if c.is_string() {
                    Some(text(c))
                } else if c["type"] == "text" {
                    Some(text(&c["text"]))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        if !chunks.is_empty() {
            return (chunks.join("\n"), "state_content");
        }
    }
    if st["output"].is_string() {
        return (text(&st["output"]), "state_output");
    }
    if st["error"]["message"].is_string() {
        return (text(&st["error"]["message"]), "state_error_msg");
    }
    if st["error"].is_string() {
        return (text(&st["error"]), "state_error_str");
    }
    if st.get("input").is_some() {
        return (
            serde_json::to_string_pretty(&st["input"]).unwrap_or_default(),
            "state_input",
        );
    }
    (String::new(), "none")
}
fn tool_display(p: &Value) -> String {
    let mut lines = Vec::new();
    if let Some(inp) = p["state"]["input"].as_object().filter(|v| !v.is_empty()) {
        if inp.len() == 1 && inp.contains_key("command") {
            lines.push(format!("$ {}", text(&inp["command"])));
        } else {
            lines.push(serde_json::to_string_pretty(inp).unwrap_or_default());
        }
    }
    let (body, mode) = tool_text(p);
    if !body.is_empty() && mode != "state_input" {
        if !lines.is_empty() {
            lines.push(String::new());
        }
        lines.push(body);
    }
    lines.join("\n").trim().to_owned()
}
fn build_item(
    msg_id: &str,
    seq: i64,
    key: String,
    kind: &str,
    subtype: &str,
    label: &str,
    body: String,
    ts: String,
    tool: Value,
    status: Value,
    code: String,
    readonly: bool,
    truncate: bool,
) -> Value {
    let actions = if readonly {
        vec![]
    } else if kind == "user" {
        vec!["edit", "del_single", "cut_head", "cut_tail"]
    } else {
        vec!["edit", "del_single", "cut_tail"]
    };
    let mut val = json!({"key":key,"msg_id":msg_id,"seq":seq,"kind":kind,"subtype":subtype,"label":label,"text":body,"ts":ts,"tool":tool,"status":status,"code":code,"in_db":true,"actions":actions});
    if truncate {
        truncate_preview_item(&mut val);
    }
    val
}
fn push_message_items(
    msg_id: &str,
    msg_kind: &str,
    seq: i64,
    created: i64,
    d: &Value,
    readonly: bool,
    include_tools: bool,
    truncate: bool,
    out: &mut Vec<Value>,
) {
    let ts = iso(created);
    let key = format!("m:{msg_id}");
    match msg_kind {
        "user" => out.push(build_item(
            msg_id,
            seq,
            key,
            "user",
            "user",
            "Вы",
            text(
                d.get("text")
                    .filter(|v| !v.is_null())
                    .unwrap_or(&d["metadata"]["displayText"]),
            ),
            ts,
            Value::Null,
            Value::Null,
            String::new(),
            readonly,
            truncate,
        )),
        "assistant" => {
            if let Some(parts) = d["content"].as_array() {
                for (i, p) in parts.iter().enumerate() {
                    if !p.is_object() {
                        continue;
                    }
                    let ptype = s(&p["type"]);
                    if !include_tools && ptype != "text" {
                        continue;
                    }
                    let pts = iso(p["time"]["created"]
                        .as_i64()
                        .filter(|t| *t != 0)
                        .unwrap_or(created));
                    let key = format!("p:{msg_id}:{i}");
                    let mut v = match ptype {
                        "text" => {
                            let body = text(&p["text"]);
                            if body.trim().is_empty() && parts.len() > 1 {
                                continue;
                            }
                            build_item(
                                msg_id,
                                seq,
                                key,
                                "assistant",
                                "text",
                                "OpenCode",
                                body,
                                pts,
                                Value::Null,
                                Value::Null,
                                String::new(),
                                readonly,
                                truncate,
                            )
                        }
                        "reasoning" => build_item(
                            msg_id,
                            seq,
                            key,
                            "tool",
                            "thinking",
                            "Размышления",
                            text(&p["text"]),
                            pts,
                            json!("reasoning"),
                            json!("ok"),
                            String::new(),
                            readonly,
                            truncate,
                        ),
                        "tool" => {
                            let name = p["name"]
                                .as_str()
                                .filter(|s| !s.is_empty())
                                .unwrap_or("tool");
                            build_item(
                                msg_id,
                                seq,
                                key,
                                "tool",
                                "tool_use",
                                name,
                                tool_display(p),
                                pts,
                                json!(name),
                                json!(if p["state"]["status"] == "error" {
                                    "error"
                                } else {
                                    "ok"
                                }),
                                String::new(),
                                readonly,
                                truncate,
                            )
                        }
                        _ => continue,
                    };
                    v["part_idx"] = json!(i);
                    out.push(v);
                }
            }
            if truthy(&d["error"]) {
                let e = &d["error"];
                let code = if e.is_object() {
                    text(&e["type"])
                } else {
                    String::new()
                };
                let body = if e.is_object() {
                    if s(&e["message"]).is_empty() {
                        e.to_string()
                    } else {
                        text(&e["message"])
                    }
                } else {
                    text(e)
                };
                out.push(build_item(
                    msg_id,
                    seq,
                    format!("e:{msg_id}"),
                    "error",
                    "error",
                    "Ошибка OpenCode",
                    body,
                    ts,
                    Value::Null,
                    json!("error"),
                    code,
                    readonly,
                    truncate,
                ));
            }
        }
        "compaction" if include_tools => out.push(build_item(
            msg_id,
            seq,
            key,
            "compact",
            "compaction",
            "Компакция",
            text(&d["summary"]),
            ts,
            Value::Null,
            Value::Null,
            String::new(),
            readonly,
            truncate,
        )),
        "system" | "synthetic" if include_tools => {
            let label = d["description"]
                .as_str()
                .filter(|s| !s.is_empty())
                .unwrap_or(if msg_kind == "system" {
                    "Система"
                } else {
                    "Служебное"
                });
            out.push(build_item(
                msg_id,
                seq,
                key,
                "system",
                msg_kind,
                label,
                text(&d["text"]),
                ts,
                Value::Null,
                Value::Null,
                String::new(),
                readonly,
                truncate,
            ));
        }
        _ => {}
    }
}
fn visit_message_entries(
    msg_id: &str,
    msg_kind: &str,
    created: i64,
    d: &Value,
    f: &mut dyn FnMut(EntryRef<'_>) -> bool,
) -> bool {
    let ts = iso(created);
    let key = format!("m:{msg_id}");
    match msg_kind {
        "user" => {
            let body = text(
                d.get("text")
                    .filter(|v| !v.is_null())
                    .unwrap_or(&d["metadata"]["displayText"]),
            );
            f(EntryRef {
                key: &key,
                kind: "user",
                subtype: "user",
                label: "Вы",
                ts: &ts,
                text: &body,
            })
        }
        "assistant" => {
            if let Some(parts) = d["content"].as_array() {
                for (i, p) in parts.iter().enumerate() {
                    if !p.is_object() {
                        continue;
                    }
                    let pts = iso(p["time"]["created"]
                        .as_i64()
                        .filter(|t| *t != 0)
                        .unwrap_or(created));
                    let pkey = format!("p:{msg_id}:{i}");
                    let keep = match s(&p["type"]) {
                        "text" => {
                            let body = text(&p["text"]);
                            if body.trim().is_empty() && parts.len() > 1 {
                                continue;
                            }
                            f(EntryRef {
                                key: &pkey,
                                kind: "assistant",
                                subtype: "text",
                                label: "OpenCode",
                                ts: &pts,
                                text: &body,
                            })
                        }
                        "reasoning" => {
                            let body = text(&p["text"]);
                            f(EntryRef {
                                key: &pkey,
                                kind: "tool",
                                subtype: "thinking",
                                label: "Размышления",
                                ts: &pts,
                                text: &body,
                            })
                        }
                        "tool" => {
                            let name = p["name"]
                                .as_str()
                                .filter(|s| !s.is_empty())
                                .unwrap_or("tool");
                            let body = tool_display(p);
                            f(EntryRef {
                                key: &pkey,
                                kind: "tool",
                                subtype: "tool_use",
                                label: name,
                                ts: &pts,
                                text: &body,
                            })
                        }
                        _ => continue,
                    };
                    if !keep {
                        return false;
                    }
                }
            }
            if truthy(&d["error"]) {
                let e = &d["error"];
                let body = if e.is_object() {
                    if s(&e["message"]).is_empty() {
                        e.to_string()
                    } else {
                        text(&e["message"])
                    }
                } else {
                    text(e)
                };
                let ekey = format!("e:{msg_id}");
                return f(EntryRef {
                    key: &ekey,
                    kind: "error",
                    subtype: "error",
                    label: "Ошибка OpenCode",
                    ts: &ts,
                    text: &body,
                });
            }
            true
        }
        "compaction" => {
            let body = text(&d["summary"]);
            f(EntryRef {
                key: &key,
                kind: "compact",
                subtype: "compaction",
                label: "Компакция",
                ts: &ts,
                text: &body,
            })
        }
        "system" | "synthetic" => {
            let label = d["description"]
                .as_str()
                .filter(|s| !s.is_empty())
                .unwrap_or(if msg_kind == "system" {
                    "Система"
                } else {
                    "Служебное"
                });
            let body = text(&d["text"]);
            f(EntryRef {
                key: &key,
                kind: "system",
                subtype: msg_kind,
                label,
                ts: &ts,
                text: &body,
            })
        }
        _ => true,
    }
}
fn items_from_messages(messages: &[Message], readonly: bool, include_tools: bool) -> Vec<Value> {
    let mut out = Vec::new();
    for m in messages {
        push_message_items(
            &m.id,
            &m.kind,
            m.seq,
            m.created,
            &m.data,
            readonly,
            include_tools,
            false,
            &mut out,
        );
    }
    out
}
fn page_slice(flat: &[Value], data: &Value, sid: &str) -> Value {
    let tools = data["include_tools"].as_bool().unwrap_or(false);
    let total = flat.len();
    let lim = data["limit"].as_u64().unwrap_or(80).clamp(1, 150) as usize;
    if let Some(target_key) = data["around_key"].as_str().filter(|s| !s.is_empty()) {
        if let Some(idx) = flat.iter().position(|it| s(&it["key"]) == target_key) {
            let after_limit = (lim / 2).max(15);
            let half_max = if tools { 120 } else { lim };
            let mut end = idx + 1;
            let mut after_msgs = usize::from(matches!(s(&flat[idx]["kind"]), "user" | "assistant"));
            while end < total && after_msgs < after_limit && end - idx < half_max {
                if matches!(s(&flat[end]["kind"]), "user" | "assistant") {
                    after_msgs += 1;
                }
                end += 1;
            }
            let before_limit = lim.saturating_sub(after_msgs).max(15);
            let mut start = idx;
            let mut before_msgs = 0;
            while start > 0 && before_msgs < before_limit && idx - start < half_max {
                start -= 1;
                if matches!(s(&flat[start]["kind"]), "user" | "assistant") {
                    before_msgs += 1;
                }
            }
            let window = flat[start..end].to_vec();
            let turns = if window.is_empty() {
                vec![]
            } else {
                vec![json!({"items":window})]
            };
            let mut resp = json!({"provider":"opencode","cid":sid,"turns":turns,"page_msgs":before_msgs+after_msgs,"total_label":format!("записей: {total}"),"cursor":if start>0{Some(start.to_string())}else{None},"has_more":start>0,"writable":true});
            if end < total {
                resp["has_newer"] = json!(true);
            }
            return resp;
        }
    }
    let end = data["before"]
        .as_str()
        .and_then(|s| s.parse::<usize>().ok())
        .or_else(|| data["before"].as_u64().map(|n| n as usize))
        .unwrap_or(total)
        .min(total);
    let max = if tools { 350 } else { lim * 2 };
    let mut start = end;
    let mut msgs = 0;
    while start > 0 && msgs < lim && end - start < max {
        start -= 1;
        if matches!(s(&flat[start]["kind"]), "user" | "assistant") {
            msgs += 1;
        }
    }
    let window = flat[start..end].to_vec();
    let turns = if window.is_empty() {
        vec![]
    } else {
        vec![json!({"items":window})]
    };
    json!({"provider":"opencode","cid":sid,"turns":turns,"page_msgs":msgs,"total_label":format!("записей: {total}"),"cursor":if start>0{Some(start.to_string())}else{None},"has_more":start>0,"writable":true})
}
fn update_message(c: &Connection, sid: &str, m: &Message, now: i64) -> Result<()> {
    c.execute(
        "UPDATE session_message SET data=?,time_updated=? WHERE id=? AND session_id=?",
        params![serde_json::to_string(&m.data)?, now, m.id, sid],
    )?;
    Ok(())
}
fn wal_checkpoint(c: &Connection, truncate: bool) {
    let mode = if truncate { "TRUNCATE" } else { "PASSIVE" };
    let _ = c.execute_batch(&format!("PRAGMA wal_checkpoint({mode});"));
}
fn file_stat_sig(path: &Path) -> Option<(u64, u128)> {
    let meta = fs::metadata(path).ok()?;
    let mtime = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    Some((meta.len(), mtime))
}
fn quick_check(c: &Connection) -> Result<()> {
    let check: String = c.query_row("PRAGMA quick_check", [], |r| r.get(0))?;
    if check != "ok" {
        bail!("quick_check failed: {check}");
    }
    Ok(())
}
fn table_names(c: &Connection) -> Result<Vec<String>> {
    let mut q = c.prepare("SELECT name FROM sqlite_master WHERE type='table'")?;
    let v = q
        .query_map([], |r| r.get(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(v)
}
fn columns(c: &Connection, table: &str) -> Result<Vec<String>> {
    let mut q = c.prepare(&format!(
        "PRAGMA table_info(\"{}\")",
        table.replace('"', "\"\"")
    ))?;
    let v = q
        .query_map([], |r| r.get(1))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(v)
}

struct SnapshotTable<'a> {
    columns: Vec<&'a str>,
    rows: Vec<&'a Vec<Value>>,
}
fn validate_snapshot<'a>(
    snap: &'a Value,
    sid: &str,
) -> Result<BTreeMap<&'static str, SnapshotTable<'a>>> {
    let tables = snap["tables"]
        .as_object()
        .context("Повреждена резервная копия OpenCode: отсутствуют таблицы")?;
    for name in ["session_v2", "session_message"] {
        if !tables.contains_key(name) {
            bail!("Повреждена резервная копия OpenCode: отсутствует {name}");
        }
    }
    let mut validated = BTreeMap::new();
    for (name, owner) in BACKUP_TABLES {
        let Some(data) = tables.get(name) else {
            continue;
        };
        let raw_columns = data["columns"]
            .as_array()
            .or_else(|| data["cols"].as_array())
            .with_context(|| format!("Повреждена схема резервной копии {name}"))?;
        let columns = raw_columns
            .iter()
            .map(|v| {
                v.as_str()
                    .filter(|s| !s.is_empty())
                    .context("Некорректное имя столбца резервной копии")
            })
            .collect::<Result<Vec<_>>>()?;
        let mut unique = std::collections::HashSet::new();
        if !columns.iter().all(|c| unique.insert(*c)) {
            bail!("Повторяющиеся столбцы в резервной копии {name}");
        }
        let owner_idx = columns
            .iter()
            .position(|c| *c == owner)
            .with_context(|| format!("В резервной копии {name} отсутствует {owner}"))?;
        let raw_rows = data["rows"]
            .as_array()
            .with_context(|| format!("Повреждены строки резервной копии {name}"))?;
        if name == "session_v2" && raw_rows.len() != 1 {
            bail!("Резервная копия должна содержать ровно одну сессию OpenCode");
        }
        let mut rows = Vec::new();
        for (number, value) in raw_rows.iter().enumerate() {
            let row = value
                .as_array()
                .with_context(|| format!("Повреждена строка {number} таблицы {name}"))?;
            if row.len() != columns.len() {
                bail!("Длина строки {number} таблицы {name} не совпадает со схемой");
            }
            if row[owner_idx].as_str() != Some(sid) {
                bail!("Резервная копия {name} содержит данные другого диалога");
            }
            for value in row {
                json_sql(value).with_context(|| {
                    format!("Некорректные данные в строке {number} таблицы {name}")
                })?;
            }
            rows.push(row);
        }
        validated.insert(name, SnapshotTable { columns, rows });
    }
    Ok(validated)
}

fn sql_json(v: ValueRef<'_>) -> Value {
    match v {
        ValueRef::Null => Value::Null,
        ValueRef::Integer(n) => json!(n),
        ValueRef::Real(n) => json!(n),
        ValueRef::Text(s) => json!(String::from_utf8_lossy(s)),
        ValueRef::Blob(b) => {
            use base64::Engine;
            json!({"__k1tty_blob__":base64::engine::general_purpose::STANDARD.encode(b)})
        }
    }
}
fn json_sql(v: &Value) -> Result<SqlValue> {
    Ok(match v {
        Value::Null => SqlValue::Null,
        Value::Bool(b) => SqlValue::Integer(i64::from(*b)),
        Value::Number(n) => {
            if let Some(n) = n.as_i64() {
                SqlValue::Integer(n)
            } else {
                SqlValue::Real(n.as_f64().context("Некорректное число")?)
            }
        }
        Value::String(s) => SqlValue::Text(s.clone()),
        Value::Object(o) if o.len() == 1 && o.contains_key("__k1tty_blob__") => {
            use base64::Engine;
            SqlValue::Blob(
                base64::engine::general_purpose::STANDARD.decode(
                    o["__k1tty_blob__"]
                        .as_str()
                        .context("Некорректное бинарное поле резервной копии")?,
                )?,
            )
        }
        _ => bail!("Неподдерживаемое поле резервной копии"),
    })
}
fn snapshot_title_map(tables: &Map<String, Value>) -> String {
    let Some(v) = tables.get("session_v2") else {
        return String::new();
    };
    let cols = v["columns"].as_array().or_else(|| v["cols"].as_array());
    cols.and_then(|cols| cols.iter().position(|v| v == "title"))
        .map(|i| text(&v["rows"][0][i]).trim().to_owned())
        .unwrap_or_default()
}
fn snapshot_title(tables: &Value) -> String {
    tables
        .as_object()
        .map(snapshot_title_map)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (tempfile::TempDir, OpenCodeProvider) {
        let temp = tempfile::tempdir().unwrap();
        let p = OpenCodeProvider::new(ProviderPaths {
            id: "opencode".into(),
            name: "OpenCode".into(),
            short: "OC".into(),
            root: temp.path().join("data"),
            backup_root: temp.path().join("backups"),
        });
        fs::create_dir_all(&p.paths.root).unwrap();
        let c = Connection::open(p.db()).unwrap();
        c.execute_batch("PRAGMA foreign_keys=ON;CREATE TABLE session_v2(id TEXT PRIMARY KEY,parent_id TEXT REFERENCES session_v2(id) ON DELETE CASCADE,slug TEXT,directory TEXT,title TEXT,time_created INTEGER,time_updated INTEGER,time_archived INTEGER,revert TEXT);CREATE TABLE session_message(id TEXT PRIMARY KEY,session_id TEXT REFERENCES session_v2(id) ON DELETE CASCADE,type TEXT,seq INTEGER,time_created INTEGER,time_updated INTEGER,data TEXT);CREATE TABLE event_sequence(aggregate_id TEXT PRIMARY KEY,seq INTEGER,owner_id TEXT);INSERT INTO session_v2 VALUES('one',NULL,'slug','/tmp','Title',1700000000000,1700000000000,NULL,NULL);INSERT INTO event_sequence VALUES('one',4,NULL);").unwrap();
        let messages = [
            (
                "user",
                "user",
                json!({"text":"Привет 🌍","metadata":{"displayText":"Привет 🌍"},"futureField":{"retain":true}}),
            ),
            (
                "assistant",
                "assistant",
                json!({"content":[{"type":"reasoning","text":"Thinking"},{"type":"text","text":"Ответ"},{"type":"tool","name":"bash","state":{"input":{"command":"pwd"},"content":[{"type":"text","text":"/tmp"}]}}],"futureField":42}),
            ),
            (
                "final",
                "assistant",
                json!({"content":[{"type":"text","text":"Done"}]}),
            ),
        ];
        for (i, (id, kind, data)) in messages.into_iter().enumerate() {
            c.execute(
                "INSERT INTO session_message VALUES(?,'one',?,?,1700000000000,1700000000000,?)",
                params![id, kind, i + 1, data.to_string()],
            )
            .unwrap();
        }
        (temp, p)
    }
    #[test]
    fn edit_backup_restore_and_unknown_fields() {
        let (_temp, p) = fixture();
        let rep = p
            .call(
                "apply_edit",
                &json!({"cid":"one","key":"m:user","text":"Изменено"}),
                &OpContext::default(),
            )
            .unwrap();
        assert_eq!(p.preview_edit("one", "m:user").unwrap()["text"], "Изменено");
        let m = OpenCodeProvider::load(&p.ro().unwrap(), "one", "user").unwrap();
        assert_eq!(m.data["futureField"]["retain"], true);
        assert_eq!(m.data["metadata"]["displayText"], "Изменено");
        p.restore(Path::new(s(&rep["backup"]))).unwrap();
        assert_eq!(
            p.preview_edit("one", "m:user").unwrap()["text"],
            "Привет 🌍"
        );
    }
    #[test]
    fn cut_keeps_prior_parts_restore_keeps_sequence_monotonic() {
        let (_temp, p) = fixture();
        let backup = p.backup("one", "cut").unwrap();
        p.cut("one", "p:assistant:1", "tail").unwrap();
        let items = p.items("one").unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[1]["subtype"], "thinking");
        p.rw()
            .unwrap()
            .execute("UPDATE event_sequence SET seq=100", [])
            .unwrap();
        p.restore(&backup).unwrap();
        assert_eq!(p.items("one").unwrap().len(), 5);
        let seq: i64 = p
            .ro()
            .unwrap()
            .query_row("SELECT seq FROM event_sequence", [], |r| r.get(0))
            .unwrap();
        assert_eq!(seq, 100);
    }
    #[test]
    fn tool_edit_and_delete_only_target_part() {
        let (_temp, p) = fixture();
        p.edit("one", "p:assistant:2", "changed").unwrap();
        let m = OpenCodeProvider::load(&p.ro().unwrap(), "one", "assistant").unwrap();
        assert_eq!(m.data["content"][2]["state"]["input"]["command"], "pwd");
        assert_eq!(
            m.data["content"][2]["state"]["content"][0]["text"],
            "changed"
        );
        p.delete_single("one", "p:assistant:1").unwrap();
        let m = OpenCodeProvider::load(&p.ro().unwrap(), "one", "assistant").unwrap();
        assert_eq!(m.data["content"].as_array().unwrap().len(), 2);
        assert_eq!(m.data["futureField"], 42);
    }
    #[test]
    fn paginates_and_inspects_python_layout() {
        let (_temp, p) = fixture();
        let result = p
            .call(
                "entries",
                &json!({"cid":"one","limit":1,"include_tools":true}),
                &OpContext::default(),
            )
            .unwrap();
        assert_eq!(result["cursor"], "4");
        let backup = p.backup("one", "inspect").unwrap();
        let inspected = p
            .backup_entries(&json!({"backup_dir":backup,"include_tools":true}))
            .unwrap();
        assert_eq!(inspected["readonly"], true);
        assert!(inspected["turns"][0]["items"][0]["actions"]
            .as_array()
            .unwrap()
            .is_empty());
        assert_eq!(inspected["title"], "Title");
    }

    #[test]
    fn corrupt_backup_is_rejected_before_any_rows_change() {
        let (_temp, p) = fixture();
        let backup = p.backup("one", "validation").unwrap();
        let path = backup.join("session_backup.json");
        let original = storage::read_json(&path).unwrap();
        let mut variants = Vec::new();
        variants.push(json!({"session_id":"one","tables":{}}));
        let mut missing = original.clone();
        missing["tables"]
            .as_object_mut()
            .unwrap()
            .remove("session_message");
        variants.push(missing);
        let mut owner = original.clone();
        let owner_col = owner["tables"]["session_message"]["columns"]
            .as_array()
            .unwrap()
            .iter()
            .position(|v| v == "session_id")
            .unwrap();
        owner["tables"]["session_message"]["rows"][0][owner_col] = json!("another");
        variants.push(owner);
        let mut sid = original.clone();
        let sid_col = sid["tables"]["session_v2"]["columns"]
            .as_array()
            .unwrap()
            .iter()
            .position(|v| v == "id")
            .unwrap();
        sid["tables"]["session_v2"]["rows"][0][sid_col] = json!("another");
        variants.push(sid);
        let mut width = original.clone();
        width["tables"]["session_message"]["rows"][0]
            .as_array_mut()
            .unwrap()
            .pop();
        variants.push(width);
        let mut duplicate = original.clone();
        duplicate["tables"]["session_message"]["columns"][1] =
            duplicate["tables"]["session_message"]["columns"][0].clone();
        variants.push(duplicate);
        let mut invalid = original.clone();
        invalid["tables"]["session_message"]["rows"][0][0] = json!({"unsupported":true});
        variants.push(invalid);
        let mut invalid_blob = original.clone();
        invalid_blob["tables"]["session_message"]["rows"][0][0] = json!({"__k1tty_blob__":null});
        variants.push(invalid_blob);
        let mut empty = original.clone();
        empty["tables"]["session_v2"]["rows"] = json!([]);
        variants.push(empty);
        let before = p.items("one").unwrap();
        for (i, snapshot) in variants.into_iter().enumerate() {
            storage::atomic_write(&path, &serde_json::to_vec(&snapshot).unwrap()).unwrap();
            assert!(p.restore(&backup).is_err(), "corrupt variant {i} accepted");
            assert_eq!(p.items("one").unwrap(), before, "variant {i} changed data");
            assert_eq!(p.sessions().unwrap().len(), 1);
        }
    }

    #[test]
    fn restore_preserves_child_sessions_with_cascade_foreign_keys() {
        let (_temp, p) = fixture();
        let backup = p.backup("one", "parent").unwrap();
        let c = p.rw().unwrap();
        c.execute("INSERT INTO session_v2 VALUES('child','one','child','/tmp','Child',1700000000000,1700000000000,NULL,NULL)",[]).unwrap();
        c.execute("INSERT INTO session_message VALUES('child-user','child','user',1,1700000000000,1700000000000,?)",[json!({"text":"Independent child 🐱"}).to_string()]).unwrap();
        drop(c);
        p.edit("one", "m:user", "Changed parent").unwrap();
        p.restore(&backup).unwrap();
        assert_eq!(
            p.preview_edit("one", "m:user").unwrap()["text"],
            "Привет 🌍"
        );
        assert_eq!(
            p.preview_edit("child", "m:child-user").unwrap()["text"],
            "Independent child 🐱"
        );
        assert_eq!(p.sessions().unwrap().len(), 2);
    }

    #[test]
    fn malformed_message_payloads_cannot_panic_or_mutate_storage() {
        let (_temp, p) = fixture();
        for malformed in [json!([]), json!(true), json!(12), json!("bad"), Value::Null] {
            let raw = malformed.to_string();
            p.rw()
                .unwrap()
                .execute("UPDATE session_message SET data=? WHERE id='user'", [&raw])
                .unwrap();
            assert!(p
                .items("one")
                .unwrap()
                .iter()
                .all(|v| v["msg_id"] != "user"));
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                p.edit("one", "m:user", "New text")
            }));
            assert!(result.is_ok());
            assert!(result.unwrap().is_err());
            assert_eq!(
                p.ro()
                    .unwrap()
                    .query_row(
                        "SELECT data FROM session_message WHERE id='user'",
                        [],
                        |r| r.get::<_, String>(0)
                    )
                    .unwrap(),
                raw
            );
        }
    }

    #[test]
    fn restores_unknown_columns_and_blob_values_without_clobbering_other_sessions() {
        let (_temp, p) = fixture();
        p.rw().unwrap().execute_batch("ALTER TABLE session_v2 ADD COLUMN future_blob BLOB;UPDATE session_v2 SET future_blob=X'0001FF';").unwrap();
        let backup = p.backup("one", "blob").unwrap();
        p.rw()
            .unwrap()
            .execute("UPDATE session_v2 SET future_blob=X'22'", [])
            .unwrap();
        p.restore(&backup).unwrap();
        assert_eq!(
            p.ro()
                .unwrap()
                .query_row(
                    "SELECT future_blob FROM session_v2 WHERE id='one'",
                    [],
                    |r| r.get::<_, Vec<u8>>(0)
                )
                .unwrap(),
            vec![0, 1, 255]
        );
        let path = backup.join("session_backup.json");
        let mut snapshot = storage::read_json(&path).unwrap();
        snapshot["tables"]["session_v2"]["columns"]
            .as_array_mut()
            .unwrap()
            .push(json!("newer_client_column"));
        snapshot["tables"]["session_v2"]["rows"][0]
            .as_array_mut()
            .unwrap()
            .push(json!("not in this schema"));
        storage::atomic_write(&path, &serde_json::to_vec(&snapshot).unwrap()).unwrap();
        p.restore(&backup).unwrap();
        assert_eq!(
            p.preview_edit("one", "m:user").unwrap()["text"],
            "Привет 🌍"
        );
    }
}
