use super::*;
use std::io::{BufRead, BufReader};

fn slug(cwd: &str) -> String {
    let mut trimmed = cwd.trim();
    if trimmed.len() > 1 {
        trimmed = trimmed.trim_end_matches(['\\', '/']);
    }
    if trimmed.len() == 2 && trimmed.as_bytes().get(1) == Some(&b':') {
        return format!("{trimmed}-").replace(':', "-");
    }
    let mut result: String = trimmed
        .encode_utf16()
        .map(|unit| {
            if unit <= 0x7f && (unit as u8).is_ascii_alphanumeric() {
                unit as u8 as char
            } else {
                '-'
            }
        })
        .collect();
    if result.len() > 200 {
        let hash = trimmed.encode_utf16().fold(0i32, |hash, unit| {
            hash.wrapping_mul(31).wrapping_add(i32::from(unit))
        });
        let mut magnitude = hash.unsigned_abs();
        let mut suffix = Vec::new();
        loop {
            suffix.push(char::from_digit(magnitude % 36, 36).unwrap());
            magnitude /= 36;
            if magnitude == 0 {
                break;
            }
        }
        result.truncate(200);
        result.push('-');
        result.extend(suffix.into_iter().rev());
    }
    if result.is_empty() {
        "imported".into()
    } else {
        result
    }
}

fn existing_directory(path: &Path) -> Option<String> {
    if !path.is_absolute() || !path.is_dir() {
        return None;
    }
    path.canonicalize()
        .ok()
        .map(|path| strip_long(&path.to_string_lossy()))
}

fn create_workspace(path: &Path, guard: &mut ImportGuard) -> Result<String> {
    guard.own(path.to_path_buf());
    fs::create_dir_all(path).with_context(|| {
        format!(
            "Не удалось создать рабочую папку импортированного диалога Claude: «{}»",
            path.display()
        )
    })?;
    let canonical = path.canonicalize().with_context(|| {
        format!(
            "Не удалось открыть рабочую папку импортированного диалога Claude: «{}»",
            path.display()
        )
    })?;
    Ok(strip_long(&canonical.to_string_lossy()))
}

fn working_directory(
    doc: &Value,
    root: &Path,
    default_home: bool,
    scratch: Option<&Path>,
    guard: &mut ImportGuard,
) -> Result<String> {
    let directory = strip_long(&s(&doc["meta"], "directory"));
    if !directory.is_empty() && !ephemeral(&directory) {
        if let Some(directory) = existing_directory(Path::new(&directory)) {
            return Ok(directory);
        }
    }
    if default_home {
        if let Some(scratch) = scratch {
            let path = scratch.join(format!(
                "scratch-{}-{}",
                Utc::now().format("%Y-%m-%d"),
                &uuid_hex()[..6]
            ));
            match create_workspace(&path, guard) {
                Ok(directory) => return Ok(directory),
                Err(_) => {
                    let _ = fs::remove_dir(&path);
                }
            }
        }
        if let Some(directory) = existing_directory(&home_dir()) {
            return Ok(directory);
        }
    }
    create_workspace(
        &root
            .join("workspaces")
            .join(format!("imported-{}", &uuid_hex()[..6])),
        guard,
    )
}
fn template(root: &Path) -> Value {
    let mut tpl = json!({"version":"2.1.325","model":"claude-sonnet-5-5","entrypoint":"claude-desktop","cwd":home_dir()});
    for p in recent_files(&root.join("projects"), 2, "jsonl")
        .into_iter()
        .take(5)
    {
        let Ok(f) = fs::File::open(p) else { continue };
        for line in BufReader::new(f).split(b'\n').take(80) {
            let Ok(raw) = line else { break };
            let Ok(v) = serde_json::from_slice::<Value>(&raw) else {
                break;
            };
            for key in ["version", "entrypoint"] {
                if !s(&v, key).is_empty() {
                    tpl[key] = v[key].clone();
                }
            }
            if !s(&v, "cwd").is_empty() && Path::new(&s(&v, "cwd")).is_dir() {
                tpl["cwd"] = v["cwd"].clone();
            }
            if v["type"] == "assistant" && !s(&v["message"], "model").is_empty() {
                tpl["model"] = v["message"]["model"].clone();
                return tpl;
            }
        }
    }
    tpl
}
fn newest_local_json(dir: &Path) -> Option<PathBuf> {
    recent_files(dir, usize::MAX, "json").into_iter().find(|p| {
        p.file_name()
            .is_some_and(|n| n.to_string_lossy().starts_with("local_"))
    })
}

fn find_desktop_in_base(base: &Path) -> Option<(PathBuf, Option<PathBuf>, Value)> {
    let ccs = base.join("claude-code-sessions");
    if !ccs.is_dir() {
        return None;
    }
    let cfg = fs::read(base.join("config.json"))
        .ok()
        .and_then(|b| serde_json::from_slice::<Value>(&b).ok())
        .unwrap_or(Value::Null);
    let active_account = nonempty(s(&cfg, "lastKnownAccountUuid"))
        .or_else(|| nonempty(s(&cfg["bootSideSessions"], "accountUuid")))
        .or_else(|| nonempty(s(&cfg["sshTransportFlagLastAnswer"], "accountUuid")));
    let active_org = active_account.as_deref().and_then(|acc| {
        if s(&cfg["sshTransportFlagLastAnswer"], "accountUuid") == acc {
            nonempty(s(&cfg["sshTransportFlagLastAnswer"], "orgUuid"))
        } else {
            None
        }
    });

    let mut session_dir: Option<PathBuf> = None;
    if let Some(acc) = &active_account {
        let acc_dir = ccs.join(acc);
        if let Some(org) = &active_org {
            let org_dir = acc_dir.join(org);
            if org_dir.is_dir() {
                session_dir = Some(org_dir);
            }
        }
        if session_dir.is_none() && acc_dir.is_dir() {
            if let Some(newest) = newest_local_json(&acc_dir).or_else(|| {
                recent_files(&acc_dir, usize::MAX, "json")
                    .into_iter()
                    .next()
            }) {
                if let Some(parent) = newest.parent() {
                    if parent != acc_dir {
                        session_dir = Some(parent.to_path_buf());
                    }
                }
            }
            if session_dir.is_none() {
                let mut subdirs: Vec<(std::time::SystemTime, PathBuf)> = fs::read_dir(&acc_dir)
                    .into_iter()
                    .flatten()
                    .filter_map(Result::ok)
                    .map(|e| e.path())
                    .filter(|p| p.is_dir())
                    .map(|p| {
                        let m = fs::metadata(&p)
                            .and_then(|m| m.modified())
                            .unwrap_or(std::time::UNIX_EPOCH);
                        (m, p)
                    })
                    .collect();
                subdirs.sort_by(|a, b| b.0.cmp(&a.0));
                if let Some((_, dir)) = subdirs.into_iter().next() {
                    session_dir = Some(dir);
                }
            }
        }
        if session_dir.is_none() {
            if let Some(org) = &active_org {
                session_dir = Some(acc_dir.join(org));
            }
        }
    }

    if session_dir.is_none() {
        if let Some(newest) = newest_local_json(&ccs)
            .or_else(|| recent_files(&ccs, usize::MAX, "json").into_iter().next())
        {
            session_dir = newest.parent().map(Path::to_path_buf);
        }
    }

    let dir = session_dir?;
    let tpl_path = newest_local_json(&dir).or_else(|| newest_local_json(&ccs));
    let object = tpl_path
        .and_then(|p| fs::read(p).ok())
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or(Value::Null);
    let relative = dir.strip_prefix(&ccs).unwrap_or(Path::new(""));
    let scratch_root = base.join("scratch-workspaces");
    let scratch = scratch_root.join(relative);
    Some((
        dir,
        if scratch.is_dir() || scratch_root.is_dir() {
            Some(scratch)
        } else {
            None
        },
        object,
    ))
}

fn find_desktop() -> (Option<PathBuf>, Option<PathBuf>, Value) {
    let user = home_dir();
    for root in [
        user.join("AppData/Local/Claude-Data"),
        user.join("AppData/Roaming/Claude"),
        user.join("Library/Application Support/Claude"),
        user.join(".config/Claude"),
    ] {
        if !root.is_dir() {
            continue;
        }
        let bases = if root.join("claude-code-sessions").is_dir() {
            vec![root]
        } else {
            fs::read_dir(&root)
                .into_iter()
                .flatten()
                .filter_map(Result::ok)
                .map(|e| e.path())
                .filter(|p| p.is_dir() && p.join("claude-code-sessions").is_dir())
                .collect()
        };
        for base in bases {
            if let Some((dir, scratch, obj)) = find_desktop_in_base(&base) {
                return (Some(dir), scratch, obj);
            }
        }
    }
    (None, None, Value::Null)
}
struct DesktopSession<'a> {
    sid: &'a str,
    cwd: &'a str,
    title: &'a str,
    first: i64,
    last: i64,
    now: i64,
    turns: usize,
    last_uuid: Option<&'a str>,
    last_text: &'a str,
    model: &'a str,
}
fn write_desktop(
    dir: &Path,
    tpl: &Value,
    d: DesktopSession<'_>,
    guard: &mut ImportGuard,
) -> Result<()> {
    let local = format!("local_{}", uuid());
    let model = nonempty(s(tpl, "model")).unwrap_or_else(|| d.model.to_owned());
    let mut obj = json!({"sessionId":local,"cliSessionId":d.sid,"cwd":d.cwd,"originCwd":d.cwd,
        "gitAnchorsLookupOnly":true,"gitAnchorsFolderRealpath":d.cwd,"folderPlace":{"cwd":d.cwd,"realPath":d.cwd},
        "lastFocusedAt":d.now,"createdAt":d.first,"lastActivityAt":d.now,"lastTurnEdgeAt":d.last,"model":model,
        "effort":nonempty(s(tpl,"effort")).unwrap_or_else(||"high".into()),"sessionSettings":{"ultracode":false},
        "isArchived":false,"title":d.title,"titleSource":"auto",
        "permissionMode":nonempty(s(tpl,"permissionMode")).unwrap_or_else(||"auto".into()),
        "chromePermissionMode":"skip_all_permission_checks","completedTurns":d.turns.max(1),"titleTurn":0,
        "lastAssistantUuid":d.last_uuid,"lastAssistantCliSessionId":d.sid,"interruptedAwaitingPerson":null,
        "lastSpawnRootDetected":false,"autoChosenInApp":true,"scratchPromptRecents":[],"envScopeId":"builtin_local",
        "startedFromEnvironmentId":null,"remoteControlAutoEligible":false,"steeredByRemoteClient":false,
        "latestUserFrameAt":d.last,"alwaysAllowedReasons":[],"sessionPermissionUpdates":[],"classifierSummaryEnabled":true,
        "reportFindingsCard":true,"turnBoxDeclared":true,"setupTools":true,"turnBoxMounted":true,
        "autoModeServerFallbackPrompt":true,"spawnSeed":{}});
    if let Some(last_uuid) = d.last_uuid {
        let summary = shorten(
            &d.last_text.split_whitespace().collect::<Vec<_>>().join(" "),
            220,
        );
        obj["postTurnSummary"] = json!({"status_category":"review_ready","status_detail":if summary.is_empty(){d.title}else{&summary},"needs_action":"","summarizes_uuid":last_uuid});
        obj["postTurnSummaryFor"] = json!(last_uuid);
    }
    if tpl["toolSurfaceSnapshot"].is_object() {
        let mut snap = tpl["toolSurfaceSnapshot"].clone();
        snap["recordedAt"] = json!(d.now);
        obj["toolSurfaceSnapshot"] = snap;
    }
    fs::create_dir_all(dir).with_context(|| {
        format!(
            "Не удалось создать папку сессий Claude Desktop: «{}»",
            dir.display()
        )
    })?;
    let path = dir.join(format!("{local}.json"));
    guard.own(path.clone());
    write_json(&path, &obj).with_context(|| {
        format!(
            "Не удалось сохранить запись импортированного диалога Claude Desktop: «{}»",
            path.display()
        )
    })
}
fn usage(text: &str) -> Value {
    json!({"input_tokens":0,"cache_creation_input_tokens":0,"cache_read_input_tokens":0,
    "output_tokens":(text.chars().count()/4).max(1),"output_tokens_details":{"thinking_tokens":0},
    "server_tool_use":{"web_search_requests":0,"web_fetch_requests":0},"service_tier":"standard"})
}

pub(super) fn import(
    doc: &Value,
    root: &Path,
    title: Option<&str>,
    include_tools: bool,
    default_home: bool,
    desktop_override: Option<&Path>,
) -> Result<Value> {
    let entries = filtered(doc, include_tools)?;
    let title = title_for(doc, &entries, title);
    let tpl = template(root);
    let version = s(&tpl, "version");
    let model = s(&tpl, "model");
    let entrypoint = s(&tpl, "entrypoint");
    let (desktop, scratch, dt_tpl) = if default_home && desktop_override.is_none() {
        find_desktop()
    } else {
        (desktop_override.map(Path::to_path_buf), None, Value::Null)
    };
    let mut guard = ImportGuard::default();
    let now = now_sec();
    let cwd = working_directory(doc, root, default_home, scratch.as_deref(), &mut guard)?;
    let sid = uuid();
    let project = root.join("projects").join(slug(&cwd));
    fs::create_dir_all(&project).with_context(|| {
        format!(
            "Не удалось создать папку проекта Claude для «{cwd}»: «{}»",
            project.display()
        )
    })?;
    let target = project.join(format!("{sid}.jsonl"));
    guard.own(target.clone());
    guard.own(project.join(&sid));
    let times = timestamps(&entries, now);
    let first = times[0];
    let last = times[times.len() - 1];
    let first_iso = iso(first);
    let mut lines = vec![
        json!({"type":"queue-operation","operation":"enqueue","timestamp":first_iso,"sessionId":sid}),
        json!({"type":"queue-operation","operation":"dequeue","timestamp":first_iso,"sessionId":sid}),
    ];
    let mut prev: Option<String> = None;
    let mut last_uuid: Option<String> = None;
    let mut last_text = String::new();
    let mut last_user = String::new();
    let mut prompt = 0usize;
    let mut prompt_id = uuid();
    let platform = if entrypoint == "claude-desktop" {
        "desktop_app"
    } else {
        "cli"
    };
    let common = |id: &str, ts: &str| {
        json!({"isSidechain":false,"userType":"external","entrypoint":entrypoint,
        "cwd":cwd,"sessionId":sid,"version":version,"gitBranch":"","uuid":id,"timestamp":ts})
    };
    let mut i = 0usize;
    while i < entries.len() {
        let e = &entries[i];
        let time = times[i];
        let ts = iso(time);
        let uid = uuid();
        let kind = nonempty(s(e, "kind")).unwrap_or_else(|| "assistant".into());
        let subtype = s(e, "subtype");
        let text = s(e, "text");
        if kind == "system"
            && subtype == "safeguard"
            && doc["meta"]["source_provider"] == "claude"
            && crate::claude::is_classifier_text(&text)
        {
            i += 1;
            let mut rec = common(&uid, &ts);
            rec["parentUuid"] = json!(prev);
            rec["type"] = json!("system");
            rec["subtype"] = json!("informational");
            rec["level"] = json!("warning");
            rec["isMeta"] = json!(false);
            rec["content"] = json!(text);
            lines.push(rec);
            prev = Some(uid);
            continue;
        }
        if kind == "user" {
            i += 1;
            prompt += 1;
            prompt_id = uuid();
            last_user = text.clone();
            let mut rec = common(&uid, &ts);
            rec["parentUuid"] = json!(prev);
            rec["promptId"] = json!(prompt_id);
            rec["type"] = json!("user");
            rec["message"] = json!({"role":"user","content":text});
            rec["permissionMode"] = json!("auto");
            rec["origin"] = json!({"kind":"human"});
            rec["promptSource"] = json!("sdk");
            rec["turnOrigin"] = json!("human");
            rec["turnPosition"] = json!({"promptIndex":prompt,"turnIndex":prompt});
            rec["clientPlatform"] = json!(platform);
            rec["turnAttributionKey"] = json!(uid);
            lines.push(rec);
            prev = Some(uid);
            continue;
        }
        if kind == "assistant" {
            i += 1;
            let short = shorten(&uid.replace('-', ""), 24);
            let mut rec = common(&uid, &ts);
            rec["parentUuid"] = json!(prev);
            rec["message"] = json!({"model":model,"id":format!("msg_{short}"),"type":"message","role":"assistant","content":[{"type":"text","text":text}],
                "container":null,"stop_reason":"end_turn","stop_sequence":null,"usage":usage(&text)});
            rec["requestId"] = json!(format!("req_{short}"));
            rec["requestedModel"] = json!(model);
            rec["type"] = json!("assistant");
            rec["madeBy"] = json!("model");
            lines.push(rec);
            prev = Some(uid.clone());
            last_uuid = Some(uid);
            last_text = text;
            continue;
        }
        if kind == "tool" && subtype == "thinking" {
            i += 1;
            if text.trim().is_empty() {
                continue;
            }
            let short = shorten(&uid.replace('-', ""), 24);
            let mut rec = common(&uid, &ts);
            rec["parentUuid"] = json!(prev);
            rec["message"] = json!({"model":model,"id":format!("msg_{short}"),"type":"message","role":"assistant","content":[{"type":"thinking","thinking":text}],
                "container":null,"stop_reason":Value::Null,"stop_sequence":null,"usage":usage(&text)});
            rec["requestId"] = json!(format!("req_{short}"));
            rec["requestedModel"] = json!(model);
            rec["type"] = json!("assistant");
            rec["madeBy"] = json!("model");
            lines.push(rec);
            prev = Some(uid);
            continue;
        }
        if kind == "tool" {
            if tools::is_tool_result_entry(e) {
                i += 1;
                continue;
            }
            let inv = tools::consume_tool_pair(&entries, &times, &mut i);
            let call_ts = iso(inv.ts_call);
            let res_ts = iso(inv.ts_result.max(inv.ts_call));
            let short = shorten(&uid.replace('-', ""), 24);
            let tool_id = format!("toolu_{short}");
            let out_or_ok = if inv.result_text.is_empty() {
                "ok".to_owned()
            } else {
                inv.result_text.clone()
            };
            let nlines = out_or_ok.lines().count().max(1);
            let (tool_name, input, tr_content, tool_use_result): (String, Value, Value, Value) =
                match inv.canonical {
                    tools::CanonicalTool::Read {
                        path,
                        start_line,
                        end_line,
                    } => {
                        let mut inp = json!({"file_path":path});
                        if let Some(s) = start_line {
                            inp["offset"] = json!(s);
                            if let Some(e) = end_line.filter(|e| *e >= s) {
                                inp["limit"] = json!(e - s + 1);
                            }
                        }
                        let tr = json!(out_or_ok);
                        let tur = if inv.is_error {
                            json!(format!("Error: {out_or_ok}"))
                        } else {
                            json!({"type":"text","file":{"filePath":path,"content":out_or_ok,"numLines":nlines,"startLine":start_line.unwrap_or(1),"totalLines":nlines}})
                        };
                        ("Read".into(), inp, tr, tur)
                    }
                    tools::CanonicalTool::Edit {
                        path,
                        old_string,
                        new_string,
                        content,
                        ..
                    } => {
                        if !old_string.is_empty() {
                            let inp = json!({"file_path":path,"old_string":old_string,"new_string":new_string,"replace_all":false});
                            let tr = json!(out_or_ok);
                            let tur = if inv.is_error {
                                json!(format!("Error: {out_or_ok}"))
                            } else {
                                json!({"filePath":path,"oldString":old_string,"newString":new_string,"replaceAll":false})
                            };
                            ("Edit".into(), inp, tr, tur)
                        } else {
                            let file_content = content.unwrap_or(new_string);
                            let inp = json!({"file_path":path,"content":file_content});
                            let tr = json!(out_or_ok);
                            let tur = if inv.is_error {
                                json!(format!("Error: {out_or_ok}"))
                            } else {
                                json!({"type":"create","filePath":path,"content":file_content})
                            };
                            ("Write".into(), inp, tr, tur)
                        }
                    }
                    tools::CanonicalTool::Write { path, content } => {
                        let inp = json!({"file_path":path,"content":content});
                        let tr = json!(out_or_ok);
                        let tur = if inv.is_error {
                            json!(format!("Error: {out_or_ok}"))
                        } else {
                            json!({"type":"create","filePath":path,"content":content})
                        };
                        ("Write".into(), inp, tr, tur)
                    }
                    tools::CanonicalTool::Delete { path } => {
                        let cmd = format!("rm -f {}", tools::shell_quote(&path));
                        let desc = format!("Delete {}", tools::basename(&path));
                        let inp = json!({"command":cmd,"description":desc});
                        let tr = json!(out_or_ok);
                        let tur = if inv.is_error {
                            json!(format!("Error: {out_or_ok}"))
                        } else {
                            json!({"stdout":inv.result_text,"stderr":"","interrupted":false,"isImage":false})
                        };
                        ("Bash".into(), inp, tr, tur)
                    }
                    tools::CanonicalTool::Shell {
                        command,
                        description,
                    } => {
                        let mut inp = json!({"command":command});
                        if let Some(desc) = description.filter(|s| !s.is_empty()) {
                            inp["description"] = json!(desc);
                        }
                        let tr = json!(out_or_ok);
                        let tur = if inv.is_error {
                            json!(format!("Error: {out_or_ok}"))
                        } else {
                            json!({"stdout":inv.result_text,"stderr":"","interrupted":false,"isImage":false})
                        };
                        ("Bash".into(), inp, tr, tur)
                    }
                    tools::CanonicalTool::ListDir { path, pattern } => {
                        let pat = pattern.unwrap_or_else(|| "*".into());
                        let inp = json!({"pattern":pat,"path":path});
                        let files: Vec<String> = inv
                            .result_text
                            .lines()
                            .map(str::trim)
                            .filter(|s| !s.is_empty())
                            .map(str::to_owned)
                            .collect();
                        let tr = json!(out_or_ok);
                        let tur = if inv.is_error {
                            json!(format!("Error: {out_or_ok}"))
                        } else {
                            json!({"filenames":files,"durationMs":12,"numFiles":files.len(),"truncated":false})
                        };
                        ("Glob".into(), inp, tr, tur)
                    }
                    tools::CanonicalTool::Grep { pattern, path } => {
                        let mut inp = json!({"pattern":pattern,"output_mode":"content"});
                        if let Some(p) = path.filter(|s| !s.is_empty()) {
                            inp["path"] = json!(p);
                        }
                        let tr = json!(out_or_ok);
                        let tur = if inv.is_error {
                            json!(format!("Error: {out_or_ok}"))
                        } else {
                            json!({"mode":"content","numFiles":0,"filenames":[],"content":inv.result_text,"numLines":nlines,"totalLines":nlines})
                        };
                        ("Grep".into(), inp, tr, tur)
                    }
                    tools::CanonicalTool::WebSearch { query } => {
                        let inp = json!({"query":query});
                        let tr = json!([{"type":"text","text":out_or_ok}]);
                        ("WebSearch".into(), inp, tr.clone(), tr)
                    }
                    tools::CanonicalTool::Generic { name, args } => {
                        let safe: String = name
                            .chars()
                            .map(|ch| {
                                if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
                                    ch
                                } else {
                                    '_'
                                }
                            })
                            .take(64)
                            .collect();
                        let safe_name = if safe.is_empty() {
                            "tool".to_owned()
                        } else {
                            safe
                        };
                        let inp = if args.as_object().is_some_and(|m| !m.is_empty()) {
                            args
                        } else {
                            json!({"summary":shorten(&out_or_ok, 300)})
                        };
                        let tr = json!([{"type":"text","text":out_or_ok}]);
                        (safe_name, inp, tr.clone(), tr)
                    }
                };
            let mut rec = common(&uid, &call_ts);
            rec["parentUuid"] = json!(prev);
            rec["message"] = json!({"model":model,"id":format!("msg_{short}"),"type":"message","role":"assistant","content":[{"type":"tool_use","id":tool_id,"name":tool_name,"input":input}],
                "container":null,"stop_reason":"tool_use","stop_sequence":null,"usage":usage(&inv.call_text)});
            rec["requestId"] = json!(format!("req_{short}"));
            rec["requestedModel"] = json!(model);
            rec["type"] = json!("assistant");
            rec["madeBy"] = json!("model");
            lines.push(rec);

            let rid = uuid();
            let mut result = common(&rid, &res_ts);
            result["parentUuid"] = json!(uid);
            result["promptId"] = json!(prompt_id);
            result["type"] = json!("user");
            let mut tr_block =
                json!({"tool_use_id":tool_id,"type":"tool_result","content":tr_content});
            if inv.is_error {
                tr_block["is_error"] = json!(true);
            }
            result["message"] = json!({"role":"user","content":[tr_block]});
            result["toolUseResult"] = tool_use_result;
            result["sourceToolAssistantUUID"] = json!(uid);
            lines.push(result);
            prev = Some(rid);
            continue;
        }
        i += 1;
        if kind == "system"
            && subtype == "informational"
            && matches!(
                s(e, "status").as_str(),
                "info" | "notice" | "suggestion" | "warning"
            )
        {
            let mut rec = common(&uid, &ts);
            rec["parentUuid"] = json!(prev);
            rec["type"] = json!("system");
            rec["subtype"] = json!("informational");
            rec["level"] = e["status"].clone();
            rec["isMeta"] = json!(false);
            rec["content"] = json!(text);
            lines.push(rec);
        } else if kind == "compact" {
            lines.push(json!({"parentUuid":null,"logicalParentUuid":prev,"isSidechain":false,"type":"system","subtype":"compact_boundary",
                "content":if text.is_empty(){"Conversation compacted"}else{&text},"isMeta":false,"level":"info",
                "timestamp":ts,"uuid":uid,"cwd":cwd,"sessionId":sid,"version":version}));
        } else {
            lines.push(json!({"parentUuid":prev,"isSidechain":false,"userType":"external","cwd":cwd,"sessionId":sid,"version":version,
                "type":"system","subtype":if subtype.is_empty(){&kind}else{&subtype},"content":text,"uuid":uid,"timestamp":ts}));
        }
        prev = Some(uid);
    }
    lines.push(json!({"type":"custom-title","customTitle":title,"sessionId":sid}));
    lines.push(json!({"type":"agent-name","agentName":title,"sessionId":sid}));
    if let Some(prev) = &prev {
        lines.push(json!({"type":"last-prompt","lastPrompt":shorten(if last_user.is_empty(){&title}else{&last_user},500),"leafUuid":prev,"sessionId":sid}));
    }
    write_lines(&target, &lines).with_context(|| {
        format!(
            "Не удалось записать сообщения импортированного диалога Claude: «{}»",
            target.display()
        )
    })?;
    let title_path = project.join(&sid).join("custom-title.json");
    write_json(&title_path, &json!({"customTitle":title})).with_context(|| {
        format!(
            "Не удалось сохранить название импортированного диалога Claude: «{}»",
            title_path.display()
        )
    })?;
    let history = root.join("history.jsonl");
    let before_history = match fs::read(&history) {
        Ok(bytes) => Some(bytes),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => {
            return Err(error).with_context(|| {
                format!(
                    "Не удалось прочитать историю Claude перед добавлением диалога: «{}»",
                    history.display()
                )
            });
        }
    };
    let mut bytes = before_history.clone().unwrap_or_default();
    if !bytes.is_empty() && !bytes.ends_with(b"\n") {
        bytes.push(b'\n');
    }
    let item = json!({"display":title,"pastedContents":{},"timestamp":(now*1000.0) as i64,"project":cwd,"sessionId":sid});
    serde_json::to_writer(&mut bytes, &item)?;
    bytes.push(b'\n');
    let history_result = atomic_write(&history, &bytes);
    guard.index(history.clone(), "sessionId", &sid, before_history, bytes);
    history_result.with_context(|| {
        format!(
            "Не удалось добавить импортированный диалог в историю Claude: «{}»",
            history.display()
        )
    })?;
    if let Some(dir) = desktop {
        write_desktop(
            &dir,
            &dt_tpl,
            DesktopSession {
                sid: &sid,
                cwd: &cwd,
                title: &title,
                first: (first * 1000.0) as i64,
                last: (last * 1000.0) as i64,
                now: (now * 1000.0) as i64,
                turns: prompt,
                last_uuid: last_uuid.as_deref(),
                last_text: &last_text,
                model: &model,
            },
            &mut guard,
        )?;
    }
    guard.commit();
    Ok(json!({"provider":"claude","cid":sid,"title":title,"entries":entries.len()}))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn slugs_are_platform_independent() {
        assert_eq!(slug("C:\\Users\\пример"), "C--Users-------");
        assert_eq!(slug("C:\\"), "C--");
        assert_eq!(slug("/tmp/my project"), "-tmp-my-project");
        assert_eq!(slug("C:\\Projects\\🐈"), "C--Projects---");
    }

    #[test]
    fn long_slug_matches_claude_code_reference_vector() {
        let path = format!(
            "/private/tmp/cr-probe2/{}/{}/{}",
            "d".repeat(80),
            "e".repeat(80),
            "f".repeat(80)
        );
        let result = slug(&path);
        assert_eq!(path.len(), 265);
        assert_eq!(result.len(), 207);
        assert!(result.ends_with("-gmu1b2"));
        assert_eq!(&result[..200], &path.replace('/', "-")[..200]);
        assert_ne!(result, slug(&format!("{path}other")));
    }

    #[test]
    fn stale_desktop_scratch_directory_does_not_block_import() {
        let d = tempfile::tempdir().unwrap();
        let scratch = d.path().join("removed-scratch");
        fs::write(&scratch, b"retained").unwrap();
        let mut guard = ImportGuard::default();
        let directory = working_directory(
            &json!({"meta":{"directory":""}}),
            &d.path().join("claude"),
            true,
            Some(&scratch),
            &mut guard,
        )
        .unwrap();
        assert!(Path::new(&directory).is_absolute());
        assert!(Path::new(&directory).is_dir());
        assert_eq!(fs::read(&scratch).unwrap(), b"retained");
    }

    #[test]
    fn find_desktop_prefers_active_account_from_config_json() {
        let d = tempfile::tempdir().unwrap();
        let base = d.path().join("claude_profile");
        let old_dir = base.join("claude-code-sessions/acc-old/org-old");
        let new_dir = base.join("claude-code-sessions/acc-new/org-new");
        fs::create_dir_all(&old_dir).unwrap();
        fs::create_dir_all(&new_dir).unwrap();
        fs::create_dir_all(base.join("scratch-workspaces")).unwrap();
        write_json(
            &old_dir.join("local_11111111-1111-1111-1111-111111111111.json"),
            &json!({"model":"claude-opus-5-5","effort":"high"}),
        )
        .unwrap();
        write_json(&new_dir.join("scheduled-tasks.json"), &json!([])).unwrap();
        write_json(
            &base.join("config.json"),
            &json!({
                "lastKnownAccountUuid": "acc-new",
                "sshTransportFlagLastAnswer": {
                    "accountUuid": "acc-new",
                    "orgUuid": "org-new"
                }
            }),
        )
        .unwrap();

        let (dir, scratch, tpl) = find_desktop_in_base(&base).unwrap();
        assert_eq!(dir, new_dir);
        assert_eq!(
            scratch,
            Some(base.join("scratch-workspaces/acc-new/org-new"))
        );
        assert_eq!(tpl["model"], "claude-opus-5-5");
    }
}
