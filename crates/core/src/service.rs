use crate::{
    converter, deepseek, pack, process,
    provider::{floor_char_boundary, required, valid_id, OpContext, Provider, ProviderPaths},
    storage,
};
use anyhow::{bail, Context, Result};
use serde_json::{json, Map, Value};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap, VecDeque},
    fs,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, AtomicUsize, Ordering},
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
    data_dir: PathBuf,
    settings_path: PathBuf,
    emit: EventSink,
    view: Mutex<Option<(String, String)>>,
    downloads: Mutex<BTreeMap<String, CachedPack>>,
    uploads: Mutex<BTreeMap<String, CachedPack>>,
    active_jobs: AtomicUsize,
    thread_search_gen: AtomicU64,
    global_search_gen: AtomicU64,
    context_stats_cache: Arc<Mutex<HashMap<(String, String), (String, Value)>>>,
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
        let saved_settings = storage::read_json(&settings_path).ok();
        let auto_backup = saved_settings
            .as_ref()
            .and_then(|v| v["auto_backup"].as_bool())
            .unwrap_or(true);
        let auto_restart_hub = saved_settings
            .as_ref()
            .and_then(|v| v["auto_restart_hub"].as_bool())
            .unwrap_or(false);
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
            instance.set_auto_restart_hub(auto_restart_hub);
            providers.insert(id, instance);
        }
        let locks = providers
            .keys()
            .map(|id| (id.clone(), Mutex::new(())))
            .collect();
        Ok(Arc::new(Self {
            providers,
            locks,
            settings: Mutex::new(
                json!({"auto_backup":auto_backup,"auto_restart_hub":auto_restart_hub}),
            ),
            data_dir: paths.data_dir,
            settings_path,
            emit,
            view: Mutex::new(None),
            downloads: Mutex::new(BTreeMap::new()),
            uploads: Mutex::new(BTreeMap::new()),
            active_jobs: AtomicUsize::new(0),
            thread_search_gen: AtomicU64::new(0),
            global_search_gen: AtomicU64::new(0),
            context_stats_cache: Arc::new(Mutex::new(HashMap::new())),
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
        let s = self.settings();
        OpContext {
            auto_backup: s["auto_backup"].as_bool().unwrap_or(true),
            auto_restart_hub: s["auto_restart_hub"].as_bool().unwrap_or(false),
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
                let hub_changed = data["auto_restart_hub"].as_bool();
                if let Some(v) = hub_changed {
                    settings["auto_restart_hub"] = json!(v);
                    for prov in self.providers.values() {
                        prov.set_auto_restart_hub(v);
                    }
                }
                storage::atomic_write(
                    &self.settings_path,
                    &serde_json::to_vec_pretty(&*settings)?,
                )?;
                let snap = settings.clone();
                drop(settings);
                (self.emit)(json!({"type":"settings","settings":snap}));
                if hub_changed.is_some() {
                    let view = self.view.lock().unwrap_or_else(|e| e.into_inner()).clone();
                    if let Some((view_pid, view_cid)) = view {
                        if let Ok(prov) = self.provider(&view_pid) {
                            if let Ok(mut status) = prov.lock_status(Some(&view_cid)) {
                                if let Some(obj) = status.as_object_mut() {
                                    obj.insert("type".into(), json!("conv_lock"));
                                    obj.insert("provider".into(), json!(view_pid));
                                    obj.insert("cid".into(), json!(view_cid));
                                    (self.emit)(status);
                                }
                            }
                        }
                    }
                    let ctx = self.context();
                    let mut delta = Map::new();
                    for ag_id in ["antigravity", "antigravity_cli"] {
                        if let Ok(prov) = self.provider(ag_id) {
                            if let Ok(st) = prov.call("state", &json!({}), &ctx) {
                                delta.insert(ag_id.into(), st);
                            }
                        }
                    }
                    if !delta.is_empty() {
                        (self.emit)(json!({"type":"procs","providers":delta}));
                    }
                }
                return Ok(json!({"settings":snap}));
            }
            "deepseek_config" | "generator_config" => {
                return Ok(deepseek::public_config(&self.data_dir))
            }
            "deepseek_save_config" | "generator_save_config" => {
                return deepseek::save_config(&self.data_dir, &data)
            }
            "deepseek_models" | "generator_models" => {
                return deepseek::fetch_models(&self.data_dir, &data);
            }
            "deepseek_generate" | "generator_generate" => {
                let _op = self.active_operation();
                return deepseek::generate_dialogue(&self.data_dir, &data);
            }
            "generator_drafts" => return deepseek::list_drafts(&self.data_dir),
            "generator_load_draft" => {
                return deepseek::load_draft(&self.data_dir, required(&data, "id")?);
            }
            "generator_save_draft" => return deepseek::save_draft(&self.data_dir, &data),
            "generator_delete_draft" => {
                return deepseek::delete_draft(&self.data_dir, required(&data, "id")?);
            }
            "deepseek_save_chat" | "generator_save_chat" => {
                let target_id = data["target"]
                    .as_str()
                    .filter(|s| !s.is_empty())
                    .unwrap_or(pid);
                let dst = self.provider(target_id)?;
                dst.assert_writable(None)?;
                let title = data["title"].as_str().filter(|s| !s.trim().is_empty());
                let include_tools = data["include_tools"].as_bool().unwrap_or(true);
                let doc = deepseek::prepare_doc_for_save(&data["doc"], title, include_tools)?;
                let _op = self.active_operation();
                let _guard = self.locks[target_id]
                    .lock()
                    .unwrap_or_else(|e| e.into_inner());
                let result =
                    converter::import_doc_to_provider(&doc, dst.as_ref(), title, include_tools)?;
                if let Some(cid) = result["cid"].as_str().filter(|s| !s.is_empty()) {
                    let _ = dst.after_external_import(cid, &self.context());
                }
                return Ok(json!({"ok":true,"result":result}));
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
                let whole_word = data["whole_word"].as_bool().unwrap_or(false);
                let query = SearchQuery::with_options(raw_q, whole_word);
                if query.is_empty() {
                    return Ok(json!({"query": raw_q, "results": []}));
                }
                let search_gen = self.global_search_gen.fetch_add(1, Ordering::Relaxed) + 1;
                let limit = data["limit"].as_u64().unwrap_or(40).clamp(1, 100) as usize;
                let ctx = self.context();
                let mut results = Vec::new();
                for p in self.targets(pid)? {
                    if results.len() >= limit
                        || self.global_search_gen.load(Ordering::Relaxed) != search_gen
                    {
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
                        if results.len() >= limit
                            || self.global_search_gen.load(Ordering::Relaxed) != search_gen
                        {
                            break;
                        }
                        let cid = conv["id"].as_str().unwrap_or("");
                        if cid.is_empty() {
                            continue;
                        }
                        let title = conv["title"].as_str().unwrap_or("");
                        let mtime = conv["mtime"].as_str().unwrap_or("");
                        let mtime_ts = conv["mtime_ts"].as_f64().unwrap_or(0.0);
                        let title_hit = query.matches(title);
                        let mut hit_count = 0usize;
                        let mut first_key = String::new();
                        let mut first_kind = String::new();
                        let mut first_snippet: Option<String> = None;
                        let mut aborted = false;
                        let _ = p.visit_entries(cid, &mut |entry| {
                            if self.global_search_gen.load(Ordering::Relaxed) != search_gen {
                                aborted = true;
                                return false;
                            }
                            if first_snippet.is_none() {
                                if let Some(snip) = query.snippet(entry.text) {
                                    hit_count += 1;
                                    first_key = entry.key.to_owned();
                                    first_kind = entry.kind.to_owned();
                                    first_snippet = Some(snip);
                                } else if !entry.label.is_empty() && query.matches(entry.label) {
                                    hit_count += 1;
                                    first_key = entry.key.to_owned();
                                    first_kind = entry.kind.to_owned();
                                    first_snippet = Some(trim_chars(entry.label, 140));
                                }
                            } else if query.matches(entry.text)
                                || (!entry.label.is_empty() && query.matches(entry.label))
                            {
                                hit_count += 1;
                            }
                            true
                        });
                        if aborted {
                            return Ok(json!({"query": raw_q, "results": []}));
                        }
                        if hit_count > 0 || title_hit {
                            let snippet = first_snippet.unwrap_or_else(|| trim_chars(title, 140));
                            results.push(json!({
                                "provider": id,
                                "cid": cid,
                                "title": title,
                                "mtime": mtime,
                                "mtime_ts": mtime_ts,
                                "key": first_key,
                                "kind": first_kind,
                                "snippet": snippet,
                                "hit_count": hit_count.max(usize::from(title_hit))
                            }));
                        }
                    }
                }
                if self.global_search_gen.load(Ordering::Relaxed) != search_gen {
                    return Ok(json!({"query": raw_q, "results": []}));
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
        if matches!(op, "unlock_db" | "relock_db" | "restart_hub") {
            if !matches!(pid, "antigravity" | "antigravity_cli") {
                bail!("Живой анлок БД поддерживается только для Antigravity.");
            }
            let _guard = self.locks[pid].lock().unwrap_or_else(|e| e.into_inner());
            let res = p.call(op, &data, &self.context())?;
            if let Some(cid) = data["cid"].as_str().filter(|s| !s.is_empty()) {
                if let Ok(mut status) = p.lock_status(Some(cid)) {
                    if let Some(obj) = status.as_object_mut() {
                        obj.insert("type".into(), json!("conv_lock"));
                        obj.insert("provider".into(), json!(pid));
                        obj.insert("cid".into(), json!(cid));
                        (self.emit)(status);
                    }
                }
            }
            if op == "restart_hub" {
                if let Ok(st) = p.call("state", &json!({}), &self.context()) {
                    let mut delta = Map::new();
                    delta.insert(pid.into(), st);
                    (self.emit)(json!({"type":"procs","providers":delta}));
                }
            }
            return Ok(res);
        }
        if op == "convert_chat" {
            let cid = required(&data, "cid")?.to_owned();
            let dst = self.provider(required(&data, "dst_provider")?)?;
            dst.assert_writable(None)?;
            let ids = vec![pid.to_owned(), dst.paths().id.clone()];
            let job_provider = dst.paths().id.clone();
            return self.start_job_as(ids, "convert_chat", job_provider, move |ctx| {
                dst.assert_writable(None)?;
                let result = converter::convert_chat(
                    p.as_ref(),
                    &cid,
                    dst.as_ref(),
                    data["title"].as_str().filter(|s| !s.trim().is_empty()),
                    data["include_tools"].as_bool().unwrap_or(false),
                    ctx,
                )?;
                if let Some(new_cid) = result["cid"].as_str().filter(|s| !s.is_empty()) {
                    let _ = dst.after_external_import(new_cid, ctx);
                }
                Ok(result)
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
        if op == "entry_text" {
            let cid = required(&data, "cid")?;
            let key = required(&data, "key")?;
            let text = match p.read_entry_text(cid, key)? {
                Some(text) => text,
                None => {
                    let preview = p.call(
                        "edit_preview",
                        &json!({"cid": cid, "key": key}),
                        &self.context(),
                    )?;
                    preview["text"]
                        .as_str()
                        .context("Не удалось прочитать полный текст записи.")?
                        .to_owned()
                }
            };
            return Ok(json!({"ok": true, "key": key, "text": text}));
        }
        if op == "search_thread" {
            let cid = required(&data, "cid")?;
            let raw_q = data["query"].as_str().unwrap_or("").trim();
            let include_tools = data["include_tools"].as_bool().unwrap_or(true);
            let whole_word = data["whole_word"].as_bool().unwrap_or(false);
            let query = SearchQuery::with_options(raw_q, whole_word);
            if query.is_empty() {
                return Ok(json!({
                    "query": raw_q,
                    "total": 0,
                    "shown": 0,
                    "truncated": false,
                    "matches": []
                }));
            }
            let search_gen = self.thread_search_gen.fetch_add(1, Ordering::Relaxed) + 1;
            let mut matches = VecDeque::with_capacity(250);
            let mut total_matches = 0usize;
            let mut aborted = false;
            p.visit_entries(cid, &mut |entry| {
                if self.thread_search_gen.load(Ordering::Relaxed) != search_gen {
                    aborted = true;
                    return false;
                }
                if !include_tools && entry.kind == "tool" {
                    return true;
                }
                let snippet = query.snippet(entry.text).or_else(|| {
                    if !entry.label.is_empty() && query.matches(entry.label) {
                        Some(trim_chars(entry.label, 140))
                    } else {
                        None
                    }
                });
                if let Some(snip) = snippet {
                    total_matches += 1;
                    if matches.len() == 250 {
                        matches.pop_front();
                    }
                    matches.push_back(json!({
                        "key": entry.key,
                        "kind": entry.kind,
                        "subtype": entry.subtype,
                        "label": entry.label,
                        "ts": entry.ts,
                        "snippet": snip
                    }));
                }
                true
            })?;
            if aborted || self.thread_search_gen.load(Ordering::Relaxed) != search_gen {
                return Ok(json!({
                    "query": raw_q,
                    "total": 0,
                    "shown": 0,
                    "truncated": false,
                    "matches": []
                }));
            }
            let shown = matches.len();
            let out: Vec<Value> = matches.into_iter().collect();
            return Ok(json!({
                "query": raw_q,
                "total": total_matches,
                "shown": shown,
                "truncated": total_matches > shown,
                "matches": out
            }));
        }
        if op == "context_stats" {
            let cid = required(&data, "cid")?;
            let sig = p.content_signature(cid);
            if let Some(s) = sig.as_deref() {
                let cache = self
                    .context_stats_cache
                    .lock()
                    .unwrap_or_else(|e| e.into_inner());
                if let Some((cached_sig, cached_val)) = cache.get(&(pid.to_owned(), cid.to_owned()))
                {
                    if cached_sig == s {
                        return Ok(cached_val.clone());
                    }
                }
            }
            let mut total_steps = 0usize;
            let mut total_bytes = 0usize;
            let mut dialogue_bytes = 0usize;
            let mut dialogue_steps = 0usize;
            let mut thinking_bytes = 0usize;
            let mut thinking_steps = 0usize;
            let mut tool_bytes = 0usize;
            let mut tool_steps = 0usize;
            let mut top5: Vec<(usize, Value)> = Vec::with_capacity(6);
            p.visit_entries(cid, &mut |entry| {
                total_steps += 1;
                let bytes = entry.text.len();
                total_bytes += bytes;
                if matches!(entry.kind, "user" | "assistant") {
                    dialogue_bytes += bytes;
                    dialogue_steps += 1;
                } else if entry.subtype == "thinking"
                    || entry.label.eq_ignore_ascii_case("reasoning")
                {
                    thinking_bytes += bytes;
                    thinking_steps += 1;
                } else if entry.kind == "tool" {
                    tool_bytes += bytes;
                    tool_steps += 1;
                }
                if bytes >= 512 && (top5.len() < 5 || bytes > top5.last().unwrap().0) {
                    let row = json!({
                        "key": entry.key,
                        "kind": entry.kind,
                        "subtype": entry.subtype,
                        "label": entry.label,
                        "bytes": bytes,
                        "tokens": (bytes + 3) / 4,
                        "preview": collapse_preview(entry.text, 110)
                    });
                    top5.push((bytes, row));
                    top5.sort_by(|a, b| b.0.cmp(&a.0));
                    if top5.len() > 5 {
                        top5.truncate(5);
                    }
                }
                true
            })?;
            let heaviest: Vec<Value> = top5.into_iter().map(|(_, v)| v).collect();
            let can_slim = matches!(pid, "claude" | "codex" | "opencode");
            let stats = json!({
                "provider": pid,
                "cid": cid,
                "total_steps": total_steps,
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
            });
            if let Some(s) = sig {
                let mut cache = self
                    .context_stats_cache
                    .lock()
                    .unwrap_or_else(|e| e.into_inner());
                if cache.len() >= 64 {
                    cache.clear();
                }
                cache.insert((pid.to_owned(), cid.to_owned()), (s, stats.clone()));
            }
            return Ok(stats);
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
            let stats_cache = self.context_stats_cache.clone();
            let pid_owned = pid.to_owned();
            return self.start_job(vec![pid.into()], name, move |ctx| {
                let res = p.call(&op, &data, ctx)?;
                if let Some(cid) = target {
                    stats_cache
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .remove(&(pid_owned, cid));
                }
                Ok(res)
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
        let base_ctx = self.context();
        let ctx = OpContext {
            auto_backup: base_ctx.auto_backup,
            auto_restart_hub: base_ctx.auto_restart_hub,
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
            let mut hub_restarted_provider: Option<String> = None;
            let mut job = shared.lock().unwrap_or_else(|e| e.into_inner());
            match outcome {
                Ok(Ok(result)) => {
                    if result["hub_restarted"].as_bool() == Some(true) {
                        hub_restarted_provider = job["provider"].as_str().map(str::to_owned);
                    }
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
            drop(job);
            if let Some(pid) = hub_restarted_provider {
                if let Ok(prov) = service.provider(&pid) {
                    if let Ok(st) = prov.call("state", &json!({}), &service.context()) {
                        let mut delta = Map::new();
                        delta.insert(pid, st);
                        (service.emit)(json!({"type":"procs","providers":delta}));
                    }
                }
            }
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
        if let Some(cid) = result["cid"].as_str().filter(|s| !s.is_empty()) {
            let _ = p.after_external_import(cid, &self.context());
        }
        Ok(json!({"ok":true,"result":result}))
    }

    pub fn start_watcher(self: &Arc<Self>) {
        let weak = Arc::downgrade(self);
        thread::spawn(move || {
            let mut previous = process::ProcessMap::new();
            let mut previous_lock = None;
            let mut previous_view: Option<(String, String)> = None;
            let mut tick: u64 = 0;
            loop {
                thread::sleep(Duration::from_millis(2500));
                tick = tick.wrapping_add(1);
                let Some(service) = weak.upgrade() else {
                    break;
                };
                let current = process::snapshot_force(true);
                let procs_changed = current != previous;
                if procs_changed {
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
                    let view_changed = previous_view
                        .as_ref()
                        .is_none_or(|(p, c)| p != &id || c != &cid);
                    let provider_running = previous.get(&id).is_some_and(|list| !list.is_empty());
                    let heartbeat = tick % 4 == 0;
                    if view_changed || procs_changed || provider_running || heartbeat {
                        if let Some(lock) = service.locks.get(&id) {
                            if let Ok(_guard) = lock.try_lock() {
                                if let Ok(p) = service.provider(&id) {
                                    if let Ok(mut status) = p.lock_status(Some(&cid)) {
                                        let next = (id.clone(), cid.clone(), status.clone());
                                        if previous_lock.as_ref() != Some(&next) {
                                            if let Some(obj) = status.as_object_mut() {
                                                obj.insert("type".into(), json!("conv_lock"));
                                                obj.insert("provider".into(), json!(id));
                                                obj.insert("cid".into(), json!(cid));
                                                (service.emit)(status);
                                            }
                                            previous_lock = Some(next);
                                        }
                                    }
                                }
                                previous_view = Some((id, cid));
                            }
                        }
                    }
                } else {
                    previous_lock = None;
                    previous_view = None;
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

fn collapse_preview(text: &str, max_chars: usize) -> String {
    let mut out = String::new();
    let mut count = 0usize;
    let mut first = true;
    for word in text.split_whitespace() {
        if !first {
            if count >= max_chars {
                out.push('…');
                return out;
            }
            out.push(' ');
            count += 1;
        }
        first = false;
        for ch in word.chars() {
            if count >= max_chars {
                out.push('…');
                return out;
            }
            out.push(ch);
            count += 1;
        }
    }
    out
}

struct SearchQuery {
    ascii_tokens: Option<Vec<Vec<u8>>>,
    unicode_tokens: Vec<Vec<char>>,
    first_char_lower: char,
    first_char_upper: Option<char>,
    whole_word: bool,
}

fn is_word_char(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_'
}

fn is_whole_word_match(text: &str, start: usize, end: usize) -> bool {
    if start > 0 {
        if let Some(prev) = text[..start].chars().next_back() {
            if is_word_char(prev) {
                return false;
            }
        }
    }
    if end < text.len() {
        if let Some(next) = text[end..].chars().next() {
            if is_word_char(next) {
                return false;
            }
        }
    }
    true
}

impl SearchQuery {
    fn new(raw_q: &str) -> Self {
        Self::with_options(raw_q, false)
    }

    fn with_options(raw_q: &str, whole_word: bool) -> Self {
        let mut unicode_tokens = Vec::new();
        for tok in raw_q.split_whitespace() {
            let chars: Vec<char> = tok.to_lowercase().chars().collect();
            if !chars.is_empty() {
                unicode_tokens.push(chars);
            }
        }
        if unicode_tokens.is_empty() {
            return Self {
                ascii_tokens: None,
                unicode_tokens,
                first_char_lower: '\0',
                first_char_upper: None,
                whole_word,
            };
        }
        let ascii_tokens = if unicode_tokens.iter().all(|t| t.iter().all(char::is_ascii)) {
            Some(
                unicode_tokens
                    .iter()
                    .map(|t| t.iter().map(|&c| c as u8).collect())
                    .collect(),
            )
        } else {
            None
        };
        let first_char_lower = unicode_tokens[0][0];
        let mut upper_it = first_char_lower.to_uppercase();
        let u1 = upper_it.next();
        let first_char_upper = if upper_it.next().is_none() { u1 } else { None };
        Self {
            ascii_tokens,
            unicode_tokens,
            first_char_lower,
            first_char_upper,
            whole_word,
        }
    }

    fn is_empty(&self) -> bool {
        self.unicode_tokens.is_empty()
    }

    fn find(&self, text: &str) -> Option<(usize, usize)> {
        if self.unicode_tokens.is_empty() || text.is_empty() {
            return None;
        }
        if let Some(tokens) = &self.ascii_tokens {
            let bytes = text.as_bytes();
            let first = &tokens[0];
            let mut search_from = 0usize;
            if tokens.len() == 1 {
                while let Some((start, end)) = find_ascii_token(bytes, first, search_from) {
                    if !self.whole_word || is_whole_word_match(text, start, end) {
                        return Some((start, end));
                    }
                    search_from = start + 1;
                }
                return None;
            }
            while let Some((start, mut cur)) = find_ascii_token(bytes, first, search_from) {
                let mut matched = true;
                for tok in &tokens[1..] {
                    let ws_start = cur;
                    while cur < bytes.len() {
                        let b = bytes[cur];
                        if b < 0x80 {
                            if b.is_ascii_whitespace() {
                                cur += 1;
                            } else {
                                break;
                            }
                        } else {
                            let Some(ch) = text[cur..].chars().next() else {
                                break;
                            };
                            if ch.is_whitespace() {
                                cur += ch.len_utf8();
                            } else {
                                break;
                            }
                        }
                    }
                    if cur == ws_start
                        || cur + tok.len() > bytes.len()
                        || !bytes[cur..cur + tok.len()].eq_ignore_ascii_case(tok)
                    {
                        matched = false;
                        break;
                    }
                    cur += tok.len();
                }
                if matched && (!self.whole_word || is_whole_word_match(text, start, cur)) {
                    return Some((start, cur));
                }
                search_from = start + 1;
            }
            return None;
        }
        let mut search_from = 0usize;
        while let Some((start, end)) = self.find_unicode_token(text, search_from) {
            if !self.whole_word || is_whole_word_match(text, start, end) {
                return Some((start, end));
            }
            let step = text[start..].chars().next().map_or(1, char::len_utf8);
            search_from = start + step;
        }
        None
    }

    fn find_unicode_token(&self, text: &str, from: usize) -> Option<(usize, usize)> {
        let bytes = text.as_bytes();
        if from >= bytes.len() {
            return None;
        }
        let first_lo = self.first_char_lower;
        let first_hi = self.first_char_upper;
        if first_lo.is_ascii() {
            let lo = first_lo as u8;
            let hi = lo.to_ascii_uppercase();
            let mut pos = from;
            while pos < bytes.len() {
                let rel = if lo == hi {
                    bytes[pos..].iter().position(|&b| b == lo)
                } else {
                    bytes[pos..].iter().position(|&b| b == lo || b == hi)
                }?;
                let idx = pos + rel;
                if let Some(end) = match_unicode_tokens_at(text, idx, &self.unicode_tokens) {
                    return Some((idx, end));
                }
                pos = idx + 1;
            }
            return None;
        }
        let mut i = from;
        while i < bytes.len() {
            if bytes[i] < 0x80 {
                let rel = bytes[i..].iter().position(|&b| b >= 0x80)?;
                i += rel;
            }
            let Some(ch) = text[i..].chars().next() else {
                break;
            };
            let clen = ch.len_utf8();
            let hit = ch == first_lo
                || first_hi == Some(ch)
                || (!ch.is_lowercase() && ch.to_lowercase().next() == Some(first_lo));
            if hit {
                if let Some(end) = match_unicode_tokens_at(text, i, &self.unicode_tokens) {
                    return Some((i, end));
                }
            }
            i += clen;
        }
        None
    }

    fn matches(&self, text: &str) -> bool {
        self.find(text).is_some()
    }

    fn snippet(&self, text: &str) -> Option<String> {
        let (match_start, match_end) = self.find(text)?;
        Some(build_snippet(text, match_start, match_end))
    }
}

fn find_ascii_token(hay: &[u8], needle: &[u8], from: usize) -> Option<(usize, usize)> {
    let nlen = needle.len();
    if nlen == 0 || from + nlen > hay.len() {
        return None;
    }
    let first_lo = needle[0];
    let first_hi = first_lo.to_ascii_uppercase();
    let last = hay.len() - nlen;
    let mut pos = from;
    if first_lo == first_hi {
        while let Some(rel) = hay[pos..=last].iter().position(|&b| b == first_lo) {
            let idx = pos + rel;
            if nlen == 1 || hay[idx + 1..idx + nlen].eq_ignore_ascii_case(&needle[1..]) {
                return Some((idx, idx + nlen));
            }
            pos = idx + 1;
        }
    } else {
        while let Some(rel) = hay[pos..=last]
            .iter()
            .position(|&b| b == first_lo || b == first_hi)
        {
            let idx = pos + rel;
            if nlen == 1 || hay[idx + 1..idx + nlen].eq_ignore_ascii_case(&needle[1..]) {
                return Some((idx, idx + nlen));
            }
            pos = idx + 1;
        }
    }
    None
}

fn match_unicode_tokens_at(text: &str, start: usize, tokens: &[Vec<char>]) -> Option<usize> {
    let mut iter = text[start..].char_indices().peekable();
    for (ti, tok) in tokens.iter().enumerate() {
        if ti > 0 {
            let mut saw_ws = false;
            while iter.peek().is_some_and(|(_, c)| c.is_whitespace()) {
                iter.next();
                saw_ws = true;
            }
            if !saw_ws {
                return None;
            }
        }
        let mut k = 0usize;
        while k < tok.len() {
            let (_, ch) = iter.next()?;
            if ch.is_whitespace() {
                return None;
            }
            if ch.is_ascii() {
                if (ch as u8).to_ascii_lowercase() as char != tok[k] {
                    return None;
                }
                k += 1;
            } else if ch == tok[k] {
                k += 1;
            } else {
                for lc in ch.to_lowercase() {
                    if k >= tok.len() || lc != tok[k] {
                        return None;
                    }
                    k += 1;
                }
            }
        }
    }
    let end = iter
        .peek()
        .map(|(off, _)| start + *off)
        .unwrap_or(text.len());
    Some(end)
}

fn build_snippet(text: &str, match_start: usize, match_end: usize) -> String {
    let win_start = floor_char_boundary(text, match_start.saturating_sub(256));
    let win_end = floor_char_boundary(text, match_end.saturating_add(400).min(text.len()));
    let window = &text[win_start..win_end];
    let rel_start = match_start - win_start;
    let rel_end = match_end - win_start;
    let mut chars = Vec::with_capacity(window.len().min(700));
    let mut in_ws = false;
    let mut started = false;
    let mut hit_start: Option<usize> = None;
    let mut hit_end: Option<usize> = None;
    for (off, ch) in window.char_indices() {
        if hit_start.is_none() && off >= rel_start {
            hit_start = Some(chars.len());
        }
        if hit_end.is_none() && off >= rel_end {
            hit_end = Some(chars.len());
        }
        if ch.is_whitespace() {
            if started && !in_ws {
                chars.push(' ');
                in_ws = true;
            }
        } else {
            started = true;
            in_ws = false;
            chars.push(ch);
        }
    }
    if chars.last() == Some(&' ') {
        chars.pop();
    }
    let h_start = hit_start.unwrap_or(0).min(chars.len());
    let h_end = hit_end.unwrap_or(chars.len()).min(chars.len());
    let start = h_start.saturating_sub(42);
    let end = (h_end + 72).min(chars.len());
    let has_before = start > 0 || text[..win_start].chars().rev().any(|c| !c.is_whitespace());
    let has_after = end < chars.len() || text[win_end..].chars().any(|c| !c.is_whitespace());
    let mut out = String::with_capacity((end - start) * 2 + 6);
    if has_before {
        out.push('…');
    }
    for &ch in &chars[start..end] {
        out.push(ch);
    }
    if has_after {
        out.push('…');
    }
    out
}

pub fn match_snippet(text: &str, needle_lower: &str) -> Option<String> {
    if text.is_empty() || needle_lower.is_empty() {
        return None;
    }
    SearchQuery::new(needle_lower).snippet(text)
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

    #[test]
    fn match_snippet_handles_ascii_cyrillic_multiword_and_giant_non_match() {
        assert_eq!(match_snippet("", "abc"), None);
        assert_eq!(match_snippet("abc", ""), None);
        assert_eq!(match_snippet("abc", "   "), None);

        let ascii_hit = match_snippet("Prefix\n\t  Alpha   BETA  gamma  suffix", "alpha beta")
            .expect("ascii multi-word hit");
        assert_eq!(ascii_hit, "Prefix Alpha BETA gamma suffix");

        let cyr_hit = match_snippet("Строка   один\n\tПРИВЕТ   Мир   конец", "привет мир")
            .expect("cyrillic multi-word hit");
        assert_eq!(cyr_hit, "Строка один ПРИВЕТ Мир конец");

        let mixed = match_snippet("run_command: Проверка   Статуса ok", "проверка статуса")
            .expect("mixed hit");
        assert!(mixed.contains("Проверка Статуса"));

        let mixed_ascii_first = match_snippet("Code K1TTY   Поиск   Тест done", "k1tty поиск")
            .expect("mixed ascii-first");
        assert!(mixed_ascii_first.contains("K1TTY Поиск"));

        let mut giant = "abcdefghij ".repeat(200_000);
        assert_eq!(match_snippet(&giant, "zz_not_present_zz"), None);
        assert_eq!(match_snippet(&giant, "zz_not present"), None);
        assert_eq!(match_snippet(&giant, "кириллица отсутствует"), None);

        let cyr_pad = "ж".repeat(300);
        let bounded = format!("{cyr_pad} ЦЕЛЕВОЙ   МАРКЕР {cyr_pad}");
        let snip = match_snippet(&bounded, "целевой маркер").expect("bounded cyrillic hit");
        assert!(snip.starts_with('…'));
        assert!(snip.ends_with('…'));
        assert!(snip.contains("ЦЕЛЕВОЙ МАРКЕР"));
        assert!(snip.chars().count() < 160);

        giant.push_str(" TAIL_NEEDLE_XYZ ");
        giant.push_str(&"q".repeat(1000));
        let deep = match_snippet(&giant, "tail_needle_xyz").expect("deep giant hit");
        assert!(deep.starts_with('…'));
        assert!(deep.ends_with('…'));
        assert!(deep.contains("TAIL_NEEDLE_XYZ"));
        assert!(deep.chars().count() < 160);
    }
}
