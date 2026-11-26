use super::*;
use rusqlite::{params, OptionalExtension};
use sha1::{Digest, Sha1};

fn id(prefix: &str, ts_ms: i64, counter: i64, descending: bool) -> String {
    let mut val = ((ts_ms as u64).wrapping_mul(4096) + (counter as u64 & 4095)) & 0xffffffffffff;
    if descending {
        val = 0xffffffffffff - val;
    }
    let random = Uuid::new_v4();
    let alphabet = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
    let tail: String = random
        .as_bytes()
        .iter()
        .take(14)
        .map(|b| alphabet[(*b % 62) as usize] as char)
        .collect();
    format!("{prefix}_{val:012x}{tail}")
}
fn ensure_schema(c: &Connection) -> Result<()> {
    c.execute_batch("CREATE TABLE IF NOT EXISTS project (
        id TEXT PRIMARY KEY,worktree TEXT NOT NULL,vcs TEXT,name TEXT,icon_url TEXT,icon_url_override TEXT,icon_color TEXT,
        time_created INTEGER NOT NULL,time_updated INTEGER NOT NULL,time_initialized INTEGER,time_active INTEGER NOT NULL DEFAULT 0,
        time_archived INTEGER,sandboxes TEXT NOT NULL DEFAULT '[]',commands TEXT);
        CREATE TABLE IF NOT EXISTS session_v2 (
        id TEXT PRIMARY KEY,project_id TEXT NOT NULL,workspace_id TEXT,parent_id TEXT,fork_session_id TEXT,fork_boundary TEXT,
        slug TEXT NOT NULL,directory TEXT NOT NULL,path TEXT,title TEXT,version TEXT NOT NULL,share_url TEXT,
        summary_additions INTEGER,summary_deletions INTEGER,summary_files INTEGER,summary_diffs TEXT,metadata TEXT,
        cost REAL NOT NULL DEFAULT 0,tokens_input INTEGER NOT NULL DEFAULT 0,tokens_output INTEGER NOT NULL DEFAULT 0,
        tokens_reasoning INTEGER NOT NULL DEFAULT 0,tokens_cache_read INTEGER NOT NULL DEFAULT 0,tokens_cache_write INTEGER NOT NULL DEFAULT 0,
        revert TEXT,permission TEXT,agent TEXT,model TEXT,time_created INTEGER NOT NULL,time_updated INTEGER NOT NULL,
        time_busy INTEGER,time_idle INTEGER,time_viewed INTEGER,idle_outcome TEXT,time_compacting INTEGER,time_archived INTEGER,
        time_suspended INTEGER,resume_attempts INTEGER NOT NULL DEFAULT 0);
        CREATE TABLE IF NOT EXISTS session_message (id TEXT PRIMARY KEY,session_id TEXT NOT NULL,type TEXT NOT NULL,
        seq INTEGER NOT NULL,time_created INTEGER NOT NULL,time_updated INTEGER NOT NULL,data TEXT NOT NULL);
        CREATE TABLE IF NOT EXISTS event_sequence (aggregate_id TEXT PRIMARY KEY,seq INTEGER NOT NULL,owner_id TEXT);")?;
    Ok(())
}
fn template(c: &Connection) -> Value {
    let mut tpl = json!({"version":"2.2.5","agent":"build","model_id":"nemotron-3.5-super-vl-free","provider_id":"opencode","variant":"default"});
    if let Ok((version,agent,model))=c.query_row("SELECT version,agent,model FROM session_v2 WHERE agent IS NOT NULL AND model IS NOT NULL AND slug NOT LIKE 'imported-%' ORDER BY time_updated DESC LIMIT 1",[],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?))){
        if !version.is_empty(){tpl["version"]=json!(version);}if !agent.is_empty(){tpl["agent"]=json!(agent);}
        if let Ok(m)=serde_json::from_str::<Value>(&model){
            if !s(&m,"id").is_empty(){tpl["model_id"]=m["id"].clone();}
            if !s(&m,"providerID").is_empty(){tpl["provider_id"]=m["providerID"].clone();}
        }
    }
    if let Ok(data)=c.query_row("SELECT data FROM session_message WHERE type='assistant' AND data LIKE '%\"variant\"%' ORDER BY time_created DESC LIMIT 1",[],|r|r.get::<_,String>(0)){
        if let Ok(data)=serde_json::from_str::<Value>(&data){if !s(&data["model"],"variant").is_empty(){tpl["variant"]=data["model"]["variant"].clone();}}
    }
    tpl
}
struct Writer<'a> {
    c: &'a Connection,
    sid: &'a str,
    agent: &'a str,
    model: Value,
    tokens: Value,
    seq: i64,
    counter: i64,
    prev: Option<String>,
    parts: Vec<Value>,
    start: Option<i64>,
    end: Option<i64>,
    first: i64,
}
impl Writer<'_> {
    fn insert(&mut self, mid: &str, kind: &str, start: i64, end: i64, data: &Value) -> Result<()> {
        self.c.execute("INSERT INTO session_message(id,session_id,type,seq,time_created,time_updated,data) VALUES (?,?,?,?,?,?,?)",
            params![mid,self.sid,kind,self.seq,start,end,serde_json::to_string(data)?])?;
        self.seq += 1;
        self.prev = Some(mid.to_string());
        Ok(())
    }
    fn next_id(&mut self, ms: i64) -> String {
        let out = id("msg", ms, self.counter, false);
        self.counter += 1;
        out
    }
    fn part(&mut self, part: Value, ms: i64) {
        if self.start.is_none() {
            self.start = Some(ms);
        }
        self.end = Some(ms);
        self.parts.push(part);
    }
    fn has_tool(&self) -> bool {
        self.parts.iter().any(|p| p["type"] == "tool")
    }
    fn flush(&mut self) -> Result<()> {
        if self.parts.is_empty() {
            return Ok(());
        }
        let start = self.start.unwrap_or(self.first);
        let end = self.end.unwrap_or(start);
        let tool = self.has_tool();
        let mid = self.next_id(start);
        let data = json!({"time":{"created":start,"streamed":end,"completed":end},"agent":self.agent,"model":self.model,
            "content":self.parts,"finish":if tool{"tool-calls"}else{"stop"},"rawFinish":if tool{"tool_calls"}else{"stop"},
            "cost":0,"tokens":self.tokens});
        self.insert(&mid, "assistant", start, end, &data)?;
        self.parts.clear();
        self.start = None;
        self.end = None;
        Ok(())
    }
}
pub(super) fn import(
    doc: &Value,
    db: &Path,
    title: Option<&str>,
    include_tools: bool,
) -> Result<Value> {
    let entries = filtered(doc, include_tools)?;
    let title = title_for(doc, &entries, title);
    let mut meta_dir = strip_long(&s(&doc["meta"], "directory")).replace('\\', "/");
    if ephemeral(&meta_dir) {
        meta_dir.clear();
    }
    let now = now_sec();
    let now_ms = (now * 1000.0) as i64;
    let times = timestamps(&entries, now);
    let first = (times[0] * 1000.0) as i64;
    let mut last = first.max((times[times.len() - 1] * 1000.0) as i64);
    let sid = id("ses", now_ms, 1, true);
    let slug = format!("imported-{}", &uuid_hex()[..8]);
    let mut c = connection(db)?;
    ensure_schema(&c)?;
    let tpl = template(&c);
    let agent = s(&tpl, "agent");
    let model_id = s(&tpl, "model_id");
    let provider = s(&tpl, "provider_id");
    let model = json!({"id":model_id,"providerID":provider,"variant":tpl["variant"]});
    let user_model = json!({"modelID":model_id,"providerID":provider});
    let session_model = serde_json::to_string(&json!({"id":model_id,"providerID":provider}))?;
    let tokens = json!({"input":0,"output":0,"reasoning":0,"cache":{"read":0,"write":0}});
    let tx = c.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    let mut project = None;
    if !meta_dir.is_empty() {
        project = tx
            .query_row(
                "SELECT id,worktree FROM project WHERE LOWER(worktree)=LOWER(?) LIMIT 1",
                [&meta_dir],
                |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)),
            )
            .optional()?;
    }
    if project.is_none() {
        project=tx.query_row("SELECT id,worktree FROM project WHERE time_archived IS NULL ORDER BY time_active DESC,time_updated DESC LIMIT 1",[],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?))).optional()?;
    }
    if project.is_none() {
        project = tx
            .query_row(
                "SELECT id,worktree FROM project ORDER BY time_updated DESC LIMIT 1",
                [],
                |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)),
            )
            .optional()?;
    }
    let fallback = if meta_dir.is_empty() {
        home_dir().to_string_lossy().replace('\\', "/")
    } else {
        meta_dir.clone()
    };
    let (project_id, directory) = if let Some((pid, worktree)) = project {
        (
            pid,
            if worktree.is_empty() {
                fallback
            } else {
                worktree
            },
        )
    } else {
        let pid = hex::encode(Sha1::digest(fallback.as_bytes()));
        tx.execute("INSERT INTO project(id,worktree,time_created,time_updated,time_active,sandboxes) VALUES (?,?,?,?,?,'[]')",params![pid,fallback,now_ms,now_ms,now_ms])?;
        (pid, fallback)
    };
    tx.execute("INSERT INTO session_v2(id,project_id,slug,directory,path,title,version,cost,tokens_input,tokens_output,tokens_reasoning,tokens_cache_read,tokens_cache_write,permission,agent,model,time_created,time_updated,time_busy,time_idle,time_viewed,idle_outcome,resume_attempts) VALUES (?,?,?,?,'',?,?,0,0,0,0,0,0,'[]',?,?,?,?,?,?,?,'succeeded',0)",
        params![sid,project_id,slug,directory,title,s(&tpl,"version"),agent,session_model,first,last.max(now_ms),first,last,last])?;
    let mut w = Writer {
        c: &tx,
        sid: &sid,
        agent: &agent,
        model: model.clone(),
        tokens: tokens.clone(),
        seq: 1,
        counter: 1,
        prev: None,
        parts: Vec::new(),
        start: None,
        end: None,
        first,
    };
    let mut i = 0usize;
    while i < entries.len() {
        let e = &entries[i];
        let time = times[i];
        let ms = (time * 1000.0) as i64;
        last = last.max(ms);
        let kind = nonempty(s(e, "kind")).unwrap_or_else(|| "assistant".into());
        let text = s(e, "text");
        if kind == "assistant" {
            i += 1;
            if w.has_tool() {
                w.flush()?;
            }
            w.part(json!({"type":"text","text":text}), ms);
            continue;
        }
        if kind == "tool" && e["subtype"] == "thinking" {
            i += 1;
            if !text.trim().is_empty() {
                if w.has_tool() {
                    w.flush()?;
                }
                w.part(
                    json!({"type":"reasoning","text":text,"time":{"created":ms,"completed":ms}}),
                    ms,
                );
            }
            continue;
        }
        if kind == "tool" {
            let inv = tools::consume_tool_pair(&entries, &times, &mut i);
            let call_ms = (inv.ts_call * 1000.0) as i64;
            let res_ms = ((inv.ts_result * 1000.0) as i64).max(call_ms);
            last = last.max(res_ms);
            let (name, input, meta): (String, Value, Value) = match inv.canonical {
                tools::CanonicalTool::Shell { command, .. } => (
                    "shell".into(),
                    json!({"command":command}),
                    json!({"status":"completed","truncated":false,"exit":if inv.is_error{1}else{0}}),
                ),
                tools::CanonicalTool::Read {
                    path,
                    start_line,
                    end_line,
                } => {
                    let mut inp = json!({"path":path});
                    if let Some(s) = start_line {
                        inp["offset"] = json!(s);
                        if let Some(end) = end_line.filter(|end| *end >= s) {
                            inp["limit"] = json!(end - s + 1);
                        }
                    }
                    ("read".into(), inp, json!({"truncated":false}))
                }
                tools::CanonicalTool::Write { path, content } => (
                    "write".into(),
                    json!({"path":path,"content":content}),
                    json!({"truncated":false}),
                ),
                tools::CanonicalTool::Edit {
                    path,
                    old_string,
                    new_string,
                    diff,
                    content,
                } => {
                    if !old_string.is_empty() {
                        let udiff = diff
                            .unwrap_or_else(|| tools::build_unified_diff(&old_string, &new_string));
                        let (adds, dels) = tools::count_diff_stats(&udiff);
                        (
                            "edit".into(),
                            json!({"path":path,"oldString":old_string,"newString":new_string}),
                            json!({"files":[{"file":path,"patch":udiff,"status":"modified","additions":adds,"deletions":dels}],"truncated":false}),
                        )
                    } else {
                        let body = content.unwrap_or(new_string);
                        (
                            "write".into(),
                            json!({"path":path,"content":body}),
                            json!({"truncated":false}),
                        )
                    }
                }
                tools::CanonicalTool::Delete { path } => (
                    "shell".into(),
                    json!({"command":format!("rm -f {}", tools::shell_quote(&path))}),
                    json!({"status":"completed","truncated":false,"exit":if inv.is_error{1}else{0}}),
                ),
                tools::CanonicalTool::ListDir { path, pattern } => (
                    "glob".into(),
                    json!({"pattern":pattern.unwrap_or_else(|| "*".into()),"path":path}),
                    json!({"truncated":false}),
                ),
                tools::CanonicalTool::Grep { pattern, path } => {
                    let mut inp = json!({"pattern":pattern});
                    if let Some(p) = path.filter(|s| !s.is_empty()) {
                        inp["path"] = json!(p);
                    }
                    let matches = inv
                        .result_text
                        .lines()
                        .filter(|l| !l.trim().is_empty())
                        .count();
                    (
                        "grep".into(),
                        inp,
                        json!({"matches":matches,"truncated":false}),
                    )
                }
                tools::CanonicalTool::WebSearch { query } => (
                    "websearch".into(),
                    json!({"query":query}),
                    json!({"truncated":false}),
                ),
                tools::CanonicalTool::Generic { name, args } => (
                    name.to_ascii_lowercase(),
                    if args.is_object() { args } else { json!({}) },
                    json!({"status":"completed","truncated":false}),
                ),
            };
            let state = if inv.is_error {
                json!({"status":"error","input":input,"error":{"type":"aborted","message":if inv.result_text.is_empty(){"Tool error"}else{&inv.result_text}}})
            } else {
                json!({"status":"completed","input":input,"content":[{"type":"text","text":inv.result_text}],"metadata":meta})
            };
            w.part(
                json!({"type":"tool","id":format!("chatcmpl-tool-{}", &uuid_hex()[..16]),"name":name,"executed":false,"state":state,"time":{"created":call_ms,"ran":call_ms,"completed":res_ms}}),
                call_ms,
            );
            continue;
        }
        i += 1;
        w.flush()?;
        let mid = w.next_id(ms);
        let (typ, data) = match kind.as_str() {
            "user" => (
                "user",
                json!({"metadata":{"displayText":text,"comments":[],"attachments":[],"agent":agent,"model":user_model},"time":{"created":ms},"text":text,"files":[],"agents":[]}),
            ),
            "compact" => (
                "compaction",
                json!({"metadata":{"keepID":w.prev.as_deref().unwrap_or(&mid)},"time":{"created":ms},"status":"completed","reason":"auto","model":model,"summary":text}),
            ),
            "error" => (
                "assistant",
                json!({"time":{"created":ms,"completed":ms},"agent":agent,"model":model,"content":[],"finish":"error","error":{"type":"aborted","message":if text.is_empty(){"Error"}else{&text}},"cost":0,"tokens":tokens}),
            ),
            _ => (
                "system",
                json!({"metadata":{"notice":"instructions","instructionSources":[]},"time":{"created":ms},"text":text,"description":nonempty(s(e,"label")).unwrap_or_else(||"Imported context".into())}),
            ),
        };
        w.insert(&mid, typ, ms, ms, &data)?;
    }
    w.flush()?;
    let idle = w.next_id(last);
    w.insert(
        &idle,
        "idle",
        last,
        last,
        &json!({"time":{"created":last},"outcome":"succeeded"}),
    )?;
    let sequence = w.seq;
    drop(w);
    tx.execute(
        "INSERT OR REPLACE INTO event_sequence(aggregate_id,seq,owner_id) VALUES (?,?,NULL)",
        params![sid, sequence],
    )?;
    tx.execute(
        "UPDATE session_v2 SET time_updated=?,time_idle=?,time_viewed=? WHERE id=?",
        params![last.max(now_ms), last, last, sid],
    )?;
    tx.commit()?;
    Ok(json!({"provider":"opencode","cid":sid,"title":title,"entries":entries.len()}))
}
