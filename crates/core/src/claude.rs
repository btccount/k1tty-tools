use crate::{
    jsonl as j,
    provider::{OpContext, Provider, ProviderPaths},
};
use anyhow::{bail, ensure, Context, Result};
use regex::Regex;
use serde_json::{json, Value};
use std::{
    collections::{HashMap, HashSet},
    fs::{self, File},
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
};

pub struct ClaudeProvider {
    paths: ProviderPaths,
}
impl ClaudeProvider {
    pub fn new(paths: ProviderPaths) -> Self {
        Self { paths }
    }
    fn projects(&self) -> PathBuf {
        self.paths.root.join("projects")
    }
    fn history(&self) -> PathBuf {
        self.paths.root.join("history.jsonl")
    }
    fn test_mode(&self) -> bool {
        let n = self
            .paths
            .root
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_lowercase();
        n == "sandbox" || n == "claude_sandbox" || n.contains("test")
    }
    fn files(&self) -> Vec<PathBuf> {
        j::paths(&self.projects(), "jsonl")
            .into_iter()
            .filter(|p| {
                p.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| !n.starts_with('.') && !n.contains(".tmp."))
            })
            .collect()
    }
    fn session(&self, cid: &str) -> Result<PathBuf> {
        j::valid_id(cid)?;
        self.files()
            .into_iter()
            .find(|p| p.file_stem().and_then(|n| n.to_str()) == Some(cid))
            .with_context(|| format!("Файл сессии {cid} не найден"))
    }
    fn running(&self) -> Vec<String> {
        crate::process::snapshot()
            .remove("claude")
            .unwrap_or_default()
    }
    fn revision(&self, cid: &str) -> Result<String> {
        j::hash_file(&self.session(cid)?)
    }
    fn check_revision(&self, cid: &str, expected: &str) -> Result<()> {
        if expected.is_empty() || self.revision(cid)? != expected {
            bail!("Сессия изменилась после предпросмотра. Обновите страницу и повторите действие.")
        }
        Ok(())
    }
    fn list(&self) -> Result<Vec<Value>> {
        let mut titles = HashMap::<String, String>::new();
        if self.history().is_file() {
            for line in j::read_lines(&self.history())? {
                if let Ok(v) = line.obj() {
                    let id = j::s(&v, "sessionId");
                    let title = j::s(&v, "display").trim().lines().next().unwrap_or("");
                    if !id.is_empty() && !title.is_empty() && !title.starts_with('/') {
                        titles
                            .entry(id.into())
                            .or_insert_with(|| j::truncate(title, 100));
                    }
                }
            }
        }
        let mut list = Vec::new();
        for p in self.files() {
            let cid = p
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();
            let slug = p
                .parent()
                .unwrap_or(Path::new(""))
                .file_name()
                .unwrap_or_default()
                .to_string_lossy();
            let (file_title, cwd) = extract_title(&p);
            let custom = p.parent().unwrap().join(&cid).join("custom-title.json");
            let title = j::load_json(&custom)
                .ok()
                .map(|v| j::s(&v, "customTitle").to_string())
                .filter(|t| !t.is_empty())
                .or_else(|| titles.get(&cid).cloned())
                .or_else(|| (!file_title.is_empty()).then_some(file_title))
                .unwrap_or_else(|| format!("Сессия {}", j::truncate(&cid, 8)));
            let project = if !cwd.is_empty() {
                cwd
            } else {
                decode_slug(&slug)
            };
            let badge = project.rsplit(['/', '\\']).next().unwrap_or("");
            let ts = j::mtime(&p);
            list.push(json!({"provider":"claude","id":cid,"title":title,"subtitle":project,"badge":if badge.is_empty(){"CL".to_string()}else{j::truncate(badge,16)},"size":p.metadata()?.len(),"mtime":j::display_time(ts),"mtime_ts":ts,"writable":true}));
        }
        list.sort_by(|a, b| {
            b["mtime_ts"]
                .as_f64()
                .partial_cmp(&a["mtime_ts"].as_f64())
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        Ok(list)
    }
    fn title(&self, cid: &str) -> String {
        self.list()
            .unwrap_or_default()
            .into_iter()
            .find(|v| j::s(v, "id") == cid)
            .map(|v| j::s(&v, "title").to_string())
            .unwrap_or_else(|| cid.into())
    }
    fn entries(&self, data: &Value) -> Result<Value> {
        let cid = j::required(data, "cid")?;
        let path = self.session(cid)?;
        let include = data["include_tools"] == true;
        let limit = data["limit"].as_u64().unwrap_or(80).clamp(1, 150) as usize;
        let end = if let Some(b) = data["before"].as_str().filter(|b| !b.is_empty()) {
            Some(
                b.strip_prefix('b')
                    .context("Некорректный курсор истории")?
                    .parse::<u64>()?,
            )
        } else {
            None
        };
        let mut items = Vec::new();
        let mut msgs = 0;
        let mut cursor = None;
        j::reverse_lines(File::open(path)?, end, |off, raw| {
            if !include
                && !raw.windows(6).any(|w| w == b"\"text\"")
                && !raw.windows(6).any(|w| w == b"\"user\"")
            {
                return Ok(true);
            }
            let Ok(v) = serde_json::from_slice::<Value>(raw) else {
                return Ok(true);
            };
            let Some(it) = history_item(&v, off) else {
                return Ok(true);
            };
            let k = j::s(&it, "kind");
            if !include && !matches!(k, "user" | "assistant") {
                return Ok(true);
            }
            if matches!(k, "user" | "assistant") {
                msgs += 1;
            }
            items.push(entry(it, true));
            if msgs >= limit || items.len() >= 2000 {
                cursor = (off > 0).then(|| format!("b{off}"));
                return Ok(false);
            }
            Ok(true)
        })?;
        items.reverse();
        let lock = self.probe_lock(Some(cid))?;
        Ok(
            json!({"provider":"claude","cid":cid,"turns":j::turns(items),"page_msgs":msgs,"total_label":format!("загружено: {msgs}"),"cursor":cursor,"has_more":cursor.is_some(),"writable":true,"file_locked":lock.is_some(),"lock_reason":lock.unwrap_or_default()}),
        )
    }
    fn target(&self, path: &Path, key: &str) -> Result<(Vec<j::Line>, usize, Value)> {
        let lines = j::read_lines(path)?;
        let off = key.strip_prefix('b').map(str::parse::<u64>).transpose()?;
        for (idx, l) in lines.iter().enumerate() {
            if off.is_some_and(|o| o != l.offset) {
                continue;
            }
            if let Ok(v) = l.obj() {
                if off == Some(l.offset) || (off.is_none() && j::s(&v, "uuid") == key) {
                    return Ok((lines, idx, v));
                }
            }
        }
        bail!("Запись с ключом {key} не найдена в сессии")
    }
    fn edit_preview(&self, data: &Value) -> Result<Value> {
        let cid = j::required(data, "cid")?;
        let rev = self.revision(cid)?;
        let (_, _, v) = self.target(&self.session(cid)?, j::required(data, "key")?)?;
        let (text, kind) = edit_info(&v);
        self.check_revision(cid, &rev)?;
        Ok(
            json!({"text":text,"kind":kind,"hint":"Изменение сохранится прямо в JSONL-файл сессии Claude Code. Перед записью создаётся автоматическая резервная копия.","revision":rev,"writable":true}),
        )
    }
    fn preview_cut(&self, data: &Value) -> Result<Value> {
        let cid = j::required(data, "cid")?;
        let mode = j::s(data, "mode");
        if !matches!(mode, "head" | "tail") {
            bail!("Недопустимый режим обрезки")
        };
        let rev = self.revision(cid)?;
        let (lines, idx, _) = self.target(&self.session(cid)?, j::required(data, "key")?)?;
        let cut = if mode == "tail" {
            &lines[idx..]
        } else {
            &lines[..idx]
        };
        let raw = cut_report(cid, mode, cut);
        self.check_revision(cid, &rev)?;
        Ok(
            json!({"report":{"provider":"claude","mode":mode,"steps_count":raw["will_delete"]["steps_preview_count"],"raw_count":raw["rollout_lines"],"raw_label":"строк JSONL","bytes":raw["bytes"],"extra_label":"DAG-целостность сохраняется","steps_preview":raw["will_delete"]["steps_preview"],"raw":raw},"revision":rev,"writable":true,"hint":"Будут удалены записи из JSONL-файла сессии. При обрезке сверху выбранная запись станет новым корнем (parentUuid = null). Создаётся полная резервная копия."}),
        )
    }
    fn backup(&self, cid: &str, tag: &str) -> Result<PathBuf> {
        let path = self.session(cid)?;
        let bdir = self
            .paths
            .backup_root
            .join(cid)
            .join(format!("{}_{}", j::stamp(), tag));
        fs::create_dir_all(&bdir)?;
        let file_name = path
            .file_name()
            .context("Нет имени файла")?
            .to_string_lossy();
        let sha = j::copy_checked(&path, &bdir.join(file_name.as_ref()))?;
        let raw = fs::read(&path)?;
        let custom = path.parent().unwrap().join(cid).join("custom-title.json");
        if custom.is_file() {
            j::copy_checked(&custom, &bdir.join("custom-title.json"))?;
        }
        let mut meta = json!({"tid":cid,"tag":tag,"created":bdir.file_name().unwrap().to_string_lossy(),"file_name":file_name,"project_slug":path.parent().unwrap().file_name().unwrap_or_default().to_string_lossy(),"relative_path":path.strip_prefix(self.projects())?.to_string_lossy(),"sha256":sha,"lines":raw.iter().filter(|&&b|b==b'\n').count()+usize::from(!raw.is_empty()&&!raw.ends_with(b"\n")),"size":raw.len()});
        if tag == "delete_conv" {
            let sub = path.parent().unwrap().join(cid);
            if sub.is_dir() {
                copy_tree(&sub, &bdir.join("session-data"))?;
            }
            if self.history().is_file() {
                let saved: Vec<_> = j::read_lines(&self.history())?
                    .into_iter()
                    .filter(|l| l.obj().is_ok_and(|v| j::s(&v, "sessionId") == cid))
                    .map(|l| {
                        String::from_utf8_lossy(&l.raw)
                            .trim_end_matches(['\r', '\n'])
                            .to_string()
                    })
                    .collect();
                meta["history_lines"] = json!(saved);
            }
        }
        j::save_json(&bdir.join("backup.meta.json"), &meta)?;
        Ok(bdir)
    }
    fn system_notices_info(&self, data: &Value) -> Result<Value> {
        let cid = j::required(data, "cid")?;
        let path = self.session(cid)?;
        let revision = self.revision(cid)?;
        let mut counts = notice_counts();
        let mut total = 0;
        for raw in BufReader::new(File::open(&path)?).split(b'\n') {
            let raw = raw?;
            if let Ok(value) = serde_json::from_slice::<Value>(&raw) {
                if let Some(kind) = system_notice_type(&value) {
                    counts[kind] = json!(counts[kind].as_u64().unwrap_or(0) + 1);
                    total += 1;
                }
            }
        }
        self.check_revision(cid, &revision)?;
        let lock = if path.metadata()?.permissions().readonly() {
            Some("Файл сессии доступен только для чтения.".to_owned())
        } else {
            self.probe_lock(Some(cid))?
        };
        Ok(json!({
            "provider":"claude","cid":cid,"revision":revision,
            "notice_count":total,"counts":counts,"types":system_notice_types(),
            "writable":lock.is_none(),"lock_reason":lock.unwrap_or_default()
        }))
    }
    fn system_notices_operation(&self, op: &str, data: &Value, ctx: &OpContext) -> Result<Value> {
        let cid = j::required(data, "cid")?;
        let revision = j::required(data, "revision")?;
        self.assert_writable(Some(cid))?;
        let path = self.session(cid)?;
        ensure!(
            !path.metadata()?.permissions().readonly(),
            "Файл сессии доступен только для чтения: {}",
            path.display()
        );
        let mut locked = j::LockedFile::open(&path, false)?;
        self.check_revision(cid, revision)?;
        let mut lines = j::read_lines(&path)?;
        let original = j::write_lines(&lines);
        ensure!(
            j::sha(&original) == revision,
            "Сессия изменилась после предпросмотра. Повторите действие."
        );
        let mut deleted_keys = Vec::new();
        let mut deleted_uuids = Vec::new();
        let mut added_entry = Value::Null;
        let added = match op {
            "add_system_notice" => {
                let kind = j::required(data, "notice_type")?;
                ensure!(
                    NOTICE_TYPES.contains(&kind),
                    "Неподдерживаемый тип уведомления Claude: {kind}"
                );
                let text = data["text"].as_str().unwrap_or("");
                ensure!(
                    kind == "safety_classifier" || !text.trim().is_empty(),
                    "Введите текст уведомления."
                );
                let row = make_system_notice(cid, &lines, kind, text)?;
                let ending = lines
                    .iter()
                    .map(|line| j::ending(&line.raw))
                    .find(|ending| !ending.is_empty())
                    .unwrap_or(b"\n")
                    .to_vec();
                if let Some(last) = lines.last_mut() {
                    if j::ending(&last.raw).is_empty() {
                        last.raw.extend_from_slice(&ending);
                    }
                }
                let offset = lines.iter().map(|line| line.raw.len() as u64).sum();
                let mut raw = serde_json::to_vec(&row)?;
                raw.extend_from_slice(&ending);
                lines.push(j::Line { offset, raw });
                added_entry = entry(
                    history_item(&row, offset).context("Не удалось создать уведомление Claude")?,
                    true,
                );
                1
            }
            "delete_system_notices" => {
                let mut removed = HashMap::new();
                let mut logical_removed = HashMap::new();
                let mut retained = HashSet::new();
                let mut kept = Vec::with_capacity(lines.len());
                for line in lines {
                    if let Ok(value) = line.obj() {
                        let id = j::s(&value, "uuid");
                        if system_notice_type(&value).is_some() {
                            deleted_keys.push(format!("b{}", line.offset));
                            if !id.is_empty() {
                                deleted_uuids.push(id.to_owned());
                                removed.insert(
                                    id.to_owned(),
                                    value
                                        .get("parentUuid")
                                        .or_else(|| value.get("logicalParentUuid"))
                                        .cloned()
                                        .unwrap_or(Value::Null),
                                );
                                logical_removed.insert(
                                    id.to_owned(),
                                    value
                                        .get("logicalParentUuid")
                                        .or_else(|| value.get("parentUuid"))
                                        .cloned()
                                        .unwrap_or(Value::Null),
                                );
                            }
                            continue;
                        }
                        if !id.is_empty() {
                            retained.insert(id.to_owned());
                        }
                    }
                    kept.push(line);
                }
                removed.retain(|id, _| !retained.contains(id));
                logical_removed.retain(|id, _| !retained.contains(id));
                deleted_uuids.retain(|id| !retained.contains(id));
                for line in &mut kept {
                    let Ok(mut value) = line.obj() else {
                        continue;
                    };
                    let mut changed = false;
                    for field in ["parentUuid", "logicalParentUuid", "leafUuid"] {
                        if let Some(previous) = value.get(field) {
                            let next = surviving_reference(
                                previous,
                                if field == "logicalParentUuid" {
                                    &logical_removed
                                } else {
                                    &removed
                                },
                            )?;
                            if next != *previous {
                                value[field] = next;
                                changed = true;
                            }
                        }
                    }
                    if changed {
                        line.set(&value)?;
                    }
                }
                lines = kept;
                0
            }
            _ => bail!("Неизвестная операция уведомлений Claude: {op}"),
        };
        let changed = added > 0 || !deleted_keys.is_empty();
        if !changed {
            return Ok(json!({
                "provider":"claude","cid":cid,"tid":cid,"revision":revision,
                "changed":false,"no_op":true,"added":0,"deleted_count":0,
                "deleted_keys":[],"deleted_uuids":[],"backup":null,"warns":[]
            }));
        }
        let backup = if ctx.auto_backup {
            ctx.phase("backup");
            Some(self.backup(cid, op)?)
        } else {
            None
        };
        ctx.phase("write");
        let bytes = j::write_lines(&lines);
        self.check_revision(cid, revision)?;
        if let Err(error) = locked.replace_bytes(&bytes) {
            locked
                .replace_bytes(&original)
                .context("Не удалось восстановить сессию после ошибки записи уведомлений")?;
            return Err(error).with_context(|| {
                format!(
                    "Не удалось сохранить уведомления Claude: {}",
                    path.display()
                )
            });
        }
        ctx.phase("done");
        Ok(json!({
            "provider":"claude","cid":cid,"tid":cid,"revision":j::sha(&bytes),
            "changed":true,"no_op":false,"added":added,"entry":added_entry,
            "deleted_count":deleted_keys.len(),"deleted_keys":deleted_keys,
            "deleted_uuids":deleted_uuids,"backup":backup,"warns":[]
        }))
    }
    fn mutate(&self, op: &str, data: &Value, ctx: &OpContext) -> Result<Value> {
        let cid = j::required(data, "cid")?;
        self.assert_writable(Some(cid))?;
        let path = self.session(cid)?;
        let key = j::required(data, "key")?;
        let mut locked = j::LockedFile::open(&path, false)?;
        if op != "delete_single" || !j::s(data, "revision").is_empty() {
            self.check_revision(cid, j::s(data, "revision"))?;
        }
        let (mut lines, idx, obj) = self.target(&path, key)?;
        let original = j::write_lines(&lines);
        let tag = match op {
            "apply_edit" => "web_edit",
            "apply_cut" => "web_cut",
            _ => "del_single",
        };
        let bdir = if ctx.auto_backup {
            ctx.phase("backup");
            Some(self.backup(cid, tag)?)
        } else {
            None
        };
        let old = edit_info(&obj).0;
        let mut removed = Vec::new();
        let mut rep;
        match op {
            "apply_edit" => {
                ctx.phase("edit");
                let text = j::s(data, "text");
                if text.trim().is_empty() {
                    bail!("Пустой текст, сохранение отменено.")
                }
                let mut edited = obj;
                set_text(&mut edited, text)?;
                lines[idx].set(&edited)?;
                rep = json!({"tid":cid,"key":key,"old":old,"updated":true});
            }
            "delete_single" => {
                ctx.phase("cut");
                let uuid = obj["uuid"].clone();
                let parent = obj["parentUuid"].clone();
                lines.remove(idx);
                if !uuid.is_null() {
                    for line in &mut lines {
                        if let Ok(mut row) = line.obj() {
                            if row["parentUuid"] == uuid {
                                row["parentUuid"] = parent.clone();
                                line.set(&row)?;
                            }
                        }
                    }
                }
                removed.push(old);
                rep = json!({"thread":cid,"deleted_key":key,"deleted_uuid":uuid});
            }
            "apply_cut" => {
                ctx.phase("cut");
                let mode = j::s(data, "mode");
                if !matches!(mode, "head" | "tail") {
                    bail!("Недопустимый режим обрезки")
                };
                let cut: Vec<_> = if mode == "tail" {
                    lines[idx..].to_vec()
                } else {
                    lines[..idx].to_vec()
                };
                rep = cut_report(cid, mode, &cut);
                for l in &cut {
                    if let Ok(v) = l.obj() {
                        if let Some(it) = history_item(&v, l.offset) {
                            removed.push(j::s(&it, "text").to_string());
                        }
                    }
                }
                if mode == "tail" {
                    lines.truncate(idx)
                } else {
                    let preamble: Vec<_> = lines[..idx.min(5)]
                        .iter()
                        .filter(|l| {
                            l.obj().is_ok_and(|v| {
                                v["uuid"].is_null()
                                    && v["parentUuid"].is_null()
                                    && matches!(
                                        j::s(&v, "type"),
                                        "mode" | "permission-mode" | "atis-latch"
                                    )
                            })
                        })
                        .cloned()
                        .collect();
                    let mut target = obj;
                    if target.get("parentUuid").is_some() {
                        target["parentUuid"] = Value::Null;
                    }
                    lines[idx].set(&target)?;
                    lines = preamble
                        .into_iter()
                        .chain(lines.into_iter().skip(idx))
                        .collect();
                }
            }
            _ => bail!("Неизвестная операция"),
        }
        let bytes = j::write_lines(&lines);
        if let Err(error) = locked.replace_bytes(&bytes) {
            locked
                .replace_bytes(&original)
                .context("Не удалось восстановить исходный файл после ошибки записи")?;
            return Err(error);
        }
        ctx.phase("leak");
        rep["leaks"] = j::leak_check(&[path, self.history()], &removed);
        rep["backup"] = json!(bdir);
        let mut warns = Vec::<String>::new();
        if !self.running().is_empty() {
            warns.push(
                "Обнаружен запущенный процесс Claude Code. Перезапустите сессию в терминале."
                    .into(),
            );
        }
        if self.test_mode() {
            warns.push("ТЕСТ-РЕЖИМ: изменения сохранены в песочницу.".into());
        }
        rep["warns"] = json!(warns);
        ctx.phase("done");
        Ok(rep)
    }
    fn delete_conv(&self, data: &Value, ctx: &OpContext) -> Result<Value> {
        let cid = j::required(data, "cid")?;
        self.assert_writable(Some(cid))?;
        let path = self.session(cid)?;
        let guard = j::LockedFile::open(&path, false)?;
        let bdir = if ctx.auto_backup {
            ctx.phase("backup");
            Some(self.backup(cid, "delete_conv")?)
        } else {
            None
        };
        ctx.phase("cut");
        drop(guard);
        fs::remove_file(&path)?;
        let sub = path.parent().unwrap().join(cid);
        if sub.is_dir() {
            fs::remove_dir_all(sub)?;
        }
        if self.history().is_file() {
            let lines = j::read_lines(&self.history())?;
            let kept: Vec<_> = lines
                .into_iter()
                .filter(|l| !l.obj().is_ok_and(|v| j::s(&v, "sessionId") == cid))
                .collect();
            j::atomic_write(&self.history(), &j::write_lines(&kept))?;
        }
        ctx.phase("done");
        Ok(
            json!({"deleted":cid,"provider":"claude","backup":bdir,"warns":if self.running().is_empty(){Vec::<String>::new()}else{vec!["Запущен Claude. Перезапустите сессию в приложении или терминале.".into()]} }),
        )
    }
    fn backup_file(&self, bdir: &Path, meta: &Value) -> Option<PathBuf> {
        let name = j::s(meta, "file_name");
        if !name.is_empty() {
            if let Ok(p) = j::safe_join(bdir, name) {
                if p.is_file() {
                    return Some(p);
                }
            }
        }
        j::paths(bdir, "jsonl").into_iter().next()
    }
    fn backup_title(&self, bdir: &Path, cid: &str, src: Option<&Path>) -> String {
        let custom = j::load_json(&bdir.join("custom-title.json"))
            .ok()
            .map(|v| j::s(&v, "customTitle").to_string())
            .filter(|s| !s.is_empty());
        if let Some(t) = custom {
            return t;
        }
        let t = self.title(cid);
        if t != cid {
            return t;
        }
        if let Some(src) = src {
            let (t, _) = extract_title(src);
            if !t.is_empty() {
                return t;
            }
        }
        cid.into()
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
        let mut groups = Vec::new();
        let mut count = 0;
        let mut size = 0;
        for dir in roots {
            let id = dir.file_name().unwrap_or_default().to_string_lossy();
            let mut backups = Vec::new();
            let mut title = String::new();
            for bdir in j::dirs(&dir) {
                let Ok(meta) = j::load_json(&bdir.join("backup.meta.json")) else {
                    continue;
                };
                let src = self.backup_file(&bdir, &meta);
                if title.is_empty() {
                    title = self.backup_title(&bdir, &id, src.as_deref());
                }
                let sz = j::size(&bdir);
                count += 1;
                size += sz;
                backups.push(json!({"provider":"claude","cid":id,"dir":bdir,"name":bdir.file_name().unwrap_or_default().to_string_lossy(),"size":sz,"meta":meta,"restorable":true,"inspectable":src.is_some(),"reason":""}));
            }
            if !backups.is_empty() {
                groups.push(json!({"provider":"claude","cid":id,"title":title,"backups":backups}));
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
        let meta = j::load_json(&bdir.join("backup.meta.json")).unwrap_or(json!({}));
        let src = self
            .backup_file(&bdir, &meta)
            .context("Файл JSONL отсутствует в резервной копии Claude.")?;
        let cid = if !j::s(&meta, "tid").is_empty() {
            j::s(&meta, "tid").to_string()
        } else {
            bdir.parent()
                .unwrap()
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned()
        };
        let include = data["include_tools"] == true;
        let flat: Vec<_> = j::read_lines(&src)?
            .iter()
            .filter_map(|l| l.obj().ok().and_then(|v| history_item(&v, l.offset)))
            .filter(|v| include || matches!(j::s(v, "kind"), "user" | "assistant"))
            .map(|v| entry(v, false))
            .collect();
        let count = flat.len();
        let end = data["before"]
            .as_str()
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(count)
            .min(count);
        let limit = data["limit"].as_u64().unwrap_or(80).clamp(1, 150) as usize;
        let max = if include { 350 } else { limit * 2 };
        let (mut start, mut msgs) = (end, 0);
        while start > 0 && msgs < limit && end - start < max {
            start -= 1;
            if matches!(j::s(&flat[start], "kind"), "user" | "assistant") {
                msgs += 1;
            }
        }
        let window = &flat[start..end];
        Ok(
            json!({"provider":"claude","cid":cid,"backup_dir":bdir,"backup_name":bdir.file_name().unwrap().to_string_lossy(),"title":self.backup_title(&bdir,&cid,Some(&src)),"readonly":true,"turns":if window.is_empty(){json!([])}else{json!([{"items":window}])},"page_msgs":msgs,"total_label":format!("записей: {count}"),"cursor":if start>0{Some(start.to_string())}else{None},"has_more":start>0,"writable":false}),
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
        let name = j::required(&meta, "file_name")?;
        j::valid_id(name)?;
        let src = j::safe_join(&bdir, name)?;
        if j::hash_file(&src)? != j::s(&meta, "sha256") {
            bail!("Контрольная сумма файла в резервной копии не совпадает")
        };
        let rel = j::s(&meta, "relative_path");
        let slug = j::s(&meta, "project_slug");
        let target = if !rel.is_empty() {
            j::safe_join(&self.projects(), rel)?
        } else if !slug.is_empty() {
            j::safe_join(&self.projects(), &format!("{slug}/{name}"))?
        } else {
            self.session(cid)?
        };
        if target.file_stem().and_then(|n| n.to_str()) != Some(cid) {
            bail!("Резервная копия относится к другой сессии.")
        };
        ctx.phase("restore");
        if target.is_file() {
            let mut locked = j::LockedFile::open(&target, false)?;
            locked.replace_from(&src)?;
        } else {
            j::atomic_write(&target, &fs::read(&src)?)?;
        }
        let custom = bdir.join("custom-title.json");
        if custom.is_file() {
            j::copy_checked(
                &custom,
                &target.parent().unwrap().join(cid).join("custom-title.json"),
            )?;
        }
        if bdir.join("session-data").is_dir() {
            copy_tree(
                &bdir.join("session-data"),
                &target.parent().unwrap().join(cid),
            )?;
        }
        if let Some(saved) = meta["history_lines"].as_array() {
            let mut raw = fs::read_to_string(self.history()).unwrap_or_default();
            let mut set: HashSet<String> = raw.lines().map(str::to_string).collect();
            for l in saved.iter().filter_map(Value::as_str) {
                if set.insert(l.into()) {
                    if !raw.is_empty() && !raw.ends_with('\n') {
                        raw.push('\n');
                    }
                    raw.push_str(l);
                    raw.push('\n');
                }
            }
            j::atomic_write(&self.history(), raw.as_bytes())?;
        }
        ctx.phase("done");
        Ok(json!({"restored":bdir,"target":target}))
    }
    fn export(&self, data: &Value) -> Result<Value> {
        let cid = j::required(data, "cid")?;
        let p = self.session(cid)?;
        let (_, cwd) = extract_title(&p);
        let remove = Regex::new(r"<system-reminder>[\s\S]*?</system-reminder>\s*")?;
        let mut entries = Vec::new();
        for line in j::read_lines(&p)? {
            if let Ok(obj) = line.obj() {
                if let Some(it) = history_item(&obj, line.offset) {
                    let k = j::s(&it, "kind");
                    let mut text = j::s(&it, "text").to_string();
                    if k == "user" {
                        let c = remove.replace_all(&text, "").trim().to_string();
                        if !c.is_empty() {
                            text = c
                        }
                    }
                    if text.trim().is_empty() && j::s(&it, "tool").is_empty() {
                        continue;
                    }
                    let mut exported = json!({"kind":k,"subtype":it.get("subtype").cloned().unwrap_or(json!(k)),"label":it.get("label").cloned().unwrap_or(json!(k)),"text":text,"ts":it["ts"],"tool":it["tool"],"status":it["status"],"code":""});
                    if j::s(&obj, "type") == "system"
                        && j::s(&obj, "subtype") == "informational"
                        && matches!(
                            j::s(&obj, "level"),
                            "info" | "notice" | "suggestion" | "warning"
                        )
                    {
                        exported["subtype"] = json!("informational");
                        exported["status"] = obj["level"].clone();
                    }
                    entries.push(exported);
                }
            }
        }
        Ok(j::normalized_doc(
            "claude",
            cid,
            &self.title(cid),
            &j::strip_long_prefix(&cwd),
            p.metadata()?.len(),
            entries,
            data["include_tools"] != false,
        ))
    }
    fn preview_slim(&self, data: &Value) -> Result<Value> {
        let cid = j::required(data, "cid")?;
        let path = self.session(cid)?;
        let lines = j::read_lines(&path)?;
        let threshold = (data["tool_threshold_kb"]
            .as_u64()
            .unwrap_or(4)
            .clamp(1, 512)
            * 1024) as usize;
        let mut thinking_count = 0usize;
        let mut thinking_bytes = 0usize;
        let mut heavy_tool_count = 0usize;
        let mut heavy_tool_bytes = 0usize;
        for line in &lines {
            let Ok(obj) = line.obj() else { continue };
            if let Some(arr) = obj["message"]["content"].as_array() {
                for item in arr {
                    let typ = j::s(item, "type");
                    if matches!(typ, "thinking" | "redacted_thinking") {
                        let sz = serde_json::to_vec(item).map(|b| b.len()).unwrap_or(0);
                        thinking_count += 1;
                        thinking_bytes += sz;
                    } else if typ == "tool_result" {
                        let sz = serde_json::to_vec(&item["content"])
                            .map(|b| b.len())
                            .unwrap_or(0);
                        if sz > threshold {
                            heavy_tool_count += 1;
                            heavy_tool_bytes += sz.saturating_sub(48);
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
    fn apply_slim(&self, data: &Value, ctx: &OpContext) -> Result<Value> {
        let cid = j::required(data, "cid")?;
        self.assert_writable(Some(cid))?;
        let path = self.session(cid)?;
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
        let mut locked = j::LockedFile::open(&path, false)?;
        let mut lines = j::read_lines(&path)?;
        let original = j::write_lines(&lines);
        let mut relinks: Vec<(Value, Value)> = Vec::new();
        let mut kept: Vec<j::Line> = Vec::with_capacity(lines.len());
        let mut thinking_removed = 0usize;
        let mut tools_trimmed = 0usize;
        for mut line in lines.drain(..) {
            let Ok(mut obj) = line.obj() else {
                kept.push(line);
                continue;
            };
            let mut modified = false;
            let mut drop_line = false;
            if let Some(arr) = obj
                .get_mut("message")
                .and_then(|m| m.get_mut("content"))
                .and_then(Value::as_array_mut)
            {
                let mut next_arr = Vec::with_capacity(arr.len());
                for mut item in arr.drain(..) {
                    let typ = j::s(&item, "type").to_owned();
                    if remove_thinking && matches!(typ.as_str(), "thinking" | "redacted_thinking") {
                        thinking_removed += 1;
                        modified = true;
                        continue;
                    }
                    if trim_heavy_tools && typ == "tool_result" {
                        let sz = serde_json::to_vec(&item["content"])
                            .map(|b| b.len())
                            .unwrap_or(0);
                        if sz > threshold {
                            item["content"] = json!(format!(
                                "[trimmed by K1tty Tools: {:.1} KB]",
                                sz as f64 / 1024.0
                            ));
                            tools_trimmed += 1;
                            modified = true;
                        }
                    }
                    next_arr.push(item);
                }
                if modified && next_arr.is_empty() {
                    drop_line = true;
                } else {
                    *arr = next_arr;
                }
            }
            if trim_heavy_tools && modified {
                for key in ["toolUseResult", "tool_use_result"] {
                    if let Some(v) = obj.get(key) {
                        let sz = serde_json::to_vec(v).map(|b| b.len()).unwrap_or(0);
                        if sz > threshold {
                            obj[key] = json!(format!(
                                "[trimmed by K1tty Tools: {:.1} KB]",
                                sz as f64 / 1024.0
                            ));
                        }
                    }
                }
            }
            if drop_line {
                let uuid = obj["uuid"].clone();
                let parent = obj["parentUuid"].clone();
                if !uuid.is_null() {
                    relinks.push((uuid, parent));
                }
            } else {
                if modified {
                    line.set(&obj)?;
                }
                kept.push(line);
            }
        }
        if thinking_removed == 0 && tools_trimmed == 0 {
            return Ok(json!({
                "provider": "claude",
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
        if !relinks.is_empty() {
            for line in &mut kept {
                let Ok(mut row) = line.obj() else { continue };
                let mut changed_parent = false;
                if let Some(mut cur) = row.get("parentUuid").cloned() {
                    let mut hops = 0;
                    while hops < 32 && !cur.is_null() {
                        if let Some((_, next_parent)) = relinks.iter().find(|(u, _)| *u == cur) {
                            cur = next_parent.clone();
                            changed_parent = true;
                            hops += 1;
                        } else {
                            break;
                        }
                    }
                    if changed_parent {
                        row["parentUuid"] = cur;
                        line.set(&row)?;
                    }
                }
            }
        }
        let backup = if ctx.auto_backup {
            ctx.phase("backup");
            Some(self.backup(cid, "slim")?)
        } else {
            None
        };
        ctx.phase("cut");
        let bytes = j::write_lines(&kept);
        let freed_bytes = original.len().saturating_sub(bytes.len());
        if let Err(error) = locked.replace_bytes(&bytes) {
            let _ = locked.replace_bytes(&original);
            return Err(error);
        }
        ctx.phase("done");
        Ok(json!({
            "provider": "claude",
            "cid": cid,
            "changed": true,
            "thinking_removed": thinking_removed,
            "tools_trimmed": tools_trimmed,
            "freed_bytes": freed_bytes,
            "freed_tokens": (freed_bytes + 3) / 4,
            "revision": j::sha(&bytes),
            "backup": backup,
            "warns": if self.running().is_empty() { Vec::<String>::new() } else { vec!["Запущен Claude. Переоткройте сессию для обновления.".into()] }
        }))
    }
}
impl Provider for ClaudeProvider {
    fn paths(&self) -> &ProviderPaths {
        &self.paths
    }
    fn probe_lock(&self, cid: Option<&str>) -> Result<Option<String>> {
        if let Some(cid) = cid {
            #[cfg(not(windows))]
            if !self.test_mode() {
                let running = self.running();
                if !running.is_empty() {
                    return Ok(Some(format!("Запущено: {}. На macOS/Linux нельзя проверить, занят ли конкретный файл, поэтому закройте Claude перед правкой.",running.join(", "))));
                }
            }
            if let Ok(p) = self.session(cid) {
                if j::LockedFile::open(&p, false).is_err() {
                    return Ok(Some(
                        "Файл сессии занят Claude. Закройте активную сессию Claude и повторите."
                            .into(),
                    ));
                }
            }
        }
        Ok(None)
    }
    fn call(&self, op: &str, data: &Value, ctx: &OpContext) -> Result<Value> {
        match op {
            "state" => {
                let running = self.running();
                let avail = self.projects().is_dir();
                let note = if !avail {
                    format!("Каталог проектов не найден: {}", self.projects().display())
                } else if self.test_mode() {
                    "ТЕСТ-РЕЖИМ: изменения сохраняются в песочницу.".into()
                } else if !running.is_empty() {
                    format!(
                        "Запущено: {}. Закройте активную сессию перед правкой.",
                        running.join(", ")
                    )
                } else {
                    "Готов к работе: правка сообщений, команд, выводов, thinking и safeguards."
                        .into()
                };
                Ok(
                    json!({"id":"claude","name":"Claude","short":"CL","available":avail,"writable":avail,"blocked":false,"running":running,"test":self.test_mode(),"appdata":self.paths.root,"backups_root":self.paths.backup_root,"note":note}),
                )
            }
            "convs" => Ok(json!(self.list()?)),
            "entries" => self.entries(data),
            "edit_preview" => self.edit_preview(data),
            "preview_cut" => self.preview_cut(data),
            "preview_slim" => self.preview_slim(data),
            "apply_slim" => self.apply_slim(data, ctx),
            "system_notices_info" => self.system_notices_info(data),
            "add_system_notice" | "delete_system_notices" => {
                self.system_notices_operation(op, data, ctx)
            }
            "apply_edit" | "apply_cut" | "delete_single" => self.mutate(op, data, ctx),
            "delete_conv" => self.delete_conv(data, ctx),
            "backups" => self.backups(data),
            "backup_entries" => self.backup_entries(data),
            "restore" => self.restore(data, ctx),
            "export_chat" => self.export(data),
            _ => bail!("Неизвестная операция Claude: {op}"),
        }
    }
}

const NOTICE_TYPES: [&str; 5] = [
    "info",
    "notice",
    "suggestion",
    "warning",
    "safety_classifier",
];
const SAFETY_CLASSIFIER_PREFIX: &str = "Your response above was stopped by a safety classifier.";

fn notice_counts() -> Value {
    json!({"info":0,"notice":0,"suggestion":0,"warning":0,"safety_classifier":0})
}

fn system_notice_types() -> Value {
    json!([
        {"id":"info","label":"Информация","level":"info","subtype":"informational","prefix":"","default_text":""},
        {"id":"notice","label":"Уведомление","level":"notice","subtype":"informational","prefix":"","default_text":""},
        {"id":"suggestion","label":"Рекомендация","level":"suggestion","subtype":"informational","prefix":"","default_text":""},
        {"id":"warning","label":"Предупреждение","level":"warning","subtype":"informational","prefix":"","default_text":""},
        {"id":"safety_classifier","label":"Шаблон Safety Classifier","level":"warning","subtype":"informational","prefix":SAFETY_CLASSIFIER_PREFIX,"default_text":""}
    ])
}

pub(crate) fn is_classifier_text(text: &str) -> bool {
    let prefix = SAFETY_CLASSIFIER_PREFIX.trim_end_matches('.');
    text.trim_start().strip_prefix(prefix).is_some_and(|rest| {
        rest.is_empty()
            || rest
                .chars()
                .next()
                .is_some_and(|c| c == '.' || c == ':' || c.is_whitespace())
    })
}

fn classifier_content(content: &Value) -> Option<String> {
    if let Some(text) = content.as_str() {
        return is_classifier_text(text).then(|| text.to_owned());
    }
    let parts = content.as_array()?;
    let mut texts = Vec::new();
    for part in parts {
        if j::s(part, "type") != "text" {
            return None;
        }
        let text = part["text"].as_str()?;
        if text.trim().is_empty() {
            continue;
        }
        if !is_classifier_text(text) {
            return None;
        }
        texts.push(text);
    }
    (!texts.is_empty()).then(|| texts.join("\n"))
}

fn system_notice_type(value: &Value) -> Option<&'static str> {
    match j::s(value, "type") {
        "user" => classifier_content(&value["message"]["content"])
            .is_some()
            .then_some("safety_classifier"),
        "system" => {
            let subtype = j::s(value, "subtype");
            if !matches!(
                subtype,
                "informational" | "notice" | "warning" | "error" | "safeguard" | "api_error"
            ) {
                return None;
            }
            let text = value["content"]
                .as_str()
                .or_else(|| value["message"].as_str())
                .or_else(|| value["message"]["content"].as_str())?;
            if text.trim().is_empty() {
                return None;
            }
            if is_classifier_text(text)
                && (j::s(value, "level") == "warning" || subtype == "safeguard")
            {
                return Some("safety_classifier");
            }
            Some(match j::s(value, "level") {
                "info" => "info",
                "notice" => "notice",
                "suggestion" => "suggestion",
                "warning" | "warn" | "error" => "warning",
                _ if matches!(subtype, "warning" | "error" | "api_error") => "warning",
                _ => "notice",
            })
        }
        _ => None,
    }
}

fn surviving_reference(value: &Value, removed: &HashMap<String, Value>) -> Result<Value> {
    let mut current = value.clone();
    let mut visited = HashSet::new();
    while let Some(id) = current.as_str() {
        let Some(parent) = removed.get(id) else {
            break;
        };
        ensure!(
            visited.insert(id.to_owned()),
            "В цепочке уведомлений Claude обнаружена циклическая ссылка. Изменение отменено."
        );
        current = parent.clone();
    }
    Ok(current)
}

fn make_system_notice(cid: &str, lines: &[j::Line], kind: &str, text: &str) -> Result<Value> {
    let mut common = serde_json::Map::new();
    let mut parent = Value::Null;
    for line in lines.iter().rev() {
        let Ok(value) = line.obj() else {
            continue;
        };
        if value["isSidechain"] == true {
            continue;
        }
        if !matches!(j::s(&value, "type"), "user" | "assistant" | "system") {
            continue;
        }
        for key in ["cwd", "version", "gitBranch", "entrypoint", "userType"] {
            if !common.contains_key(key) {
                if let Some(value) = value.get(key).filter(|v| !v.is_null()) {
                    common.insert(key.to_owned(), value.clone());
                }
            }
        }
        if parent.is_null()
            && !j::s(&value, "uuid").is_empty()
            && (value.get("parentUuid").is_some()
                || matches!(j::s(&value, "type"), "user" | "assistant"))
        {
            parent = value["uuid"].clone();
        }
    }
    let mut row = Value::Object(common);
    row["uuid"] = json!(uuid::Uuid::new_v4().to_string());
    row["parentUuid"] = parent;
    row["sessionId"] = json!(cid);
    row["timestamp"] = json!(j::now());
    row["isSidechain"] = json!(false);
    row["type"] = json!("system");
    row["subtype"] = json!("informational");
    row["isMeta"] = json!(false);
    if kind == "safety_classifier" {
        let content = if is_classifier_text(text) {
            text.to_owned()
        } else if text.trim().is_empty() {
            SAFETY_CLASSIFIER_PREFIX.to_owned()
        } else {
            format!("{SAFETY_CLASSIFIER_PREFIX}\n\n{text}")
        };
        row["level"] = json!("warning");
        row["content"] = json!(content);
    } else {
        row["level"] = json!(kind);
        row["content"] = json!(text);
    }
    Ok(row)
}

fn decode_slug(slug: &str) -> String {
    let b = slug.as_bytes();
    if b.len() > 3 && b[0].is_ascii_alphabetic() && &b[1..3] == b"--" {
        format!(
            "{}:\\{}",
            (b[0] as char).to_ascii_uppercase(),
            slug[3..].replace('-', "\\")
        )
    } else {
        slug.replace('-', "/")
    }
}
fn extract_title(path: &Path) -> (String, String) {
    let (mut title, mut cwd) = (String::new(), String::new());
    if let Ok(file) = File::open(path) {
        for raw in BufReader::new(file).split(b'\n').take(41) {
            let Ok(raw) = raw else { break };
            let Ok(v) = serde_json::from_slice::<Value>(&raw) else {
                continue;
            };
            if cwd.is_empty() {
                cwd = j::s(&v, "cwd").into();
            }
            if title.is_empty() && j::s(&v, "type") == "user" {
                let content = &v["message"]["content"];
                let txt = if let Some(t) = content.as_str() {
                    t.to_string()
                } else {
                    content
                        .as_array()
                        .into_iter()
                        .flatten()
                        .find(|b| {
                            j::s(b, "type") == "text"
                                && !j::s(b, "text").starts_with("<system-reminder>")
                        })
                        .map(|b| j::s(b, "text").to_string())
                        .unwrap_or_default()
                };
                let t = txt.trim();
                if ![
                    "<local-command-",
                    "<system-reminder>",
                    "<command-name>",
                    "Your response above",
                ]
                .iter()
                .any(|p| t.starts_with(p))
                {
                    title = j::truncate(t.lines().next().unwrap_or(""), 100);
                }
            }
        }
    }
    (title, cwd)
}

fn clean_human(c: &Value) -> (String, Option<String>) {
    if let Some(s) = c.as_str() {
        if s.starts_with("<system-reminder>") {
            if let Some(end) = s.find("</system-reminder>") {
                let end = end + "</system-reminder>".len();
                let reminder = s[..end].trim().to_string();
                let human = s[end..].trim().to_string();
                return if human.is_empty() {
                    (reminder, None)
                } else {
                    (human, Some(reminder))
                };
            }
        }
        (s.into(), None)
    } else if let Some(a) = c.as_array() {
        let mut human = Vec::new();
        let mut reminder = Vec::new();
        for b in a {
            if j::s(b, "type") == "text" {
                let t = j::s(b, "text");
                if t.starts_with("<system-reminder>") {
                    reminder.push(t)
                } else {
                    human.push(t)
                }
            }
        }
        let h = human.join("\n\n").trim().to_string();
        let r = reminder.join("\n\n").trim().to_string();
        (
            if h.is_empty() { r.clone() } else { h },
            (!r.is_empty()).then_some(r),
        )
    } else {
        (String::new(), None)
    }
}
pub fn history_item(v: &Value, offset: u64) -> Option<Value> {
    let mut base = json!({"key":format!("b{offset}"),"uuid":v["uuid"],"parent_uuid":v["parentUuid"],"offset":offset,"ts":v.get("timestamp").cloned().unwrap_or(json!(""))});
    let typ = j::s(v, "type");
    let c = &v["message"]["content"];
    let extra = match typ {
        "user" => {
            if let Some(arr) = c
                .as_array()
                .filter(|a| a.iter().any(|b| j::s(b, "type") == "tool_result"))
            {
                let mut texts = Vec::new();
                let mut tool = String::new();
                let mut error = false;
                for b in arr {
                    if j::s(b, "type") != "tool_result" {
                        continue;
                    }
                    if !j::s(b, "tool_use_id").is_empty() {
                        tool = j::s(b, "tool_use_id").into();
                    }
                    error |= b["is_error"] == true;
                    let c = &b["content"];
                    if let Some(s) = c.as_str() {
                        texts.push(s.to_string())
                    } else if let Some(a) = c.as_array() {
                        for sub in a {
                            if let Some(t) = sub.as_str() {
                                texts.push(t.into())
                            } else if !j::s(sub, "text").is_empty() {
                                texts.push(j::s(sub, "text").into());
                            }
                        }
                    }
                }
                json!({"kind":"tool","subtype":"tool_result","label":if tool.is_empty(){"Результат инструмента".into()}else{format!("Вывод ({tool})")},"tool":if tool.is_empty(){"tool_result".into()}else{tool},"text":texts.join("\n"),"status":if error{"error"}else{"ok"}})
            } else {
                let raw = c.as_str().unwrap_or("");
                if let Some(text) = classifier_content(c) {
                    json!({"kind":"system","subtype":"safeguard","notice_type":"safety_classifier","level":"warning","label":"Уведомление безопасности (Safety Classifier)","text":text})
                } else if [
                    "<local-command-caveat>",
                    "<command-name>",
                    "<local-command-stdout>",
                ]
                .iter()
                .any(|p| raw.starts_with(p))
                {
                    json!({"kind":"system","subtype":"command","label":"Локальная команда","text":raw})
                } else {
                    let (h, r) = clean_human(c);
                    json!({"kind":"user","subtype":"prompt","label":"Вы","text":h,"reminder":r})
                }
            }
        }
        "assistant" => {
            if let Some(arr) = c.as_array() {
                let mut found = None;
                for b in arr {
                    found = match j::s(b, "type") {
                        "text" => Some(
                            json!({"kind":"assistant","subtype":"reply","label":"Claude","model":v["message"].get("model").cloned().unwrap_or(json!("Claude")),"text":b.get("text").cloned().unwrap_or(json!(""))}),
                        ),
                        "thinking" => Some(
                            json!({"kind":"tool","subtype":"thinking","label":"Размышления (thinking)","tool":"thinking","text":b.get("thinking").cloned().unwrap_or(json!(""))}),
                        ),
                        "tool_use" => {
                            let name = b.get("name").and_then(Value::as_str).unwrap_or("Tool");
                            let input = &b["input"];
                            let text = if let Some(cmd) = input["command"].as_str() {
                                cmd.to_string()
                            } else {
                                j::text(input)
                            };
                            Some(
                                json!({"kind":"tool","subtype":"tool_use","label":format!("Команда: {name}"),"tool":name,"tool_id":b["id"],"text":text}),
                            )
                        }
                        _ => None,
                    };
                    if found.is_some() {
                        break;
                    }
                }
                found?
            } else if let Some(s) = c.as_str() {
                json!({"kind":"assistant","subtype":"reply","label":"Claude","model":v["message"].get("model").cloned().unwrap_or(json!("Claude")),"text":s})
            } else {
                return None;
            }
        }
        "system" => {
            let raw = if !v["content"].is_null() {
                j::text(&v["content"])
            } else {
                j::text(&v["message"])
            };
            if j::s(v, "subtype") == "compact_boundary" {
                json!({"kind":"compact","subtype":"compact_boundary","label":"Компакция","text":if raw.is_empty(){"Conversation compacted"}else{&raw}})
            } else {
                let notice_type = system_notice_type(v);
                let level = notice_type.map(|kind| {
                    if kind == "safety_classifier" {
                        "warning"
                    } else {
                        kind
                    }
                });
                let informational = j::s(v, "subtype") == "informational"
                    && matches!(
                        j::s(v, "level"),
                        "info" | "notice" | "suggestion" | "warning"
                    );
                let safe = raw.to_lowercase().contains("safeguard")
                    || j::s(v, "subtype") == "informational";
                let (label, subtype) = if notice_type == Some("safety_classifier") {
                    ("Уведомление безопасности (Safety Classifier)", "safeguard")
                } else if informational {
                    (
                        match notice_type {
                            Some("info") => "Информация Claude",
                            Some("suggestion") => "Рекомендация Claude",
                            Some("warning") => "Предупреждение Claude",
                            _ => "Уведомление Claude (Notice)",
                        },
                        "notice",
                    )
                } else if safe {
                    ("Уведомление Claude (Notice)", "safeguard")
                } else {
                    ("Система", "notice")
                };
                json!({"kind":"system","subtype":subtype,"notice_type":notice_type,"level":level,"label":label,"text":raw})
            }
        }
        _ => return None,
    };
    base.as_object_mut()?.extend(extra.as_object()?.clone());
    Some(base)
}
fn entry(it: Value, writable: bool) -> Value {
    let k = j::s(&it, "kind");
    let mut actions = Vec::new();
    if writable {
        actions.extend(["edit", "del_single"]);
        if k == "user" {
            actions.push("cut_head");
        }
        actions.push("cut_tail");
    }
    let mut result = json!({"key":it["key"],"uuid":it["uuid"],"kind":k,"subtype":it.get("subtype").cloned().unwrap_or(json!(k)),"label":it.get("label").cloned().unwrap_or(json!(j::label(k,"Claude"))),"text":it.get("text").cloned().unwrap_or(json!("")),"ts":it["ts"],"tool":it["tool"],"status":it["status"],"in_db":true,"actions":actions});
    for field in ["notice_type", "level"] {
        if let Some(value) = it.get(field).filter(|value| !value.is_null()) {
            result[field] = value.clone();
        }
    }
    result
}
fn edit_info(v: &Value) -> (String, String) {
    let typ = j::s(v, "type");
    let c = if !v["message"]["content"].is_null() {
        &v["message"]["content"]
    } else {
        &v["content"]
    };
    if typ == "system" {
        return (j::text(c), "Системное уведомление".into());
    }
    if typ == "user" {
        if let Some(a) = c.as_array() {
            for b in a {
                if j::s(b, "type") == "tool_result" {
                    let text = if let Some(a) = b["content"].as_array() {
                        a.iter()
                            .map(|b| {
                                if b.get("text").is_some() {
                                    j::text(&b["text"])
                                } else {
                                    j::text(b)
                                }
                            })
                            .collect::<Vec<_>>()
                            .join("\n")
                    } else {
                        j::text(&b["content"])
                    };
                    return (text, "Вывод инструмента (tool_result)".into());
                }
            }
            return (clean_human(c).0, "Сообщение пользователя".into());
        }
        if let Some(t) = c.as_str() {
            return (
                t.into(),
                if t.contains("stopped by a safety classifier")
                    || t.starts_with("Your response above")
                {
                    "Уведомление безопасности (Safeguard)"
                } else if t.starts_with("<local-command-") || t.starts_with("<system-reminder>") {
                    "Системная команда / контекст"
                } else {
                    "Сообщение пользователя"
                }
                .into(),
            );
        }
    }
    if typ == "assistant" {
        if let Some(a) = c.as_array() {
            for b in a {
                match j::s(b, "type") {
                    "text" => return (j::s(b, "text").into(), "Ответ Claude".into()),
                    "thinking" => {
                        return (
                            j::s(b, "thinking").into(),
                            "Размышления модели (thinking)".into(),
                        )
                    }
                    "tool_use" => {
                        let name = b.get("name").and_then(Value::as_str).unwrap_or("Tool");
                        let inp = &b["input"];
                        return if let Some(c) = inp["command"].as_str() {
                            (c.into(), format!("Команда инструмента ({name})"))
                        } else {
                            (j::text(inp), format!("Параметры инструмента ({name})"))
                        };
                    }
                    _ => {}
                }
            }
        } else if c.is_string() {
            return (j::text(c), "Ответ Claude".into());
        }
    }
    (j::text(c), "Запись".into())
}
fn set_text(v: &mut Value, text: &str) -> Result<()> {
    let typ = j::s(v, "type").to_string();
    if typ == "system" {
        v["content"] = json!(text);
        return Ok(());
    }
    if !matches!(typ.as_str(), "user" | "assistant") || !v["message"].is_object() {
        bail!("Ключ не указывает на редактируемое сообщение Claude.");
    }
    if let Some(a) = v["message"]["content"].as_array_mut() {
        for b in a.iter_mut() {
            match (typ.as_str(), j::s(b, "type")) {
                ("user", "tool_result") => {
                    b["content"] = json!(text);
                    return Ok(());
                }
                ("user", "text") if !j::s(b, "text").starts_with("<system-reminder>") => {
                    b["text"] = json!(text);
                    return Ok(());
                }
                ("assistant", "text") => {
                    b["text"] = json!(text);
                    return Ok(());
                }
                ("assistant", "thinking") => {
                    b["thinking"] = json!(text);
                    return Ok(());
                }
                ("assistant", "tool_use") => {
                    if b["input"].get("command").is_some() && !text.trim().starts_with('{') {
                        b["input"]["command"] = json!(text)
                    } else if let Ok(value) = serde_json::from_str::<Value>(text) {
                        b["input"] = value
                    } else if let Some(o) = b["input"].as_object_mut().filter(|o| o.len() == 1) {
                        let k = o.keys().next().unwrap().clone();
                        o.insert(k, json!(text));
                    } else {
                        b["input"] = json!({"command":text});
                    }
                    return Ok(());
                }
                _ => {}
            }
        }
        if typ == "user" {
            if let Some(first) = a.first_mut().filter(|v| v.get("text").is_some()) {
                first["text"] = json!(text)
            } else {
                a.push(json!({"type":"text","text":text}))
            }
            return Ok(());
        }
    }
    v["message"]["content"] = json!(text);
    Ok(())
}
fn cut_report(cid: &str, mode: &str, lines: &[j::Line]) -> Value {
    let mut count = 0;
    let mut preview = Vec::new();
    for l in lines {
        if let Ok(v) = l.obj() {
            if let Some(it) = history_item(&v, 0) {
                count += 1;
                if preview.len() < 12 {
                    preview.push(json!({"key":it["key"],"label":it["label"],"text":j::truncate(&j::s(&it,"text").replace('\n'," "),120)}));
                }
            }
        }
    }
    json!({"thread":cid,"mode":mode,"rollout_lines":lines.len(),"rollout_files":0,"bytes":lines.iter().map(|l|l.raw.len()).sum::<usize>(),"will_delete":{"steps_preview_count":count,"steps_preview":preview}})
}
fn copy_tree(src: &Path, dst: &Path) -> Result<()> {
    for e in walkdir::WalkDir::new(src).follow_links(false) {
        let e = e?;
        let p = dst.join(e.path().strip_prefix(src)?);
        if e.file_type().is_dir() {
            fs::create_dir_all(p)?;
        } else if e.file_type().is_file() {
            j::copy_checked(e.path(), &p)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (tempfile::TempDir, ClaudeProvider, PathBuf, Vec<j::Line>) {
        let d = tempfile::tempdir().unwrap();
        let root = d.path().join("claude_test");
        let p = root.join("projects/-tmp-проект/session-one.jsonl");
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        let rows = vec![
            json!({"type":"mode","value":"normal"}),
            json!({"type":"user","uuid":"u1","parentUuid":null,"cwd":"/tmp/проект","timestamp":"2026-01-01T00:00:00Z","message":{"content":[{"type":"text","text":"<system-reminder>Keep this metadata</system-reminder>"},{"type":"text","text":"Привет 🐈"}]},"unknown":{"keep":42}}),
            json!({"type":"assistant","uuid":"a1","parentUuid":"u1","message":{"content":[{"type":"thinking","thinking":"analysis","signature":"original"}]}}),
            json!({"type":"assistant","uuid":"a2","parentUuid":"a1","message":{"content":[{"type":"tool_use","id":"tool1","name":"Bash","input":{"command":"pwd","timeout":42}}]}}),
            json!({"type":"user","uuid":"u2","parentUuid":"a2","message":{"content":[{"type":"tool_result","tool_use_id":"tool1","content":"/tmp/проект"}]}}),
            json!({"type":"assistant","uuid":"a3","parentUuid":"u2","message":{"content":[{"type":"text","text":"Ответ"},{"type":"image","source":{"data":"original"}}]}}),
        ];
        let raw = rows
            .iter()
            .map(|r| serde_json::to_string(r).unwrap() + "\r\n")
            .collect::<String>();
        fs::write(&p, raw).unwrap();
        let lines = j::read_lines(&p).unwrap();
        let provider = ClaudeProvider::new(ProviderPaths {
            id: "claude".into(),
            name: "Claude".into(),
            short: "CL".into(),
            root,
            backup_root: d.path().join("backups"),
        });
        (d, provider, p, lines)
    }
    #[test]
    fn edit_preserves_metadata_and_restores_bytes() {
        let (_d, p, path, lines) = fixture();
        let original = fs::read(&path).unwrap();
        let key = format!("b{}", lines[1].offset);
        let preview = p
            .call(
                "edit_preview",
                &json!({"cid":"session-one","key":key}),
                &OpContext::default(),
            )
            .unwrap();
        let result=p.call("apply_edit",&json!({"cid":"session-one","key":key,"revision":preview["revision"],"text":"Новый вопрос ✓"}),&OpContext::default()).unwrap();
        let current = j::read_lines(&path).unwrap();
        assert_eq!(current[0].raw, lines[0].raw);
        assert_eq!(current[2].raw, lines[2].raw);
        let v = current[1].obj().unwrap();
        assert_eq!(v["unknown"]["keep"], 42);
        assert_eq!(
            v["message"]["content"][0]["text"],
            "<system-reminder>Keep this metadata</system-reminder>"
        );
        assert_eq!(v["message"]["content"][1]["text"], "Новый вопрос ✓");
        p.call(
            "restore",
            &json!({"backup_dir":result["backup"]}),
            &OpContext::default(),
        )
        .unwrap();
        assert_eq!(fs::read(path).unwrap(), original);
    }
    #[test]
    fn delete_single_relinks_dag_and_keeps_tools() {
        let (_d, p, path, lines) = fixture();
        p.call(
            "delete_single",
            &json!({"cid":"session-one","key":format!("b{}",lines[3].offset)}),
            &OpContext {
                auto_backup: false,
                ..Default::default()
            },
        )
        .unwrap();
        let rows = j::read_lines(&path).unwrap();
        assert_eq!(rows.len(), 5);
        let tool_result = rows[3].obj().unwrap();
        assert_eq!(tool_result["uuid"], "u2");
        assert_eq!(tool_result["parentUuid"], "a1");
        assert_eq!(
            tool_result["message"]["content"][0]["content"],
            "/tmp/проект"
        );
    }
    #[test]
    fn head_cut_keeps_preamble_and_creates_new_root() {
        let (_d, p, path, lines) = fixture();
        let key = format!("b{}", lines[4].offset);
        let pre = p
            .call(
                "preview_cut",
                &json!({"cid":"session-one","key":key,"mode":"head"}),
                &OpContext::default(),
            )
            .unwrap();
        p.call(
            "apply_cut",
            &json!({"cid":"session-one","key":key,"mode":"head","revision":pre["revision"]}),
            &OpContext::default(),
        )
        .unwrap();
        let rows = j::read_lines(&path).unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].raw, lines[0].raw);
        assert_eq!(rows[1].obj().unwrap()["parentUuid"], Value::Null);
        assert_eq!(rows[2].raw, lines[5].raw);
    }
    #[test]
    fn stale_revision_and_backup_corruption_refuse_write() {
        let (_d, p, path, lines) = fixture();
        let key = format!("b{}", lines[1].offset);
        let rev = p.revision("session-one").unwrap();
        let mut raw = fs::read(&path).unwrap();
        raw.extend(b"\n");
        fs::write(&path, &raw).unwrap();
        assert!(p
            .call(
                "apply_edit",
                &json!({"cid":"session-one","key":key,"revision":rev,"text":"wrong"}),
                &OpContext::default()
            )
            .is_err());
        assert_eq!(fs::read(&path).unwrap(), raw);
        let b = p.backup("session-one", "test").unwrap();
        fs::write(b.join("session-one.jsonl"), b"corrupt").unwrap();
        assert!(p
            .call("restore", &json!({"backup_dir":b}), &OpContext::default())
            .is_err());
        assert_eq!(fs::read(path).unwrap(), raw);
    }
    #[test]
    fn pagination_and_export_include_thinking() {
        let (_d, p, _path, _lines) = fixture();
        let r = p
            .call(
                "entries",
                &json!({"cid":"session-one","limit":1,"include_tools":true}),
                &OpContext::default(),
            )
            .unwrap();
        assert_eq!(r["page_msgs"], 1);
        assert_eq!(r["has_more"], true);
        let earlier = p
            .call(
                "entries",
                &json!({"cid":"session-one","limit":1,"include_tools":true,"before":r["cursor"]}),
                &OpContext::default(),
            )
            .unwrap();
        assert_eq!(earlier["page_msgs"], 1);
        let doc = p
            .call(
                "export_chat",
                &json!({"cid":"session-one","include_tools":true}),
                &OpContext::default(),
            )
            .unwrap();
        assert_eq!(doc["entries"].as_array().unwrap().len(), 5);
        assert_eq!(doc["entries"][0]["text"], "Привет 🐈");
        assert_eq!(doc["entries"][1]["subtype"], "thinking");
        let plain = p
            .call(
                "export_chat",
                &json!({"cid":"session-one","include_tools":false}),
                &OpContext::default(),
            )
            .unwrap();
        assert_eq!(plain["entries"].as_array().unwrap().len(), 2);
    }
    #[test]
    fn slim_strips_thinking_and_trims_heavy_tools_preserving_dag() {
        let (_d, p, path, _lines) = fixture();
        let big = "x".repeat(6000);
        let extra = serde_json::to_string(&json!({
            "type": "user",
            "uuid": "u-big",
            "parentUuid": "u5",
            "timestamp": "2026-04-01T10:05:00Z",
            "message": {
                "role": "user",
                "content": [{
                    "type": "tool_result",
                    "tool_use_id": "tu-big",
                    "content": big
                }]
            }
        }))
        .unwrap();
        let mut raw = fs::read_to_string(&path).unwrap();
        raw.push_str(&extra);
        raw.push('\n');
        fs::write(&path, raw).unwrap();

        let prev = p
            .call(
                "preview_slim",
                &json!({"cid":"session-one","remove_thinking":true,"trim_heavy_tools":true,"tool_threshold_kb":4}),
                &OpContext::default(),
            )
            .unwrap();
        assert_eq!(prev["can_slim"], true);
        assert_eq!(prev["thinking_count"], 1);
        assert_eq!(prev["heavy_tool_count"], 1);
        assert!(prev["freed_bytes"].as_u64().unwrap() > 5000);

        let res = p
            .call(
                "apply_slim",
                &json!({"cid":"session-one","remove_thinking":true,"trim_heavy_tools":true,"tool_threshold_kb":4,"confirm":true}),
                &OpContext::default(),
            )
            .unwrap();
        assert_eq!(res["changed"], true);
        assert_eq!(res["thinking_removed"], 1);
        assert_eq!(res["tools_trimmed"], 1);

        let after_lines = j::read_lines(&path).unwrap();
        assert!(after_lines.iter().all(|l| {
            let Ok(v) = l.obj() else { return true };
            v["message"]["content"]
                .as_array()
                .map(|a| a.iter().all(|b| b["type"] != "thinking"))
                .unwrap_or(true)
        }));
        let trimmed_row = after_lines
            .iter()
            .filter_map(|l| l.obj().ok())
            .find(|v| v["uuid"] == "u-big")
            .unwrap();
        assert_eq!(
            trimmed_row["message"]["content"][0]["tool_use_id"],
            "tu-big"
        );
        assert!(trimmed_row["message"]["content"][0]["content"]
            .as_str()
            .unwrap()
            .contains("[trimmed by K1tty Tools:"));
    }
}
