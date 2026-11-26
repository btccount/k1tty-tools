use k1tty_core::{
    converter,
    service::{AppPaths, Service},
};
use serde_json::{json, Value};
use std::{fs, sync::Arc};

fn flatten_items(resp: &Value) -> Vec<Value> {
    resp["turns"]
        .as_array()
        .into_iter()
        .flatten()
        .flat_map(|t| t["items"].as_array().into_iter().flatten())
        .cloned()
        .collect()
}

fn build_large_doc() -> (Value, String, String) {
    let mut entries = Vec::with_capacity(420);
    entries.push(json!({
        "kind": "user",
        "subtype": "prompt",
        "text": "Start of huge chat early_needle_step_zero",
        "ts": "2026-05-01T10:00:00Z"
    }));
    for i in 0..205 {
        entries.push(json!({
            "kind": "user",
            "subtype": "prompt",
            "text": format!("User turn number {i} with regular text payload"),
            "ts": format!("2026-05-01T10:{:02}:{:02}Z", (i / 60) % 60, i % 60)
        }));
        entries.push(json!({
            "kind": "assistant",
            "subtype": "reply",
            "text": format!("Assistant reply number {i} answering the user turn"),
            "ts": format!("2026-05-01T10:{:02}:{:02}Z", (i / 60) % 60, i % 60)
        }));
    }
    let big_tool_text = format!("TOOL_START_{}_TOOL_TAIL_NEEDLE_999", "я".repeat(60_000));
    let big_reply_text = format!("REPLY_START_{}_REPLY_TAIL_NEEDLE_888", "A".repeat(145_000));
    entries.push(json!({
        "kind": "assistant",
        "subtype": "reply",
        "text": big_reply_text,
        "ts": "2026-05-01T11:00:00Z"
    }));
    entries.push(json!({
        "kind": "tool",
        "subtype": "thinking",
        "tool": "thinking",
        "label": "thinking",
        "status": "ok",
        "text": big_tool_text,
        "ts": "2026-05-01T11:00:01Z"
    }));
    let doc = json!({
        "meta": {
            "source_provider": "fixture",
            "source_id": "huge-synthetic",
            "title": "Large Synthetic Chat",
            "directory": ""
        },
        "entries": entries
    });
    (doc, big_tool_text, big_reply_text)
}

#[test]
fn large_synthetic_chat_streams_truncates_searches_and_fetches_full_entry_text() {
    let dir = tempfile::tempdir().unwrap();
    let svc = Service::new(AppPaths::isolated(dir.path()), Arc::new(|_| {})).unwrap();
    let (doc, big_tool_text, big_reply_text) = build_large_doc();
    assert!(big_tool_text.len() > 100 * 1024);
    assert!(big_reply_text.len() > 128 * 1024);

    for pid in ["claude", "antigravity", "opencode"] {
        let prov = svc.provider(pid).unwrap();
        let imported = converter::import_doc_to_provider(&doc, prov.as_ref(), None, true).unwrap();
        let cid = imported["cid"].as_str().unwrap().to_owned();

        let mut visited = 0usize;
        prov.visit_entries(&cid, &mut |_| {
            visited += 1;
            true
        })
        .unwrap();
        assert!(
            visited >= 412,
            "{pid}: expected >= 412 visited, got {visited}"
        );

        let mut stopped = 0usize;
        prov.visit_entries(&cid, &mut |_| {
            stopped += 1;
            stopped < 5
        })
        .unwrap();
        assert_eq!(stopped, 5, "{pid}: early stop failed");

        let page = svc
            .rpc(
                "entries",
                Some(pid),
                json!({"cid": cid, "include_tools": true, "limit": 10}),
            )
            .unwrap();
        let items = flatten_items(&page);

        let tool_item = items
            .iter()
            .find(|it| {
                it["kind"] == "tool"
                    && it["text"]
                        .as_str()
                        .is_some_and(|s| s.contains("TOOL_START_"))
            })
            .unwrap_or_else(|| panic!("{pid}: missing big tool item in page"));
        assert_eq!(
            tool_item["text_truncated"], true,
            "{pid}: tool not truncated"
        );
        let tool_full_bytes = tool_item["full_bytes"].as_u64().unwrap() as usize;
        assert!(
            tool_full_bytes >= big_tool_text.len(),
            "{pid}: tool full_bytes {tool_full_bytes} < {}",
            big_tool_text.len()
        );
        assert!(
            tool_item["text"].as_str().unwrap().len() <= 32 * 1024,
            "{pid}: truncated tool exceeded 32 KB"
        );

        let reply_item = items
            .iter()
            .find(|it| {
                it["kind"] == "assistant"
                    && it["text"]
                        .as_str()
                        .is_some_and(|s| s.contains("REPLY_START_"))
            })
            .unwrap_or_else(|| panic!("{pid}: missing big reply item in page"));
        assert_eq!(
            reply_item["text_truncated"], true,
            "{pid}: reply not truncated"
        );
        assert_eq!(
            reply_item["full_bytes"].as_u64().unwrap() as usize,
            big_reply_text.len(),
            "{pid}: reply full_bytes mismatch"
        );
        assert!(
            reply_item["text"].as_str().unwrap().len() <= 128 * 1024,
            "{pid}: truncated reply exceeded 128 KB"
        );

        let small_item = items
            .iter()
            .find(|it| {
                it["kind"] == "user"
                    && it["text"]
                        .as_str()
                        .is_some_and(|s| s.contains("User turn number"))
            })
            .unwrap_or_else(|| panic!("{pid}: missing small user item"));
        assert!(
            small_item.get("text_truncated").is_none(),
            "{pid}: small item must not set text_truncated"
        );
        assert!(
            small_item.get("full_bytes").is_none(),
            "{pid}: small item must not set full_bytes"
        );

        let tool_key = tool_item["key"].as_str().unwrap();
        let full_tool = svc
            .rpc(
                "entry_text",
                Some(pid),
                json!({"cid": cid, "key": tool_key}),
            )
            .unwrap();
        assert_eq!(full_tool["ok"], true);
        assert_eq!(full_tool["key"], tool_key);
        let fetched_tool = full_tool["text"].as_str().unwrap();
        assert!(
            fetched_tool.contains("TOOL_TAIL_NEEDLE_999"),
            "{pid}: entry_text missing tail needle"
        );
        assert_eq!(
            fetched_tool.len(),
            tool_full_bytes,
            "{pid}: full tool length mismatch"
        );

        let reply_key = reply_item["key"].as_str().unwrap();
        let full_reply = svc
            .rpc(
                "entry_text",
                Some(pid),
                json!({"cid": cid, "key": reply_key}),
            )
            .unwrap();
        assert_eq!(full_reply["ok"], true);
        assert_eq!(full_reply["text"].as_str().unwrap(), big_reply_text);

        let stats = svc
            .rpc("context_stats", Some(pid), json!({"cid": cid}))
            .unwrap();
        assert!(
            stats["total_steps"].as_u64().unwrap() >= 412,
            "{pid}: context_stats total_steps too small"
        );
        assert!(
            stats["total_bytes"].as_u64().unwrap() as usize
                >= big_tool_text.len() + big_reply_text.len(),
            "{pid}: context_stats must count full untruncated bytes"
        );
        let heaviest = stats["heaviest"].as_array().unwrap();
        assert!(!heaviest.is_empty() && heaviest.len() <= 5);
        assert!(
            heaviest[0]["bytes"].as_u64().unwrap() as usize >= big_reply_text.len(),
            "{pid}: heaviest[0] bytes mismatch"
        );
        assert!(heaviest[0]["preview"].as_str().unwrap().chars().count() <= 111);

        let tail_search = svc
            .rpc(
                "search_thread",
                Some(pid),
                json!({"cid": cid, "query": "tool_tail_needle_999"}),
            )
            .unwrap();
        assert_eq!(tail_search["total"], 1, "{pid}: tail needle search failed");

        let early_search = svc
            .rpc(
                "search_thread",
                Some(pid),
                json!({"cid": cid, "query": "early_needle_step_zero"}),
            )
            .unwrap();
        assert_eq!(early_search["total"], 1, "{pid}: early step search failed");

        if matches!(pid, "claude" | "codex") {
            let no_tools = svc
                .rpc(
                    "entries",
                    Some(pid),
                    json!({"cid": cid, "include_tools": false, "limit": 10}),
                )
                .unwrap();
            let nt_items = flatten_items(&no_tools);
            assert!(
                nt_items.iter().all(|it| it["kind"] != "tool"),
                "{pid}: include_tools=false leaked tool item"
            );
            assert!(
                nt_items.iter().any(|it| it["text"]
                    .as_str()
                    .is_some_and(|s| s.contains("REPLY_START_"))),
                "{pid}: include_tools=false dropped assistant reply"
            );
        }

        if pid == "antigravity" {
            let logs = prov
                .paths()
                .root
                .join("brain")
                .join(&cid)
                .join(".system_generated")
                .join("logs");
            fs::remove_dir_all(&logs).unwrap();
            let mut db_visited = 0usize;
            let mut found_reply = false;
            prov.visit_entries(&cid, &mut |e| {
                db_visited += 1;
                if e.text.contains("REPLY_TAIL_NEEDLE_888") {
                    found_reply = true;
                }
                true
            })
            .unwrap();
            assert!(db_visited >= 400);
            assert!(found_reply);
            let from_db = prov.read_entry_text(&cid, reply_key).unwrap().unwrap();
            assert_eq!(from_db, big_reply_text);
        }
    }
}

#[test]
fn codex_native_large_tool_output_fast_skip_truncation_and_entry_text() {
    let dir = tempfile::tempdir().unwrap();
    let paths = AppPaths::isolated(dir.path());
    let sess_dir = paths
        .providers
        .iter()
        .find(|p| p.id == "codex")
        .unwrap()
        .root
        .join("sessions/2026/05/01");
    fs::create_dir_all(&sess_dir).unwrap();
    let rollout = sess_dir.join("rollout-2026-05-01-huge-cx.jsonl");
    let big_output = format!("CX_TOOL_HEAD_{}_CX_TOOL_TAIL_777", "к".repeat(65_000));
    let big_reply = format!("CX_REPLY_HEAD_{}_CX_REPLY_TAIL_666", "B".repeat(145_000));
    let mut raw = String::new();
    let mut ord = 0u64;
    let push_row = |buf: &mut String, val: Value| {
        buf.push_str(&serde_json::to_string(&val).unwrap());
        buf.push('\n');
    };
    push_row(
        &mut raw,
        json!({"ordinal": ord, "type": "session_meta", "payload": {"id": "huge-cx", "cwd": "/tmp"}}),
    );
    ord += 1;
    push_row(
        &mut raw,
        json!({"ordinal": ord, "type": "response_item", "timestamp": "2026-05-01T09:59:59Z", "payload": {"type": "message", "role": "user", "id": "u-first", "content": [{"type": "input_text", "text": "Early prompt cx_early_needle_0"}]}}),
    );
    ord += 1;
    for i in 0..205 {
        push_row(
            &mut raw,
            json!({"ordinal": ord, "type": "response_item", "timestamp": "2026-05-01T10:00:00Z", "payload": {"type": "message", "role": "user", "id": format!("u{i}"), "content": [{"type": "input_text", "text": format!("Prompt {i}")}]}}),
        );
        ord += 1;
        push_row(
            &mut raw,
            json!({"ordinal": ord, "type": "response_item", "timestamp": "2026-05-01T10:00:01Z", "payload": {"type": "message", "role": "assistant", "id": format!("a{i}"), "content": [{"type": "output_text", "text": format!("Answer {i}")}]}}),
        );
        ord += 1;
    }
    push_row(
        &mut raw,
        json!({"ordinal": ord, "type": "response_item", "timestamp": "2026-05-01T11:00:00Z", "payload": {"type": "function_call_output", "output": big_output, "status": "ok"}}),
    );
    ord += 1;
    push_row(
        &mut raw,
        json!({"ordinal": ord, "type": "response_item", "timestamp": "2026-05-01T11:00:01Z", "payload": {"type": "message", "role": "assistant", "id": "final", "content": [{"type": "output_text", "text": big_reply}]}}),
    );
    fs::write(&rollout, raw).unwrap();

    let svc = Service::new(paths, Arc::new(|_| {})).unwrap();
    let no_tools = svc
        .rpc(
            "entries",
            Some("codex"),
            json!({"cid": "huge-cx", "include_tools": false, "limit": 5}),
        )
        .unwrap();
    let nt_items = flatten_items(&no_tools);
    assert_eq!(nt_items.len(), 5);
    assert!(nt_items.iter().all(|it| it["kind"] != "tool"));

    let with_tools = svc
        .rpc(
            "entries",
            Some("codex"),
            json!({"cid": "huge-cx", "include_tools": true, "limit": 5}),
        )
        .unwrap();
    let wt_items = flatten_items(&with_tools);
    let tool_it = wt_items
        .iter()
        .find(|it| it["kind"] == "tool")
        .expect("huge function_call_output");
    assert_eq!(tool_it["text_truncated"], true);
    assert_eq!(
        tool_it["full_bytes"].as_u64().unwrap() as usize,
        big_output.len()
    );
    assert!(tool_it["text"].as_str().unwrap().len() <= 32 * 1024);

    let reply_it = wt_items
        .iter()
        .find(|it| {
            it["kind"] == "assistant"
                && it["text"]
                    .as_str()
                    .is_some_and(|s| s.contains("CX_REPLY_HEAD_"))
        })
        .expect("huge assistant reply");
    assert_eq!(reply_it["text_truncated"], true);
    assert_eq!(
        reply_it["full_bytes"].as_u64().unwrap() as usize,
        big_reply.len()
    );
    assert!(reply_it["text"].as_str().unwrap().len() <= 128 * 1024);

    let full = svc
        .rpc(
            "entry_text",
            Some("codex"),
            json!({"cid": "huge-cx", "key": tool_it["key"]}),
        )
        .unwrap();
    assert_eq!(full["text"].as_str().unwrap(), big_output);

    let full_reply = svc
        .rpc(
            "entry_text",
            Some("codex"),
            json!({"cid": "huge-cx", "key": reply_it["key"]}),
        )
        .unwrap();
    assert_eq!(full_reply["text"].as_str().unwrap(), big_reply);

    let search = svc
        .rpc(
            "search_thread",
            Some("codex"),
            json!({"cid": "huge-cx", "query": "cx_tool_tail_777"}),
        )
        .unwrap();
    assert_eq!(search["total"], 1);

    let early = svc
        .rpc(
            "search_thread",
            Some("codex"),
            json!({"cid": "huge-cx", "query": "cx_early_needle_0"}),
        )
        .unwrap();
    assert_eq!(early["total"], 1);

    let stats = svc
        .rpc("context_stats", Some("codex"), json!({"cid": "huge-cx"}))
        .unwrap();
    assert_eq!(stats["total_steps"], 413);
    assert!(stats["tool_bytes"].as_u64().unwrap() as usize >= big_output.len());
    assert!(stats["total_bytes"].as_u64().unwrap() as usize >= big_output.len() + big_reply.len());
}

#[test]
fn claude_page_byte_budget_and_backup_entries_streaming_work() {
    let dir = tempfile::tempdir().unwrap();
    let paths = AppPaths::isolated(dir.path());
    let claude_paths = paths
        .providers
        .iter()
        .find(|p| p.id == "claude")
        .unwrap()
        .clone();
    let proj = claude_paths.root.join("projects").join("budget-proj");
    fs::create_dir_all(&proj).unwrap();
    let session = proj.join("budget-sess.jsonl");
    let big_output = "Z".repeat(40_000);
    let mut raw = String::new();
    for i in 0..90 {
        let row = json!({
            "type": "user",
            "uuid": format!("u-{i}"),
            "parentUuid": Value::Null,
            "timestamp": "2026-05-01T12:00:00Z",
            "message": {
                "role": "user",
                "content": [{
                    "type": "tool_result",
                    "tool_use_id": format!("tu-{i}"),
                    "content": big_output
                }]
            }
        });
        raw.push_str(&serde_json::to_string(&row).unwrap());
        raw.push('\n');
    }
    fs::write(&session, &raw).unwrap();

    let bdir = claude_paths.backup_root.join("budget-sess/20260501_test");
    fs::create_dir_all(&bdir).unwrap();
    fs::write(bdir.join("budget-sess.jsonl"), &raw).unwrap();
    fs::write(
        bdir.join("backup.meta.json"),
        serde_json::to_vec(&json!({"tid": "budget-sess", "file_name": "budget-sess.jsonl"}))
            .unwrap(),
    )
    .unwrap();

    let svc = Service::new(paths, Arc::new(|_| {})).unwrap();
    let page1 = svc
        .rpc(
            "entries",
            Some("claude"),
            json!({"cid": "budget-sess", "include_tools": true, "limit": 150}),
        )
        .unwrap();
    let items1 = flatten_items(&page1);
    assert!(items1.len() >= 1 && items1.len() < 90);
    assert_eq!(page1["has_more"], true);
    assert!(items1.iter().all(|it| it["text_truncated"] == true));
    assert!(items1.iter().all(|it| it["full_bytes"] == 40_000));

    let page2 = svc
        .rpc(
            "entries",
            Some("claude"),
            json!({
                "cid": "budget-sess",
                "include_tools": true,
                "limit": 150,
                "before": page1["cursor"]
            }),
        )
        .unwrap();
    let items2 = flatten_items(&page2);
    assert!(!items2.is_empty());
    assert_eq!(items1.len() + items2.len(), 90);
    assert_eq!(page2["has_more"], false);

    let bpage = svc
        .rpc(
            "backup_entries",
            Some("claude"),
            json!({"backup_dir": bdir, "include_tools": true, "limit": 150}),
        )
        .unwrap();
    let bitems = flatten_items(&bpage);
    assert!(bitems.len() >= 1 && bitems.len() < 90);
    assert_eq!(bpage["has_more"], true);
    assert!(bitems.iter().all(|it| it["text_truncated"] == true));
}

#[test]
fn search_thread_returns_latest_250_and_around_key_jumps_in_bounded_window() {
    let dir = tempfile::tempdir().unwrap();
    let svc = Service::new(AppPaths::isolated(dir.path()), Arc::new(|_| {})).unwrap();
    let (doc, _, _) = build_large_doc();

    for pid in ["codex", "claude", "antigravity", "opencode"] {
        let prov = svc.provider(pid).unwrap();
        let imported = converter::import_doc_to_provider(&doc, prov.as_ref(), None, true).unwrap();
        let cid = imported["cid"].as_str().unwrap().to_owned();

        let no_tools_hit = svc
            .rpc(
                "search_thread",
                Some(pid),
                json!({"cid": cid, "query": "tool_tail_needle_999", "include_tools": false}),
            )
            .unwrap();
        assert_eq!(
            no_tools_hit["total"], 0,
            "{pid}: include_tools=false should exclude tool entry matches"
        );

        let many = svc
            .rpc(
                "search_thread",
                Some(pid),
                json!({"cid": cid, "query": "number", "include_tools": false}),
            )
            .unwrap();
        assert_eq!(many["total"], 410, "{pid}: expected 410 total matches");
        assert_eq!(many["shown"], 250, "{pid}: expected 250 shown matches");
        assert_eq!(many["truncated"], true);
        let matches = many["matches"].as_array().unwrap();
        assert_eq!(matches.len(), 250);
        assert!(matches[0]["snippet"].as_str().unwrap().contains("80"));
        assert!(matches[249]["snippet"].as_str().unwrap().contains("204"));

        let early = svc
            .rpc(
                "search_thread",
                Some(pid),
                json!({"cid": cid, "query": "early_needle_step_zero", "include_tools": false}),
            )
            .unwrap();
        let target_key = early["matches"][0]["key"].as_str().unwrap();

        let jumped = svc
            .rpc(
                "entries",
                Some(pid),
                json!({
                    "cid": cid,
                    "around_key": target_key,
                    "limit": 80,
                    "include_tools": false
                }),
            )
            .unwrap();
        let jumped_items = flatten_items(&jumped);
        assert!(
            jumped_items.len() <= 90,
            "{pid}: around_key window too large ({})",
            jumped_items.len()
        );
        assert!(
            jumped_items
                .iter()
                .any(|it| it["key"].as_str() == Some(target_key)),
            "{pid}: around_key response missing target key"
        );
        assert_eq!(
            jumped["has_newer"], true,
            "{pid}: jumping to early entry must report has_newer=true"
        );
    }
}

#[test]
fn search_thread_whole_word_and_context_stats_cache_invalidation() {
    let dir = tempfile::tempdir().unwrap();
    let (send, messages) = std::sync::mpsc::channel::<Value>();
    let svc = Service::new(
        AppPaths::isolated(dir.path()),
        Arc::new(move |v| {
            let _ = send.send(v);
        }),
    )
    .unwrap();
    let prov = svc.provider("claude").unwrap();
    let doc = json!({
        "meta": {
            "source_provider": "fixture",
            "source_id": "whole-word-stats",
            "title": "Whole Word And Cache",
            "directory": ""
        },
        "entries": [
            {
                "kind": "user",
                "subtype": "prompt",
                "text": "Нужно читать документацию, а не чит.",
                "ts": "2026-05-01T10:00:00Z"
            },
            {
                "kind": "assistant",
                "subtype": "reply",
                "text": "Сборка проекта завершена (чит) и читать логи.",
                "ts": "2026-05-01T10:00:01Z"
            },
            {
                "kind": "user",
                "subtype": "prompt",
                "text": "Только слово читать без отдельного токена.",
                "ts": "2026-05-01T10:00:02Z"
            }
        ]
    });
    let imported = converter::import_doc_to_provider(&doc, prov.as_ref(), None, true).unwrap();
    let cid = imported["cid"].as_str().unwrap().to_owned();

    let whole_on = svc
        .rpc(
            "search_thread",
            Some("claude"),
            json!({"cid": cid, "query": "чит", "whole_word": true}),
        )
        .unwrap();
    assert_eq!(whole_on["total"], 2);
    let matches_on = whole_on["matches"].as_array().unwrap();
    assert!(matches_on[0]["snippet"].as_str().unwrap().contains("чит"));
    assert!(matches_on[1]["snippet"].as_str().unwrap().contains("(чит)"));

    let whole_off = svc
        .rpc(
            "search_thread",
            Some("claude"),
            json!({"cid": cid, "query": "чит", "whole_word": false}),
        )
        .unwrap();
    assert_eq!(whole_off["total"], 3);

    let prefix_off = svc
        .rpc(
            "search_thread",
            Some("claude"),
            json!({"cid": cid, "query": "Сбор", "whole_word": false}),
        )
        .unwrap();
    assert_eq!(prefix_off["total"], 1);
    assert!(prefix_off["matches"][0]["snippet"]
        .as_str()
        .unwrap()
        .contains("Сборка"));

    let prefix_on = svc
        .rpc(
            "search_thread",
            Some("claude"),
            json!({"cid": cid, "query": "Сбор", "whole_word": true}),
        )
        .unwrap();
    assert_eq!(prefix_on["total"], 0);

    let stats1 = svc
        .rpc("context_stats", Some("claude"), json!({"cid": cid}))
        .unwrap();
    let stats2 = svc
        .rpc("context_stats", Some("claude"), json!({"cid": cid}))
        .unwrap();
    assert_eq!(stats1, stats2);
    let initial_bytes = stats1["total_bytes"].as_u64().unwrap();

    let entries = svc
        .rpc(
            "entries",
            Some("claude"),
            json!({"cid": cid, "include_tools": true}),
        )
        .unwrap();
    let items = flatten_items(&entries);
    let editable = items
        .iter()
        .find(|v| {
            v["kind"] == "assistant"
                && v["actions"]
                    .as_array()
                    .is_some_and(|a| a.contains(&json!("edit")))
        })
        .expect("editable assistant entry");
    let key = editable["key"].as_str().unwrap();
    let preview = svc
        .rpc(
            "edit_preview",
            Some("claude"),
            json!({"cid": cid, "key": key}),
        )
        .unwrap();
    let new_text = "Сборка полностью обновлена и расширена новым длинным текстом для проверки инвалидации кэша статистики.";
    let reply = svc
        .rpc(
            "apply_edit",
            Some("claude"),
            json!({
                "cid": cid,
                "key": key,
                "revision": preview["revision"],
                "text": new_text,
                "confirm": true
            }),
        )
        .unwrap();
    let job_id = reply["job_id"].as_str().unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        let rem = deadline.saturating_duration_since(std::time::Instant::now());
        let ev = messages.recv_timeout(rem).unwrap();
        if ev["type"] == "job"
            && ev["job"]["id"] == job_id
            && matches!(ev["job"]["state"].as_str(), Some("done" | "error"))
        {
            assert_eq!(ev["job"]["state"], "done", "{}", ev["job"]["error"]);
            break;
        }
    }
    while svc.active_jobs() > 0 && std::time::Instant::now() < deadline {
        std::thread::yield_now();
    }

    let stats3 = svc
        .rpc("context_stats", Some("claude"), json!({"cid": cid}))
        .unwrap();
    assert!(stats3["total_bytes"].as_u64().unwrap() > initial_bytes);
}
