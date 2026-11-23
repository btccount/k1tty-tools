use super::*;
use crate::pbwire;
use rusqlite::{params, OptionalExtension};

fn b(n: u32, data: impl AsRef<[u8]>) -> Vec<u8> {
    pbwire::encode_field(n, 2, data.as_ref())
}
fn v(n: u32, value: u64) -> Vec<u8> {
    pbwire::encode_field(n, 0, &pbwire::encode_varint(value))
}
fn nested(path: &[u32], data: &[u8]) -> Vec<u8> {
    path.iter().rev().fold(data.to_vec(), |out, n| b(*n, out))
}
fn ts(sec: f64) -> Vec<u8> {
    let whole = sec.max(0.0) as u64;
    let nanos = ((sec - whole as f64) * 1e9).clamp(0.0, 999_999_999.0) as u64;
    let mut out = v(1, whole);
    if nanos > 0 {
        out.extend(v(2, nanos));
    }
    out
}
fn get(bytes: &[u8], path: &[u32]) -> Vec<u8> {
    pbwire::bytes_at(bytes, path).unwrap_or_default()
}

struct Template {
    cap: Vec<u8>,
    instance: String,
    user_cfg: Vec<u8>,
}
fn template(root: &Path) -> Template {
    let mut tpl = Template {
        cap: Vec::new(),
        instance: String::new(),
        user_cfg: Vec::new(),
    };
    for path in recent_files(&root.join("conversations"), 1, "db")
        .into_iter()
        .take(6)
    {
        let Ok(c) = readonly(&path) else { continue };
        let uv: i64 = c
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap_or(0);
        if uv < 1 {
            continue;
        }
        if tpl.cap.is_empty() {
            let blob = c.query_row(
                "SELECT data FROM trajectory_metadata_blob WHERE id='main'",
                [],
                |r| r.get::<_, Vec<u8>>(0),
            );
            if let Ok(blob) = blob {
                tpl.cap = get(&blob, &[15]);
                if tpl.instance.is_empty() {
                    tpl.instance = String::from_utf8(get(&blob, &[3])).unwrap_or_default();
                }
            }
        }
        if tpl.user_cfg.is_empty() {
            if let Ok(mut stmt) = c.prepare(
                "SELECT step_payload FROM steps WHERE step_type=14 ORDER BY idx DESC LIMIT 8",
            ) {
                if let Ok(rows) = stmt.query_map([], |r| r.get::<_, Vec<u8>>(0)) {
                    for blob in rows.flatten() {
                        let cfg = get(&blob, &[19, 12]);
                        if !cfg.is_empty() {
                            tpl.user_cfg = cfg;
                            break;
                        }
                    }
                }
            }
        }
        if !tpl.cap.is_empty() && !tpl.user_cfg.is_empty() {
            break;
        }
    }
    tpl
}

fn merge(entries: &[Value], times: &[f64]) -> Vec<(Value, f64, String)> {
    let mut out = Vec::new();
    let mut pending: Vec<String> = Vec::new();
    for (e, time) in entries.iter().zip(times) {
        let kind = s(e, "kind");
        let subtype = s(e, "subtype");
        let text = s(e, "text");
        if kind == "tool" && subtype == "thinking" {
            if !text.trim().is_empty() {
                pending.push(text.trim().to_owned());
            }
            continue;
        }
        if kind == "assistant" && !pending.is_empty() {
            out.push((e.clone(), *time, pending.join("\n\n")));
            pending.clear();
        } else {
            if !pending.is_empty() {
                out.push((json!({"kind":"assistant","subtype":"planner_response","text":"","label":"thinking"}),*time,pending.join("\n\n")));
                pending.clear();
            }
            out.push((e.clone(), *time, String::new()));
        }
    }
    if !pending.is_empty() {
        out.push((
            json!({"kind":"assistant","subtype":"planner_response","text":"","label":"thinking"}),
            *times.last().unwrap_or(&now_sec()),
            pending.join("\n\n"),
        ));
    }
    out
}

fn create_schema(c: &Connection) -> Result<()> {
    c.execute_batch(
    "PRAGMA page_size=32768;
    PRAGMA journal_mode=WAL;
    CREATE TABLE trajectory_meta (trajectory_id text PRIMARY KEY,cascade_id text,trajectory_type integer,source integer);
    CREATE TABLE steps (idx integer PRIMARY KEY,step_type integer NOT NULL DEFAULT 0,status integer NOT NULL DEFAULT 0,
        has_subtrajectory numeric NOT NULL DEFAULT false,metadata blob,error_details blob,permissions blob,task_details blob,
        render_info blob,step_payload blob,step_format integer NOT NULL DEFAULT 0);
    CREATE INDEX idx_steps_status ON steps(status); CREATE INDEX idx_steps_step_type ON steps(step_type);
    CREATE TABLE gen_metadata (idx integer PRIMARY KEY,data blob,size integer NOT NULL DEFAULT 0);
    CREATE TABLE executor_metadata (idx integer PRIMARY KEY,data blob);
    CREATE TABLE parent_references (idx integer PRIMARY KEY,data blob);
    CREATE TABLE trajectory_metadata_blob (id text PRIMARY KEY DEFAULT 'main',data blob);
    CREATE TABLE battle_mode_infos (idx integer PRIMARY KEY,data blob);
    PRAGMA user_version=1;")?;
    Ok(())
}

fn ensure_summaries(c: &Connection) -> Result<()> {
    c.execute_batch("CREATE TABLE IF NOT EXISTS conversation_summaries (
        conversation_id text PRIMARY KEY,title text NOT NULL DEFAULT '',preview text NOT NULL DEFAULT '',
        step_count integer NOT NULL DEFAULT 0,last_modified_time datetime NOT NULL,workspace_uris text NOT NULL DEFAULT '',
        status text NOT NULL DEFAULT '',source text NOT NULL DEFAULT '',project_id text NOT NULL DEFAULT '',
        agent_name text NOT NULL DEFAULT '',parent_conversation_id text NOT NULL DEFAULT '',nesting_depth integer NOT NULL DEFAULT 0,
        battle_id text NOT NULL DEFAULT '',winning_conversation_id text NOT NULL DEFAULT '',not_fully_idle numeric NOT NULL DEFAULT false,
        killed numeric NOT NULL DEFAULT false,last_user_input_time datetime NOT NULL,last_user_input_step_index integer NOT NULL DEFAULT -1,
        app_data_dir text NOT NULL DEFAULT '',raw_summary blob,group_id text NOT NULL DEFAULT '');
        CREATE TABLE IF NOT EXISTS conversation_parents (conversation_id TEXT PRIMARY KEY,parent_id TEXT NOT NULL) WITHOUT ROWID;")?;
    let has = c
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type='table' AND name='conversations_fts'",
            [],
            |r| r.get::<_, i64>(0),
        )
        .optional()?;
    if has.is_none() {
        let _=c.execute_batch("CREATE VIRTUAL TABLE conversations_fts USING fts5(
        conversation_id UNINDEXED,chunk_index UNINDEXED,chunk_first_step UNINDEXED,chunk_last_step UNINDEXED,
        last_modified_time UNINDEXED,step_count UNINDEXED,last_step_created_at UNINDEXED,archived UNINDEXED,
        user_step_byte_offsets UNINDEXED,agent_step_byte_offsets UNINDEXED,title,workspace,project,user_body,agent_body,
        tokenize='unicode61 remove_diacritics 2',prefix='1 2 3');");
    }
    Ok(())
}

fn update_summaries_proto(root: &Path, is_cli: bool, cid: &str, summary: &[u8]) -> Result<()> {
    let path = root.join(if is_cli {
        "jetbox_summaries_proto.pb"
    } else {
        "agyhub_summaries_proto.pb"
    });
    if !path.is_file() {
        return Ok(());
    }
    let mut bytes = Vec::new();
    for f in pbwire::try_parse_fields(&fs::read(&path)?)? {
        if f.number == 1
            && f.wire_type == 2
            && get(f.bytes().unwrap_or_default(), &[1]) == cid.as_bytes()
        {
            continue;
        }
        bytes.extend(f.raw);
    }
    let entry = [b(1, cid), b(2, summary)].concat();
    bytes.extend(b(1, entry));
    atomic_write(&path, &bytes)
}

pub(super) fn import(
    doc: &Value,
    root: &Path,
    is_cli: bool,
    title: Option<&str>,
    include_tools: bool,
) -> Result<Value> {
    let entries = filtered(doc, include_tools)?;
    let title = title_for(doc, &entries, title);
    let cid = uuid();
    let traj = uuid();
    let now = now_sec();
    let tpl = template(root);
    let instance = if tpl.instance.is_empty() {
        uuid()
    } else {
        tpl.instance
    };
    let mut directory = strip_long(&s(&doc["meta"], "directory"));
    let ws_uri;
    if !directory.is_empty() && !ephemeral(&directory) && Path::new(&directory).is_dir() {
        let norm = directory.replace('\\', "/");
        ws_uri = if norm.starts_with("file://") {
            norm
        } else {
            format!("file:///{}", norm.trim_start_matches('/'))
        };
    } else if is_cli {
        directory = home_dir().to_string_lossy().into_owned();
        ws_uri = format!(
            "file:///{}",
            directory.replace('\\', "/").trim_start_matches('/')
        );
    } else {
        directory.clear();
        ws_uri = String::new();
    }
    let project = if is_cli {
        "default-cli-project"
    } else {
        "outside-of-project"
    };
    let ws_uris = if ws_uri.is_empty() {
        String::new()
    } else {
        serde_json::to_string(&vec![&ws_uri])?
    };
    let times = timestamps(&entries, now);
    let merged = merge(&entries, &times);
    let first = merged.first().map(|e| e.1).unwrap_or(now);
    let last = merged.last().map(|e| e.1).unwrap_or(now);
    let mut last_user_idx = -1i64;
    let mut last_user_sec = first;
    let mut preview = String::new();
    let mut turn_group = uuid();
    let mut guard = ImportGuard::default();
    let conv_dir = root.join("conversations");
    let logs = root.join("brain").join(&cid).join(".system_generated/logs");
    guard.own(root.join("brain").join(&cid));
    guard.own(root.join("annotations").join(format!("{cid}.pbtxt")));
    guard.own(conv_dir.join(format!("{cid}.db")));
    for ext in ["-wal", "-shm", "-journal"] {
        guard.own(conv_dir.join(format!("{cid}.db{ext}")));
    }
    fs::create_dir_all(&conv_dir)?;
    fs::create_dir_all(&logs)?;
    let db = conv_dir.join(format!("{cid}.db"));
    let mut c = connection(&db)?;
    create_schema(&c)?;
    let tx = c.transaction()?;
    let mut lines = Vec::new();
    let mut user_body = Vec::new();
    let mut agent_body = Vec::new();
    let mut user_offs = Vec::new();
    let mut agent_offs = Vec::new();
    for (idx, (entry, sec, thinking)) in merged.iter().enumerate() {
        let kind = nonempty(s(entry, "kind")).unwrap_or_else(|| "assistant".into());
        let text = s(entry, "text");
        let t = ts(*sec);
        let req = uuid_hex();
        if kind == "user" {
            turn_group = uuid();
        }
        let mut traj_ref = b(1, &traj);
        if idx > 0 {
            traj_ref.extend(v(2, idx as u64));
        }
        traj_ref.extend(b(4, &cid));
        let done = b(1, [v(1, 3), b(2, &t)].concat());
        let mut metadata = [
            b(1, &t),
            v(
                3,
                match kind.as_str() {
                    "user" => 4,
                    "assistant" => 2,
                    _ => 5,
                },
            ),
        ]
        .concat();
        if kind == "assistant" {
            metadata.extend(b(6, &t));
            metadata.extend(b(7, &t));
        }
        if kind != "user" {
            metadata.extend(b(8, &t));
        }
        metadata.extend(b(12, &turn_group));
        metadata.extend(b(20, &traj_ref));
        let pair = if kind == "assistant" {
            [b(1, [v(1, 8), b(2, &t)].concat()), done.clone()].concat()
        } else {
            done
        };
        metadata.extend(b(26, &pair));
        if kind == "assistant" {
            metadata.extend(b(32, &t));
        }
        if matches!(kind.as_str(), "user" | "assistant") {
            metadata.extend(b(35, &req));
        }
        let (stype, type_name, source, content, body) = match kind.as_str() {
            "user" => {
                let mut user = [b(2, &text), b(3, b(1, &text)), b(4, [])].concat();
                if !tpl.user_cfg.is_empty() {
                    user.extend(b(12, &tpl.user_cfg));
                }
                user.extend(b(19, uuid()));
                last_user_idx = idx as i64;
                last_user_sec = *sec;
                if preview.is_empty() {
                    preview = shorten(&text.split_whitespace().collect::<Vec<_>>().join(" "), 300);
                }
                user_offs.push([idx, user_body.len()]);
                user_body.extend_from_slice(text.as_bytes());
                user_body.push(b'\n');
                (
                    14,
                    "USER_INPUT",
                    "USER_EXPLICIT",
                    format!("<USER_REQUEST>\n{text}\n</USER_REQUEST>"),
                    b(19, user),
                )
            }
            "assistant" => {
                let mut asst = Vec::new();
                if !text.is_empty() {
                    asst.extend(b(1, &text));
                }
                if !thinking.is_empty() {
                    asst.extend(b(3, thinking));
                }
                asst.extend(b(6, format!("bot-{}", uuid())));
                if !text.is_empty() {
                    asst.extend(b(8, &text));
                    agent_offs.push([idx, agent_body.len()]);
                    agent_body.extend_from_slice(text.as_bytes());
                    agent_body.push(b'\n');
                }
                asst.extend(v(12, 2));
                (15, "PLANNER_RESPONSE", "MODEL", text.clone(), b(20, asst))
            }
            "compact" => (
                23,
                "CHECKPOINT",
                "SYSTEM",
                text.clone(),
                nested(&[30, 5], text.as_bytes()),
            ),
            "error" => (
                17,
                "ERROR_MESSAGE",
                "SYSTEM",
                text.clone(),
                nested(&[24, 3, 2], text.as_bytes()),
            ),
            "tool" => (
                132,
                "GENERIC",
                "SYSTEM",
                text.clone(),
                nested(&[140, 2, 1], text.as_bytes()),
            ),
            _ => (
                101,
                "SYSTEM_MESSAGE",
                "SYSTEM",
                text.clone(),
                nested(&[114, 1], text.as_bytes()),
            ),
        };
        let payload = [v(1, stype), v(4, 3), b(5, &metadata), body].concat();
        tx.execute("INSERT INTO steps(idx,step_type,status,has_subtrajectory,metadata,error_details,permissions,task_details,render_info,step_payload,step_format) VALUES (?,?,3,0,?,NULL,NULL,NULL,NULL,?,0)",params![idx as i64,stype as i64,metadata,payload])?;
        let mut line = json!({"step_index":idx,"source":source,"type":type_name,"status":"DONE","created_at":iso(*sec),"content":content});
        if !thinking.is_empty() {
            line["thinking"] = json!(thinking);
        }
        if kind == "tool" && !s(entry, "tool").is_empty() {
            line["tool_calls"] = json!([{"name":s(entry,"tool")}]);
        }
        lines.push(line);
    }
    let workspace = if ws_uri.is_empty() {
        Vec::new()
    } else {
        [b(1, &ws_uri), b(3, [])].concat()
    };
    let mut summary_meta = Vec::new();
    if !workspace.is_empty() {
        summary_meta.extend(b(1, &workspace));
    }
    summary_meta.extend(b(2, ts(first)));
    summary_meta.extend(b(3, &instance));
    summary_meta.extend(b(6, &cid));
    if !ws_uri.is_empty() {
        summary_meta.extend(b(7, &ws_uri));
    }
    let mut trajectory_metadata = summary_meta.clone();
    if !tpl.cap.is_empty() {
        trajectory_metadata.extend(b(15, &tpl.cap));
    }
    trajectory_metadata.extend(b(18, project));
    summary_meta.extend(b(18, project));
    tx.execute("INSERT INTO trajectory_meta(trajectory_id,cascade_id,trajectory_type,source) VALUES (?,?,4,?)",params![traj,cid,if is_cli{17}else{1}])?;
    tx.execute(
        "INSERT OR REPLACE INTO trajectory_metadata_blob(id,data) VALUES ('main',?)",
        params![trajectory_metadata],
    )?;
    tx.commit()?;
    let _ = c.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);");
    drop(c);
    let mut transcript = Vec::new();
    for line in &lines {
        serde_json::to_writer(&mut transcript, line)?;
        transcript.push(b'\n');
    }
    for name in ["transcript_full", "transcript"] {
        atomic_write(&logs.join(format!("{name}.jsonl")), &transcript)?;
        let chunks = logs.join("chunks").join(name);
        fs::create_dir_all(&chunks)?;
        for (idx, part) in transcript.chunks(102400).enumerate() {
            atomic_write(&chunks.join(format!("{idx:08}.jsonl")), part)?;
        }
    }
    let escaped = title
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace(['\r', '\n'], " ");
    let whole = last.max(0.0) as u64;
    let nanos = ((last - whole as f64) * 1e9).clamp(0.0, 999_999_999.0) as u64;
    let _ = atomic_write(
        &root.join("annotations").join(format!("{cid}.pbtxt")),
        format!("title:\"{escaped}\" last_user_view_time:{{seconds:{whole} nanos:{nanos}}}\n")
            .as_bytes(),
    );
    let annotations = [b(1, &title), b(7, ts(last))].concat();
    let mut summary = [
        b(1, &title),
        v(2, merged.len() as u64),
        b(3, ts(last)),
        b(4, &traj),
        v(5, 1),
        b(7, ts(first)),
    ]
    .concat();
    if !workspace.is_empty() {
        summary.extend(b(9, &workspace));
    }
    summary.extend(b(10, ts(last_user_sec)));
    summary.extend(b(15, annotations));
    if last_user_idx >= 0 {
        summary.extend(v(16, last_user_idx as u64));
    }
    summary.extend(b(17, summary_meta));
    summary.extend(v(22, 4));
    summary.extend(v(28, if is_cli { 2 } else { 4 }));
    let mut sc = connection(&root.join("conversation_summaries.db"))?;
    ensure_summaries(&sc)?;
    let stx = sc.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    stx.execute("INSERT OR REPLACE INTO conversation_summaries(conversation_id,title,preview,step_count,last_modified_time,workspace_uris,status,source,project_id,agent_name,parent_conversation_id,nesting_depth,battle_id,winning_conversation_id,not_fully_idle,killed,last_user_input_time,last_user_input_step_index,app_data_dir,raw_summary,group_id) VALUES (?,?,?,?,?,?,?,'',?,'','',0,'','',0,0,?,?,?,?,'')",
        params![cid,title,if preview.is_empty(){&title}else{&preview},merged.len() as i64,ag_dt(last),ws_uris,"CASCADE_RUN_STATUS_IDLE",project,ag_dt(last_user_sec),last_user_idx,if is_cli{"antigravity-cli"}else{"antigravity"},summary])?;
    let has_fts = stx
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type='table' AND name='conversations_fts'",
            [],
            |r| r.get::<_, i64>(0),
        )
        .optional()?
        .is_some();
    if has_fts {
        stx.execute(
            "DELETE FROM conversations_fts WHERE conversation_id=?",
            [&cid],
        )?;
        stx.execute("INSERT INTO conversations_fts(conversation_id,chunk_index,chunk_first_step,chunk_last_step,last_modified_time,step_count,last_step_created_at,archived,user_step_byte_offsets,agent_step_byte_offsets,title,workspace,project,user_body,agent_body) VALUES (?,0,0,?,?,?,?,0,?,?,?,?,?,?,?)",
            params![cid,merged.len().saturating_sub(1) as i64,ag_dt(last),merged.len() as i64,ag_dt(last),serde_json::to_string(&user_offs)?,serde_json::to_string(&agent_offs)?,title,directory,project,String::from_utf8_lossy(&user_body),String::from_utf8_lossy(&agent_body)])?;
    }
    stx.commit()?;
    let _ = update_summaries_proto(root, is_cli, &cid, &summary);
    guard.commit();
    Ok(
        json!({"provider":if is_cli{"antigravity_cli"}else{"antigravity"},"cid":cid,"title":title,"entries":merged.len()}),
    )
}
