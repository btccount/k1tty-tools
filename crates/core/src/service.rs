use crate::{
    converter, pack, process,
    provider::{required, valid_id, OpContext, Provider, ProviderPaths},
    storage,
};
use anyhow::{bail, Context, Result};
use serde_json::{json, Map, Value};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
    thread,
    time::{Duration, Instant},
};

pub const PROVIDER_ORDER: [&str; 5] = [
    "antigravity",
    "antigravity_cli",
    "codex",
    "claude",
    "opencode",
];
pub type EventSink = Arc<dyn Fn(Value) + Send + Sync>;

pub struct AppPaths {
    pub data_dir: PathBuf,
    pub providers: Vec<ProviderPaths>,
}

fn env_path(name: &str, default: PathBuf) -> PathBuf {
    std::env::var_os(name)
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .unwrap_or(default)
}

impl AppPaths {
    pub fn discover(data_dir: PathBuf) -> Result<Self> {
        let home = dirs::home_dir().context("Не удалось определить домашнюю папку")?;
        let data_dir = env_path("K1TTY_DATA_DIR", data_dir);
        fs::create_dir_all(&data_dir)?;
        let mut legacy_roots = Vec::new();
        if let Some(p) = std::env::var_os("K1TTY_LEGACY_DIR") {
            legacy_roots.push(PathBuf::from(p));
        }
        if let Ok(cwd) = std::env::current_dir() {
            legacy_roots.push(cwd);
        }
        if let Ok(exe) = std::env::current_exe() {
            if let Some(p) = exe.parent() {
                legacy_roots.push(p.to_owned());
            }
        }
        for root in legacy_roots {
            if root == data_dir {
                continue;
            }
            for old in [root.clone(), root.join("k1tty_editor")] {
                copy_missing(&old.join("backups"), &data_dir.join("backups"))?;
                let settings = old.join("k1tty_settings.json");
                if settings.is_file() && !data_dir.join("k1tty_settings.json").exists() {
                    storage::atomic_write(
                        &data_dir.join("k1tty_settings.json"),
                        &fs::read(settings)?,
                    )?;
                }
            }
            copy_missing(
                &root.join("k1tty_editor/engines/backups"),
                &data_dir.join("backups/antigravity"),
            )?;
        }
        let codex = env_path("CODEX_HOME", home.join(".codex"));
        let claude = env_path("CLAUDE_HOME", home.join(".claude"));
        let old_ag = home.join("Documents/Проект по умолчанию/antigravity_history_editor/backups");
        let ag_backups = if old_ag.is_dir() {
            old_ag
        } else {
            data_dir.join("backups/antigravity")
        };
        let providers = vec![
            ProviderPaths {
                id: "antigravity".into(),
                name: "Antigravity".into(),
                short: "AG".into(),
                root: env_path("ANTIGRAVITY_APPDATA", home.join(".gemini/antigravity")),
                backup_root: env_path("ANTIGRAVITY_BACKUP_ROOT", ag_backups),
            },
            ProviderPaths {
                id: "antigravity_cli".into(),
                name: "Antigravity CLI".into(),
                short: "CLI".into(),
                root: env_path(
                    "ANTIGRAVITY_CLI_APPDATA",
                    home.join(".gemini/antigravity-cli"),
                ),
                backup_root: env_path(
                    "ANTIGRAVITY_CLI_BACKUP_ROOT",
                    data_dir.join("backups/antigravity_cli"),
                ),
            },
            ProviderPaths {
                id: "codex".into(),
                name: "Codex".into(),
                short: "CX".into(),
                backup_root: codex.parent().unwrap_or(&home).join("codex_backups"),
                root: codex,
            },
            ProviderPaths {
                id: "claude".into(),
                name: "Claude".into(),
                short: "CL".into(),
                backup_root: claude.parent().unwrap_or(&home).join("claude_backups"),
                root: claude,
            },
            ProviderPaths {
                id: "opencode".into(),
                name: "OpenCode".into(),
                short: "OC".into(),
                root: env_path("OPENCODE_DATA_DIR", home.join(".local/share/opencode")),
                backup_root: env_path("OPENCODE_BACKUP_ROOT", data_dir.join("backups/opencode")),
            },
        ];
        Ok(Self {
            data_dir,
            providers,
        })
    }

    pub fn isolated(root: &Path) -> Self {
        let names = [
            ("antigravity", "Antigravity", "AG"),
            ("antigravity_cli", "Antigravity CLI", "CLI"),
            ("codex", "Codex", "CX"),
            ("claude", "Claude", "CL"),
            ("opencode", "OpenCode", "OC"),
        ];
        Self {
            data_dir: root.join("k1tty"),
            providers: names
                .into_iter()
                .map(|(id, name, short)| ProviderPaths {
                    id: id.into(),
                    name: name.into(),
                    short: short.into(),
                    root: root.join(id).join("sandbox"),
                    backup_root: root.join("backups").join(id),
                })
                .collect(),
        }
    }
}

fn copy_missing(source: &Path, target: &Path) -> Result<()> {
    if !source.is_dir() {
        return Ok(());
    }
    for entry in walkdir::WalkDir::new(source).follow_links(false) {
        let entry = entry?;
        let dest = target.join(entry.path().strip_prefix(source)?);
        if entry.file_type().is_dir() {
            fs::create_dir_all(dest)?;
        } else if entry.file_type().is_file() && !dest.exists() {
            fs::copy(entry.path(), dest)?;
        }
    }
    Ok(())
}

struct CachedPack {
    bytes: Vec<u8>,
    filename: String,
    created: Instant,
}

pub struct Service {
    providers: BTreeMap<String, Arc<dyn Provider>>,
    locks: BTreeMap<String, Mutex<()>>,
    settings: Mutex<Value>,
    settings_path: PathBuf,
    emit: EventSink,
    view: Mutex<Option<(String, String)>>,
    downloads: Mutex<BTreeMap<String, CachedPack>>,
    uploads: Mutex<BTreeMap<String, CachedPack>>,
    active_jobs: AtomicUsize,
    boot_id: String,
}

struct ActiveOperation<'a>(&'a AtomicUsize);
impl Drop for ActiveOperation<'_> {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}

impl Service {
    pub fn new(paths: AppPaths, emit: EventSink) -> Result<Arc<Self>> {
        fs::create_dir_all(&paths.data_dir)?;
        let settings_path = paths.data_dir.join("k1tty_settings.json");
        let auto_backup = storage::read_json(&settings_path)
            .ok()
            .and_then(|v| v["auto_backup"].as_bool())
            .unwrap_or(true);
        let mut providers: BTreeMap<String, Arc<dyn Provider>> = BTreeMap::new();
        for p in paths.providers {
            let id = p.id.clone();
            let instance: Arc<dyn Provider> = match id.as_str() {
                "antigravity" | "antigravity_cli" => {
                    Arc::new(crate::antigravity::AntigravityProvider::new(p))
                }
                "codex" => Arc::new(crate::codex::CodexProvider::new(p)),
                "claude" => Arc::new(crate::claude::ClaudeProvider::new(p)),
                "opencode" => Arc::new(crate::opencode::OpenCodeProvider::new(p)),
                _ => bail!("Неизвестный провайдер: {id}"),
            };
            providers.insert(id, instance);
        }
        let locks = providers
            .keys()
            .map(|id| (id.clone(), Mutex::new(())))
            .collect();
        Ok(Arc::new(Self {
            providers,
            locks,
            settings: Mutex::new(json!({"auto_backup":auto_backup})),
            settings_path,
            emit,
            view: Mutex::new(None),
            downloads: Mutex::new(BTreeMap::new()),
            uploads: Mutex::new(BTreeMap::new()),
            active_jobs: AtomicUsize::new(0),
            boot_id: uuid::Uuid::new_v4().simple().to_string(),
        }))
    }

    pub fn hello(&self) -> Value {
        json!({"type":"hello","server":"k1tty-rust-tauri","boot_id":self.boot_id})
    }
    pub fn active_jobs(&self) -> usize {
        self.active_jobs.load(Ordering::SeqCst)
    }
    fn active_operation(&self) -> ActiveOperation<'_> {
        self.active_jobs.fetch_add(1, Ordering::SeqCst);
        ActiveOperation(&self.active_jobs)
    }
    pub fn provider(&self, id: &str) -> Result<Arc<dyn Provider>> {
        self.providers
            .get(id)
            .cloned()
            .with_context(|| format!("Неизвестный провайдер: {id}"))
    }
    fn settings(&self) -> Value {
        self.settings
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }
    fn context(&self) -> OpContext {
        OpContext {
            auto_backup: self.settings()["auto_backup"].as_bool().unwrap_or(true),
            ..OpContext::default()
        }
    }

    pub fn rpc(self: &Arc<Self>, op: &str, provider: Option<&str>, data: Value) -> Result<Value> {
        let pid = provider.unwrap_or("all");
        match op {
            "ping" => {
                return Ok(
                    json!({"pong":true,"ts":chrono::Utc::now().timestamp_millis() as f64/1000.0}),
                )
            }
            "set_view" => {
                let mut view = self.view.lock().unwrap_or_else(|e| e.into_inner());
                *view = if self.providers.contains_key(pid) {
                    data["cid"].as_str().map(|s| (pid.into(), s.into()))
                } else {
                    None
                };
                return Ok(json!({"ok":true}));
            }
            "get_settings" => return Ok(json!({"settings":self.settings()})),
            "set_settings" => {
                let mut settings = self.settings.lock().unwrap_or_else(|e| e.into_inner());
                if let Some(v) = data["auto_backup"].as_bool() {
                    settings["auto_backup"] = json!(v);
                }
                storage::atomic_write(
                    &self.settings_path,
                    &serde_json::to_vec_pretty(&*settings)?,
                )?;
                let snap = settings.clone();
                drop(settings);
                (self.emit)(json!({"type":"settings","settings":snap}));
                return Ok(json!({"settings":snap}));
            }
            "state" => {
                process::snapshot_force(data["force"].as_bool().unwrap_or(false));
                let mut states = Map::new();
                for id in PROVIDER_ORDER {
                    if let Some(p) = self.providers.get(id) {
                        states.insert(id.into(), p.call("state", &data, &self.context())?);
                    }
                }
                return Ok(json!({"providers":states,"settings":self.settings()}));
            }
            "convs" => {
                let targets = self.targets(pid)?;
                let mut convs = Vec::new();
                let mut errors = Map::new();
                let mut counts = Map::new();
                for p in targets {
                    let id = &p.paths().id;
                    match p.call(op, &data, &self.context()) {
                        Ok(value) => {
                            let mut list = value
                                .as_array()
                                .cloned()
                                .context("Некорректный список диалогов")?;
                            counts.insert(id.clone(), json!(list.len()));
                            convs.append(&mut list);
                        }
                        Err(err) if pid == "all" => {
                            errors.insert(id.clone(), json!(format!("{err:#}")));
                            counts.insert(id.clone(), json!(0));
                        }
                        Err(err) => return Err(err),
                    }
                }
                convs.sort_by(|a, b| {
                    b["mtime_ts"]
                        .as_f64()
                        .unwrap_or(0.0)
                        .total_cmp(&a["mtime_ts"].as_f64().unwrap_or(0.0))
                });
                return Ok(json!({"convs":convs,"counts":counts,"errors":errors}));
            }
            "search_all" => {
                let raw_q = data["query"].as_str().unwrap_or("").trim();
                if raw_q.chars().count() < 2 {
                    return Ok(json!({"query": raw_q, "results": []}));
                }
                let q = raw_q.to_lowercase();
                let limit = data["limit"].as_u64().unwrap_or(40).clamp(1, 100) as usize;
                let ctx = self.context();
                let mut results = Vec::new();
                for p in self.targets(pid)? {
                    if results.len() >= limit {
                        break;
                    }
                    let id = p.paths().id.clone();
                    let Ok(convs_val) = p.call("convs", &json!({}), &ctx) else {
                        continue;
                    };
                    let Some(convs) = convs_val.as_array() else {
                        continue;
                    };
                    for conv in convs {
                        if results.len() >= limit {
                            break;
                        }
                        let cid = conv["id"].as_str().unwrap_or("");
                        if cid.is_empty() {
                            continue;
                        }
                        let title = conv["title"].as_str().unwrap_or("");
                        let mtime = conv["mtime"].as_str().unwrap_or("");
                        let mtime_ts = conv["mtime_ts"].as_f64().unwrap_or(0.0);
                        let entries = collect_entries(p.as_ref(), cid, &ctx, 2);
                        let mut hit_count = 0usize;
                        let mut first_key = String::new();
                        let mut first_kind = String::new();
                        let mut first_snippet = String::new();
                        for entry in &entries {
                            let text = entry["text"].as_str().unwrap_or("");
                            let label = entry["label"].as_str().unwrap_or("");
                            if let Some(snip) = match_snippet(text, &q) {
                                hit_count += 1;
                                if first_snippet.is_empty() {
                                    first_key = entry["key"].as_str().unwrap_or("").to_owned();
                                    first_kind = entry["kind"].as_str().unwrap_or("").to_owned();
                                    first_snippet = snip;
                                }
                            } else if !label.is_empty() && label.to_lowercase().contains(&q) {
                                hit_count += 1;
                                if first_snippet.is_empty() {
                                    first_key = entry["key"].as_str().unwrap_or("").to_owned();
                                    first_kind = entry["kind"].as_str().unwrap_or("").to_owned();
                                    first_snippet = trim_chars(label, 140);
                                }
                            }
                        }
                        let title_hit = title.to_lowercase().contains(&q);
                        if hit_count > 0 || title_hit {
                            results.push(json!({
                                "provider": id,
                                "cid": cid,
                                "title": title,
                                "mtime": mtime,
                                "mtime_ts": mtime_ts,
                                "key": first_key,
                                "kind": first_kind,
                                "snippet": if first_snippet.is_empty() { trim_chars(title, 140) } else { first_snippet },
                                "hit_count": hit_count.max(usize::from(title_hit))
                            }));
                        }
                    }
                }
                results.sort_by(|a, b| {
                    b["mtime_ts"]
                        .as_f64()
                        .unwrap_or(0.0)
                        .total_cmp(&a["mtime_ts"].as_f64().unwrap_or(0.0))
                });
                return Ok(json!({"query": raw_q, "results": results}));
            }
            "backups" => {
                let mut groups = Vec::new();
                let mut roots = Map::new();
                let mut summary = Vec::new();
                let mut count = 0u64;
                let mut bytes = 0u64;
                for p in self.targets(pid)? {
                    let d = if pid == "all" {
                        json!({})
                    } else {
                        data.clone()
                    };
                    let r = match p.call(op, &d, &self.context()) {
                        Ok(v) => v,
                        Err(_) if pid == "all" => continue,
                        Err(e) => return Err(e),
                    };
                    if let Some(gs) = r["groups"].as_array() {
                        groups.extend(gs.iter().cloned());
                    }
                    let cnt = r["total_count"].as_u64().unwrap_or(0);
                    let size = r["total_bytes"].as_u64().unwrap_or(0);
                    count += cnt;
                    bytes += size;
                    roots.insert(p.paths().id.clone(), r["root"].clone());
                    summary.push(json!({"provider":p.paths().id,"name":p.paths().name,"short":p.paths().short,"count":cnt,"bytes":size}));
                }
                return Ok(
                    json!({"groups":groups,"roots":roots,"summary":summary,"total_count":count,"total_bytes":bytes}),
                );
            }
            "backups_summary" => {
                let rows = self
                    .targets(pid)?
                    .iter()
                    .map(|p| storage::backup_summary(p.paths()))
                    .collect::<Vec<_>>();
                return Ok(
                    json!({"total_count":rows.iter().filter_map(|r|r["count"].as_u64()).sum::<u64>(),"total_bytes":rows.iter().filter_map(|r|r["bytes"].as_u64()).sum::<u64>(),"providers":rows}),
                );
            }
            "delete_all_backups" => {
                confirm(&data)?;
                let ids = self
                    .targets(pid)?
                    .iter()
                    .map(|p| p.paths().id.clone())
                    .collect::<Vec<_>>();
                let service = self.clone();
                let job_ids = ids.clone();
                return self.start_job(ids, "delete_all_backups", move |ctx| {
                    ctx.phase("cut");
                    let mut rows = Vec::new();
                    let mut deleted = 0u64;
                    let mut freed = 0u64;
                    for id in job_ids {
                        let p = service.provider(&id)?;
                        let r = storage::delete_all_backups(p.paths())?;
                        deleted += r["deleted_count"].as_u64().unwrap_or(0);
                        freed += r["freed_bytes"].as_u64().unwrap_or(0);
                        if r["deleted_count"].as_u64().unwrap_or(0) > 0 {
                            rows.push(r);
                        }
                    }
                    Ok(json!({"breakdown":rows,"total_deleted":deleted,"freed_bytes":freed}))
                });
            }
            _ => {}
        }
        let p = self.provider(pid)?;
        if let Some(cid) = data["cid"].as_str() {
            valid_id(cid)?;
        }
        if matches!(op, "preview_cut" | "apply_cut")
            && !matches!(data["mode"].as_str().unwrap_or("tail"), "head" | "tail")
        {
            bail!("Недопустимый режим обрезки.");
        }
        if matches!(op, "add_system_notice" | "delete_system_notices") && pid != "claude" {
            bail!("Системные уведомления доступны только для диалогов Claude.");
        }
        if op == "convert_chat" {
            let cid = required(&data, "cid")?.to_owned();
            let dst = self.provider(required(&data, "dst_provider")?)?;
            dst.assert_writable(None)?;
            let ids = vec![pid.to_owned(), dst.paths().id.clone()];
            let job_provider = dst.paths().id.clone();
            return self.start_job_as(ids, "convert_chat", job_provider, move |ctx| {
                dst.assert_writable(None)?;
                converter::convert_chat(
                    p.as_ref(),
                    &cid,
                    dst.as_ref(),
                    data["title"].as_str().filter(|s| !s.trim().is_empty()),
                    data["include_tools"].as_bool().unwrap_or(false),
                    ctx,
                )
            });
        }
        if op == "delete_backup" {
            confirm(&data)?;
            let bdir = required(&data, "backup_dir")?.to_owned();
            storage::backup_path(&p.paths().backup_root, &bdir)?;
            return self.start_job(vec![pid.into()], "delete_backup", move |ctx| {
                ctx.phase("cut");
                storage::delete_backup(p.paths(), &bdir)
            });
        }
        if op == "search_thread" {
            let cid = required(&data, "cid")?;
            let raw_q = data["query"].as_str().unwrap_or("").trim();
            if raw_q.is_empty() {
                return Ok(json!({"query": raw_q, "total": 0, "matches": []}));
            }
            let q = raw_q.to_lowercase();
            let entries = collect_entries(p.as_ref(), cid, &self.context(), 8);
            let mut matches = Vec::new();
            for entry in &entries {
                let text = entry["text"].as_str().unwrap_or("");
                let label = entry["label"].as_str().unwrap_or("");
                let snippet = match_snippet(text, &q).or_else(|| {
                    if !label.is_empty() && label.to_lowercase().contains(&q) {
                        Some(trim_chars(label, 140))
                    } else {
                        None
                    }
                });
                if let Some(snip) = snippet {
                    matches.push(json!({
                        "key": entry["key"],
                        "kind": entry["kind"],
                        "subtype": entry["subtype"],
                        "label": entry["label"],
                        "ts": entry["ts"],
                        "snippet": snip
                    }));
                    if matches.len() >= 250 {
                        break;
                    }
                }
            }
            return Ok(json!({"query": raw_q, "total": matches.len(), "matches": matches}));
        }
        if op == "context_stats" {
            let cid = required(&data, "cid")?;
            let ctx = self.context();
            let entries = collect_entries(p.as_ref(), cid, &ctx, 8);
            let mut total_bytes = 0usize;
            let mut dialogue_bytes = 0usize;
            let mut dialogue_steps = 0usize;
            let mut thinking_bytes = 0usize;
            let mut thinking_steps = 0usize;
            let mut tool_bytes = 0usize;
            let mut tool_steps = 0usize;
            let mut ranked: Vec<(usize, Value)> = Vec::new();
            for entry in &entries {
                let text = entry["text"].as_str().unwrap_or("");
                let kind = entry["kind"].as_str().unwrap_or("");
                let subtype = entry["subtype"].as_str().unwrap_or("");
                let label = entry["label"].as_str().unwrap_or("");
                let bytes = text.len();
                total_bytes += bytes;
                if matches!(kind, "user" | "assistant") {
                    dialogue_bytes += bytes;
                    dialogue_steps += 1;
                } else if subtype == "thinking" || label.eq_ignore_ascii_case("reasoning") {
                    thinking_bytes += bytes;
                    thinking_steps += 1;
                } else if kind == "tool" {
                    tool_bytes += bytes;
                    tool_steps += 1;
                }
                if bytes >= 512 {
                    ranked.push((
                        bytes,
                        json!({
                            "key": entry["key"],
                            "kind": kind,
                            "subtype": subtype,
                            "label": label,
                            "bytes": bytes,
                            "tokens": (bytes + 3) / 4,
                            "preview": trim_chars(&text.split_whitespace().collect::<Vec<_>>().join(" "), 110)
                        }),
                    ));
                }
            }
            ranked.sort_by(|a, b| b.0.cmp(&a.0));
            let heaviest: Vec<Value> = ranked.into_iter().take(5).map(|(_, v)| v).collect();
            let can_slim = matches!(pid, "claude" | "codex" | "opencode");
            return Ok(json!({
                "provider": pid,
                "cid": cid,
                "total_steps": entries.len(),
                "total_bytes": total_bytes,
                "total_tokens": (total_bytes + 3) / 4,
                "dialogue_steps": dialogue_steps,
                "dialogue_tokens": (dialogue_bytes + 3) / 4,
                "thinking_steps": thinking_steps,
                "thinking_bytes": thinking_bytes,
                "thinking_tokens": (thinking_bytes + 3) / 4,
                "tool_steps": tool_steps,
                "tool_bytes": tool_bytes,
                "tool_tokens": (tool_bytes + 3) / 4,
                "can_slim": can_slim,
                "heaviest": heaviest
            }));
        }
        if op == "preview_slim" && !matches!(pid, "claude" | "codex" | "opencode") {
            return Ok(json!({
                "can_slim": false,
                "thinking_count": 0,
                "thinking_bytes": 0,
                "thinking_tokens": 0,
                "heavy_tool_count": 0,
                "heavy_tool_bytes": 0,
                "heavy_tool_tokens": 0,
                "total_affected": 0,
                "freed_bytes": 0,
                "freed_tokens": 0
            }));
        }
        if matches!(
            op,
            "apply_edit"
                | "apply_cut"
                | "apply_slim"
                | "delete_single"
                | "delete_conv"
                | "restore"
                | "unlock_chat"
                | "lock_chat"
                | "add_system_notice"
                | "delete_system_notices"
        ) {
            if !matches!(op, "unlock_chat" | "lock_chat") {
                confirm(&data)?;
            }
            if op == "apply_edit" && data["text"].as_str().unwrap_or("").trim().is_empty() {
                bail!("Пустой текст, сохранение отменено.");
            }
            if op == "lock_chat" && data["message"].as_str().unwrap_or("").trim().is_empty() {
                bail!("Укажите причину блокировки.");
            }
            if op == "add_system_notice"
                && data["notice_type"] != "safety_classifier"
                && data["text"].as_str().unwrap_or("").trim().is_empty()
            {
                bail!("Укажите текст системного уведомления.");
            }
            if matches!(op, "add_system_notice" | "delete_system_notices") {
                required(&data, "revision")?;
            }
            let target = if op == "restore" {
                let b =
                    storage::backup_path(&p.paths().backup_root, required(&data, "backup_dir")?)?;
                b.parent()
                    .and_then(Path::file_name)
                    .map(|s| s.to_string_lossy().into_owned())
            } else {
                Some(required(&data, "cid")?.to_owned())
            };
            p.assert_writable(target.as_deref())?;
            let name = match op {
                "apply_edit" => "edit",
                "apply_cut" => "cut",
                x => x,
            };
            let op = op.to_owned();
            return self.start_job(vec![pid.into()], name, move |ctx| {
                p.assert_writable(target.as_deref())?;
                p.call(&op, &data, ctx)
            });
        }
        if op == "entries" {
            *self.view.lock().unwrap_or_else(|e| e.into_inner()) =
                Some((pid.into(), required(&data, "cid")?.into()));
        }
        p.call(op, &data, &self.context())
    }

    fn targets(&self, id: &str) -> Result<Vec<Arc<dyn Provider>>> {
        if id == "all" {
            Ok(PROVIDER_ORDER
                .iter()
                .filter_map(|id| self.providers.get(*id).cloned())
                .collect())
        } else {
            Ok(vec![self.provider(id)?])
        }
    }

    fn start_job<F>(self: &Arc<Self>, ids: Vec<String>, name: &str, work: F) -> Result<Value>
    where
        F: FnOnce(&OpContext) -> Result<Value> + Send + 'static,
    {
        let job_provider = if ids.len() == 1 {
            ids[0].clone()
        } else {
            "all".into()
        };
        self.start_job_as(ids, name, job_provider, work)
    }

    fn start_job_as<F>(
        self: &Arc<Self>,
        ids: Vec<String>,
        name: &str,
        job_provider: String,
        work: F,
    ) -> Result<Value>
    where
        F: FnOnce(&OpContext) -> Result<Value> + Send + 'static,
    {
        let id = uuid::Uuid::new_v4().simple().to_string()[..12].to_owned();
        let job = json!({"id":id,"provider":job_provider,"name":name,"state":"running","phase":"start","result":null,"error":null});
        let shared = Arc::new(Mutex::new(job));
        let sink = self.emit.clone();
        let progress = shared.clone();
        let ctx = OpContext {
            auto_backup: self.context().auto_backup,
            on_phase: Arc::new(move |phase| {
                let mut j = progress.lock().unwrap_or_else(|e| e.into_inner());
                j["phase"] = json!(phase);
                (sink)(json!({"type":"job","job":*j}));
            }),
        };
        let service = self.clone();
        self.active_jobs.fetch_add(1, Ordering::SeqCst);
        thread::spawn(move || {
            let started = Instant::now();
            (service.emit)(
                json!({"type":"job","job":*shared.lock().unwrap_or_else(|e|e.into_inner())}),
            );
            let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let sorted: BTreeSet<_> = ids.into_iter().collect();
                let _guards = sorted
                    .iter()
                    .filter_map(|id| service.locks.get(id))
                    .map(|m| m.lock().unwrap_or_else(|e| e.into_inner()))
                    .collect::<Vec<_>>();
                work(&ctx)
            }));
            let mut job = shared.lock().unwrap_or_else(|e| e.into_inner());
            match outcome {
                Ok(Ok(result)) => {
                    job["state"] = json!("done");
                    job["phase"] = json!("done");
                    job["result"] = result;
                }
                Ok(Err(err)) => {
                    job["state"] = json!("error");
                    job["error"] = json!(format!("{err:#}"));
                }
                Err(_) => {
                    job["state"] = json!("error");
                    job["error"] = json!("Внутренняя ошибка операции. Проверьте резервную копию.");
                }
            }
            job["ms"] = json!(started.elapsed().as_millis() as u64);
            (service.emit)(json!({"type":"job","job":*job}));
            service.active_jobs.fetch_sub(1, Ordering::SeqCst);
        });
        Ok(json!({"job_id":id}))
    }

    pub fn export_pack(&self, data: &Value) -> Result<Value> {
        let p = self.provider(required(data, "provider")?)?;
        let cid = required(data, "cid")?;
        valid_id(cid)?;
        let _guard = self.locks[&p.paths().id]
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let (bytes, stats, filename) = converter::export_to_k1tty_bytes(
            p.as_ref(),
            cid,
            data["preset"].as_str().unwrap_or("balanced"),
            data["include_tools"].as_bool().unwrap_or(true),
        )?;
        let token = uuid::Uuid::new_v4().simple().to_string();
        let mut cache = self.downloads.lock().unwrap_or_else(|e| e.into_inner());
        prune(&mut cache);
        cache.insert(
            token.clone(),
            CachedPack {
                bytes,
                filename: filename.clone(),
                created: Instant::now(),
            },
        );
        Ok(json!({"ok":true,"token":token,"filename":filename,"stats":stats}))
    }

    pub fn save_export(&self, token: &str, path: &Path) -> Result<()> {
        let _active = self.active_operation();
        let cache = self.downloads.lock().unwrap_or_else(|e| e.into_inner());
        let data = cache
            .get(token)
            .context("Экспорт устарел. Экспортируйте диалог ещё раз.")?;
        storage::atomic_write(path, &data.bytes)
    }

    pub fn export_filename(&self, token: &str) -> Result<String> {
        self.downloads
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(token)
            .map(|v| v.filename.clone())
            .context("Экспорт устарел.")
    }

    pub fn inspect_pack(&self, bytes: Vec<u8>) -> Result<Value> {
        if bytes.is_empty() {
            bail!("Пустой файл.");
        }
        if bytes.len() > 512 * 1024 * 1024 {
            bail!("Архив превышает допустимый размер 512 МиБ.");
        }
        let info = pack::inspect_k1tty(&bytes)?;
        let token = uuid::Uuid::new_v4().simple().to_string();
        let mut cache = self.uploads.lock().unwrap_or_else(|e| e.into_inner());
        prune(&mut cache);
        cache.insert(
            token.clone(),
            CachedPack {
                bytes,
                filename: String::new(),
                created: Instant::now(),
            },
        );
        Ok(json!({"ok":true,"info":info,"token":token}))
    }

    pub fn import_pack(
        &self,
        token: &str,
        target: &str,
        title: Option<&str>,
        include_tools: bool,
    ) -> Result<Value> {
        let _active = self.active_operation();
        let p = self.provider(target)?;
        p.assert_writable(None)?;
        let _guard = self.locks[target].lock().unwrap_or_else(|e| e.into_inner());
        p.assert_writable(None)?;
        let cache = self.uploads.lock().unwrap_or_else(|e| e.into_inner());
        let item = cache
            .get(token)
            .context("Файл импорта устарел. Выберите его ещё раз.")?;
        let result = converter::import_k1tty_bytes(
            &item.bytes,
            p.as_ref(),
            title.filter(|s| !s.trim().is_empty()),
            include_tools,
        )?;
        Ok(json!({"ok":true,"result":result}))
    }

    pub fn start_watcher(self: &Arc<Self>) {
        let weak = Arc::downgrade(self);
        thread::spawn(move || {
            let mut previous = process::ProcessMap::new();
            let mut previous_lock = None;
            loop {
                thread::sleep(Duration::from_secs(1));
                let Some(service) = weak.upgrade() else {
                    break;
                };
                let current = process::snapshot_force(true);
                if current != previous {
                    let mut states = Map::new();
                    for (id, p) in &service.providers {
                        if current.get(id) != previous.get(id) {
                            if let Ok(s) = p.call("state", &json!({}), &service.context()) {
                                states.insert(id.clone(), s);
                            }
                        }
                    }
                    if !states.is_empty() {
                        (service.emit)(json!({"type":"procs","providers":states}));
                    }
                    previous = current;
                }
                let view = service
                    .view
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .clone();
                if let Some((id, cid)) = view {
                    if let Some(lock) = service.locks.get(&id) {
                        if let Ok(_guard) = lock.try_lock() {
                            if let Ok(p) = service.provider(&id) {
                                if let Ok(reason) = p.probe_lock(Some(&cid)) {
                                    let next = (id.clone(), cid.clone(), reason.clone());
                                    if previous_lock.as_ref() != Some(&next) {
                                        (service.emit)(
                                            json!({"type":"conv_lock","provider":id,"cid":cid,"locked":reason.is_some(),"reason":reason.unwrap_or_default()}),
                                        );
                                        previous_lock = Some(next);
                                    }
                                }
                            }
                        }
                    }
                } else {
                    previous_lock = None;
                }
            }
        });
    }
}

fn confirm(v: &Value) -> Result<()> {
    if v["confirm"] != true {
        bail!("Требуется подтверждение.");
    }
    Ok(())
}
fn prune(cache: &mut BTreeMap<String, CachedPack>) {
    cache.retain(|_, v| v.created.elapsed() < Duration::from_secs(300));
    while cache.len() >= 5
        || cache.values().map(|p| p.bytes.len()).sum::<usize>() > 512 * 1024 * 1024
    {
        if let Some(old) = cache
            .iter()
            .min_by_key(|(_, v)| v.created)
            .map(|(k, _)| k.clone())
        {
            cache.remove(&old);
        } else {
            break;
        }
    }
}

fn collect_entries(p: &dyn Provider, cid: &str, ctx: &OpContext, max_pages: usize) -> Vec<Value> {
    let mut pages: Vec<Vec<Value>> = Vec::new();
    let mut before: Option<String> = None;
    for _ in 0..max_pages {
        let mut req = json!({"cid": cid, "include_tools": true, "limit": 1500});
        if let Some(cur) = &before {
            req["before"] = json!(cur);
        }
        let Ok(resp) = p.call("entries", &req, ctx) else {
            break;
        };
        let mut items = Vec::new();
        if let Some(turns) = resp["turns"].as_array() {
            for turn in turns {
                if let Some(arr) = turn["items"].as_array() {
                    items.extend(arr.iter().cloned());
                }
            }
        } else if let Some(arr) = resp["entries"].as_array() {
            items.extend(arr.iter().cloned());
        }
        if items.is_empty() {
            break;
        }
        pages.push(items);
        if resp["has_more"] != true {
            break;
        }
        let next = resp["cursor"]
            .as_str()
            .or_else(|| resp["next_cursor"].as_str())
            .unwrap_or("")
            .to_owned();
        if next.is_empty() || before.as_deref() == Some(next.as_str()) {
            break;
        }
        before = Some(next);
    }
    pages.reverse();
    pages.into_iter().flatten().collect()
}

fn trim_chars(s: &str, max: usize) -> String {
    let mut out = String::new();
    for (i, ch) in s.chars().enumerate() {
        if i >= max {
            out.push('…');
            break;
        }
        out.push(ch);
    }
    out
}

fn match_snippet(text: &str, needle_lower: &str) -> Option<String> {
    if text.is_empty() || needle_lower.is_empty() {
        return None;
    }
    let flat = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let chars: Vec<char> = flat.chars().collect();
    let needle: Vec<char> = needle_lower.chars().collect();
    if needle.len() > chars.len() {
        return None;
    }
    let lowered: Vec<char> = chars
        .iter()
        .map(|c| c.to_lowercase().next().unwrap_or(*c))
        .collect();
    let pos = lowered
        .windows(needle.len())
        .position(|w| w == needle.as_slice())?;
    let start = pos.saturating_sub(42);
    let end = (pos + needle.len() + 72).min(chars.len());
    let mut out = String::new();
    if start > 0 {
        out.push('…');
    }
    for ch in &chars[start..end] {
        out.push(*ch);
    }
    if end < chars.len() {
        out.push('…');
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn context_stats_and_fulltext_search_work_end_to_end() {
        let dir = tempfile::tempdir().unwrap();
        let paths = AppPaths::isolated(dir.path());
        let proj = paths
            .providers
            .iter()
            .find(|p| p.id == "claude")
            .unwrap()
            .root
            .join("projects")
            .join("demo-proj");
        std::fs::create_dir_all(&proj).unwrap();
        let session = proj.join("sess-search.jsonl");
        let rows = [
            json!({"type":"user","uuid":"u1","parentUuid":null,"timestamp":"2026-04-01T10:00:00Z","message":{"role":"user","content":"Find unique_needle_alpha in codebase"}}),
            json!({"type":"assistant","uuid":"u2","parentUuid":"u1","timestamp":"2026-04-01T10:00:01Z","message":{"role":"assistant","content":[{"type":"thinking","thinking":"deep reasoning about unique_needle_alpha"},{"type":"text","text":"Located unique_needle_alpha inside service.rs"}]}}),
        ];
        let body = rows
            .iter()
            .map(|r| serde_json::to_string(r).unwrap())
            .collect::<Vec<_>>()
            .join("\n")
            + "\n";
        std::fs::write(&session, body).unwrap();

        let svc = Service::new(paths, Arc::new(|_| {})).unwrap();
        let stats = svc
            .rpc(
                "context_stats",
                Some("claude"),
                json!({"cid":"sess-search"}),
            )
            .unwrap();
        assert_eq!(stats["can_slim"], true);
        assert!(stats["total_tokens"].as_u64().unwrap() > 10);
        assert_eq!(stats["thinking_steps"], 1);
        assert!(stats["thinking_tokens"].as_u64().unwrap() > 0);

        let local = svc
            .rpc(
                "search_thread",
                Some("claude"),
                json!({"cid":"sess-search","query":"unique_needle_alpha"}),
            )
            .unwrap();
        assert!(local["total"].as_u64().unwrap() >= 2);
        assert!(!local["matches"][0]["key"].as_str().unwrap_or("").is_empty());

        let global = svc
            .rpc(
                "search_all",
                Some("all"),
                json!({"query":"unique_needle_alpha"}),
            )
            .unwrap();
        let hits = global["results"].as_array().unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0]["provider"], "claude");
        assert_eq!(hits[0]["cid"], "sess-search");
        assert!(hits[0]["snippet"]
            .as_str()
            .unwrap()
            .contains("unique_needle_alpha"));
    }
}
