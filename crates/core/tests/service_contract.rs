use k1tty_core::{
    converter, pack,
    service::{AppPaths, Service, PROVIDER_ORDER},
};
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    fs,
    sync::{mpsc, Arc},
    time::{Duration, Instant},
};

struct Harness {
    root: tempfile::TempDir,
    service: Arc<Service>,
    messages: mpsc::Receiver<Value>,
    received: Vec<Value>,
}
impl Harness {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let (send, messages) = mpsc::channel();
        let service = Service::new(
            AppPaths::isolated(root.path()),
            Arc::new(move |v| {
                let _ = send.send(v);
            }),
        )
        .unwrap();
        Self {
            root,
            service,
            messages,
            received: Vec::new(),
        }
    }
    fn seed(&self, provider: &str) -> String {
        let p = self.service.provider(provider).unwrap();
        converter::import_doc_to_provider(&sample(), p.as_ref(), None, true).unwrap()["cid"]
            .as_str()
            .unwrap()
            .to_owned()
    }
    fn rpc(&self, op: &str, provider: &str, data: Value) -> Value {
        self.service
            .rpc(op, Some(provider), data)
            .unwrap_or_else(|e| panic!("{provider}/{op}: {e:#}"))
    }
    fn job(&mut self, op: &str, provider: &str, data: Value) -> Value {
        let reply = self.rpc(op, provider, data);
        let job = self.terminal_job(op, provider, &reply);
        assert_eq!(job["state"], "done", "{provider}/{op}: {}", job["error"]);
        assert_eq!(job["phase"], "done");
        job
    }
    fn terminal_job(&mut self, op: &str, provider: &str, reply: &Value) -> Value {
        let id = reply["job_id"]
            .as_str()
            .expect("mutation replies with job_id");
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            let message = self
                .messages
                .recv_timeout(remaining)
                .unwrap_or_else(|e| panic!("No terminal event for {provider}/{op} job {id}: {e}"));
            self.received.push(message.clone());
            if message["type"] == "job"
                && message["job"]["id"] == id
                && matches!(message["job"]["state"].as_str(), Some("done" | "error"))
            {
                let job = &message["job"];
                assert!(job["ms"].is_number());
                while self.service.active_jobs() > 0 && Instant::now() < deadline {
                    std::thread::yield_now();
                }
                assert_eq!(
                    self.service.active_jobs(),
                    0,
                    "completed jobs must release exit guard"
                );
                return job.clone();
            }
        }
    }
    fn edit_assistant(&mut self, provider: &str, cid: &str, text: &str) -> Value {
        let entries = self.rpc("entries", provider, json!({"cid":cid,"include_tools":true}));
        let item = items(&entries)
            .into_iter()
            .find(|v| {
                v["kind"] == "assistant"
                    && v["actions"]
                        .as_array()
                        .is_some_and(|a| a.contains(&json!("edit")))
            })
            .expect("editable assistant message");
        let key = item["key"].clone();
        let preview = self.rpc("edit_preview", provider, json!({"cid":cid,"key":key}));
        self.job(
            "apply_edit",
            provider,
            json!({"cid":cid,"key":key,"revision":preview["revision"],"text":text,"confirm":true}),
        )
    }
}
fn items(reply: &Value) -> Vec<Value> {
    reply["turns"]
        .as_array()
        .into_iter()
        .flatten()
        .flat_map(|t| t["items"].as_array().into_iter().flatten())
        .cloned()
        .collect()
}
fn sample() -> Value {
    json!({"meta":{"source_provider":"fixture","source_id":"fixture-session","title":"Сессия 🐈","directory":""},"entries":[
        {"kind":"user","text":"Сохрани весь контекст","ts":"2026-11-19T10:00:00Z"},
        {"kind":"tool","subtype":"thinking","text":"Сначала проверю файлы","ts":"2026-11-19T10:00:01Z"},
        {"kind":"assistant","text":"Первый ответ","ts":"2026-11-19T10:00:02Z"},
        {"kind":"tool","subtype":"tool_output","tool":"read_file","label":"read_file","text":"Результат инструмента","status":"ok","ts":"2026-11-19T10:00:03Z"},
        {"kind":"system","text":"Инструкция проекта","ts":"2026-11-19T10:00:04Z"},
        {"kind":"user","text":"Продолжай задачу","ts":"2026-11-19T10:00:05Z"},
        {"kind":"assistant","text":"Второй ответ","ts":"2026-11-19T10:00:06Z"}
    ]})
}

#[test]
fn rpc_state_lists_and_pagination_agree_after_native_imports() {
    let h = Harness::new();
    assert_eq!(h.service.hello()["type"], "hello");
    assert_eq!(h.rpc("ping", "all", json!({}))["pong"], true);
    let empty = h.rpc("state", "all", json!({"force":true}));
    assert_eq!(empty["providers"].as_object().unwrap().len(), 5);
    assert_eq!(empty["settings"]["auto_backup"], true);
    let ids: BTreeMap<_, _> = PROVIDER_ORDER
        .into_iter()
        .map(|id| (id, h.seed(id)))
        .collect();
    let state = h.rpc("state", "all", json!({"force":true}));
    let all = h.rpc("convs", "all", json!({"refresh":true}));
    assert_eq!(all["convs"].as_array().unwrap().len(), 5);
    assert_eq!(all["errors"], json!({}));
    for (provider, cid) in ids {
        assert_eq!(
            state["providers"][provider]["available"], true,
            "{provider}"
        );
        assert_eq!(all["counts"][provider], 1);
        let list = h.rpc("convs", provider, json!({}));
        assert_eq!(list["convs"][0]["id"], cid);
        let page = h.rpc(
            "entries",
            provider,
            json!({"cid":cid,"limit":1,"include_tools":false}),
        );
        assert!(
            items(&page).iter().any(|e| e["text"]
                .as_str()
                .is_some_and(|s| s.contains("Второй ответ"))),
            "{provider}: {page}"
        );
        assert_eq!(page["has_more"], true, "{provider}: {page}");
        let earlier = h.rpc(
            "entries",
            provider,
            json!({"cid":cid,"limit":150,"include_tools":true,"before":page["cursor"]}),
        );
        assert!(
            items(&earlier).iter().any(|e| e["text"]
                .as_str()
                .is_some_and(|s| s.contains("Сохрани весь контекст"))),
            "{provider}: {earlier}"
        );
        assert_eq!(earlier["has_more"], false);
    }
}

#[test]
fn confirmed_mutations_emit_jobs_and_backups_then_restore_original_messages() {
    let mut h = Harness::new();
    let cid = h.seed("codex");
    assert!(h
        .service
        .rpc(
            "apply_edit",
            Some("codex"),
            json!({"cid":cid,"key":"0","text":"not confirmed"})
        )
        .is_err());
    assert_eq!(h.service.active_jobs(), 0);
    let edited = h.edit_assistant("codex", &cid, "Отредактированный ответ");
    assert_eq!(edited["provider"], "codex");
    assert_eq!(edited["name"], "edit");
    assert!(edited["result"]["backup"].is_string());
    let events: Vec<_> = h
        .received
        .iter()
        .filter(|v| v["job"]["id"] == edited["id"])
        .collect();
    assert!(events.iter().any(|e| e["job"]["state"] == "running"));
    assert!(events.iter().any(|e| e["job"]["phase"] == "backup"));
    let changed = h.rpc("entries", "codex", json!({"cid":cid,"include_tools":true}));
    assert!(items(&changed)
        .iter()
        .any(|e| e["text"] == "Отредактированный ответ"));
    let bdir = edited["result"]["backup"].clone();
    let inspected = h.rpc(
        "backup_entries",
        "codex",
        json!({"backup_dir":bdir,"include_tools":true}),
    );
    assert_eq!(inspected["readonly"], true);
    assert!(items(&inspected)
        .iter()
        .any(|e| e["text"] == "Первый ответ"));
    let restored = h.job(
        "restore",
        "codex",
        json!({"backup_dir":bdir,"confirm":true}),
    );
    assert_eq!(restored["name"], "restore");
    let original = h.rpc("entries", "codex", json!({"cid":cid,"include_tools":true}));
    assert!(items(&original).iter().any(|e| e["text"] == "Первый ответ"));
}

#[test]
fn backups_aggregate_across_providers_and_validate_paths_before_deleting() {
    let mut h = Harness::new();
    for p in ["antigravity", "claude", "opencode"] {
        let cid = h.seed(p);
        h.edit_assistant(p, &cid, "Изменено");
    }
    let backups = h.rpc("backups", "all", json!({}));
    let summary = h.rpc("backups_summary", "all", json!({}));
    assert_eq!(backups["total_count"], 3);
    assert_eq!(summary["total_count"], 3);
    assert_eq!(backups["total_bytes"], summary["total_bytes"]);
    assert_eq!(backups["groups"].as_array().unwrap().len(), 3);
    assert_eq!(backups["summary"].as_array().unwrap().len(), 5);
    let provider = h.service.provider("antigravity").unwrap();
    let root = provider.paths().backup_root.clone();
    for bad in [
        root.clone(),
        root.parent().unwrap().to_owned(),
        h.root.path().to_owned(),
    ] {
        assert!(h
            .service
            .rpc(
                "delete_backup",
                Some("antigravity"),
                json!({"backup_dir":bad,"confirm":true})
            )
            .is_err());
        assert!(h
            .service
            .rpc(
                "restore",
                Some("antigravity"),
                json!({"backup_dir":bad,"confirm":true})
            )
            .is_err());
    }
    let own = h.rpc("backups", "antigravity", json!({}));
    let backup = own["groups"][0]["backups"][0]["dir"].clone();
    assert!(h
        .service
        .rpc(
            "delete_backup",
            Some("antigravity"),
            json!({"backup_dir":backup})
        )
        .is_err());
    let delete = h.job(
        "delete_backup",
        "antigravity",
        json!({"backup_dir":backup,"confirm":true}),
    );
    assert_eq!(delete["result"]["deleted_count"], 1);
    let all = h.job("delete_all_backups", "all", json!({"confirm":true}));
    assert_eq!(all["result"]["total_deleted"], 2);
    assert_eq!(all["result"]["breakdown"].as_array().unwrap().len(), 2);
    assert_eq!(h.rpc("backups_summary", "all", json!({}))["total_count"], 0);
}

#[test]
fn all_25_conversion_directions_are_visible_and_same_provider_never_deadlocks() {
    let mut h = Harness::new();
    let ids: BTreeMap<_, _> = PROVIDER_ORDER.into_iter().map(|p| (p, h.seed(p))).collect();
    for source in PROVIDER_ORDER {
        for destination in PROVIDER_ORDER {
            let job=h.job("convert_chat",source,json!({"cid":ids[source],"dst_provider":destination,"title":format!("{source} to {destination}"),"include_tools":true}));
            assert_eq!(job["name"], "convert_chat");
            assert_eq!(
                job["provider"], destination,
                "conversion events retain original destination-provider contract"
            );
            let result = &job["result"];
            assert_eq!(result["provider"], destination);
            assert_eq!(result["source_provider"], source);
            assert_eq!(result["source_id"], ids[source]);
            let cid = result["cid"].as_str().unwrap();
            assert_ne!(cid, ids[source]);
            let entries = h.rpc(
                "entries",
                destination,
                json!({"cid":cid,"include_tools":true,"limit":150}),
            );
            let texts = items(&entries)
                .iter()
                .filter_map(|v| v["text"].as_str())
                .collect::<Vec<_>>()
                .join("\n");
            for expected in [
                "Сохрани весь контекст",
                "Первый ответ",
                "Продолжай задачу",
                "Второй ответ",
            ] {
                assert!(
                    texts.contains(expected),
                    "{source} -> {destination}: missing {expected}; {entries}"
                );
            }
            let listed = h.rpc("convs", destination, json!({"refresh":true}));
            assert!(listed["convs"]
                .as_array()
                .unwrap()
                .iter()
                .any(|e| e["id"] == cid));
        }
    }
    assert_eq!(h.service.active_jobs(), 0);
}

#[test]
fn settings_persist_and_control_backup_creation() {
    let mut h = Harness::new();
    let cid = h.seed("antigravity");
    let settings = h.rpc("set_settings", "all", json!({"auto_backup":false}));
    assert_eq!(settings["settings"]["auto_backup"], false);
    assert_eq!(
        h.messages.recv_timeout(Duration::from_secs(1)).unwrap(),
        json!({"type":"settings","settings":{"auto_backup":false}})
    );
    let edited = h.edit_assistant("antigravity", &cid, "Без копии");
    assert!(edited["result"]["backup"].is_null());
    assert_eq!(h.rpc("backups_summary", "all", json!({}))["total_count"], 0);
    let reopened = Service::new(AppPaths::isolated(h.root.path()), Arc::new(|_| {})).unwrap();
    assert_eq!(
        reopened.rpc("get_settings", None, json!({})).unwrap()["settings"]["auto_backup"],
        false
    );
    h.rpc("set_settings", "all", json!({"auto_backup":true}));
    let edited = h.edit_assistant("antigravity", &cid, "С копией");
    assert!(edited["result"]["backup"].is_string());
}

#[test]
fn native_pack_inspection_import_and_save_have_frontend_response_shapes() {
    let h = Harness::new();
    let cid = h.seed("claude");
    let exported = h
        .service
        .export_pack(&json!({"provider":"claude","cid":cid,"preset":"fast","include_tools":true}))
        .unwrap();
    assert_eq!(exported["ok"], true);
    assert!(exported["stats"]["entries"].as_u64().unwrap() > 0);
    let token = exported["token"].as_str().unwrap();
    let filename = exported["filename"].as_str().unwrap();
    assert!(filename.ends_with(".k1tty"));
    assert_eq!(h.service.export_filename(token).unwrap(), filename);
    let path = h.root.path().join("saved/история.k1tty");
    h.service.save_export(token, &path).unwrap();
    let bytes = fs::read(&path).unwrap();
    let inspected = h.service.inspect_pack(bytes.clone()).unwrap();
    assert_eq!(inspected["ok"], true);
    assert_eq!(inspected["info"]["source_provider"], "claude");
    assert_eq!(inspected["info"]["packed_bytes"], bytes.len());
    let import_token = inspected["token"].as_str().unwrap();
    for target in PROVIDER_ORDER {
        let imported = h
            .service
            .import_pack(import_token, target, Some("Архив перенесён"), true)
            .unwrap();
        assert_eq!(imported["ok"], true);
        assert_eq!(imported["result"]["provider"], target);
        assert_eq!(imported["result"]["source_provider"], "claude");
        let view = h.rpc(
            "entries",
            target,
            json!({"cid":imported["result"]["cid"],"include_tools":true}),
        );
        assert!(
            !items(&view).is_empty(),
            "{target}: imported archive must be visible"
        );
    }
    assert!(h.service.inspect_pack(Vec::new()).is_err());
    assert!(h.service.inspect_pack(b"not an archive".to_vec()).is_err());
    assert!(h
        .service
        .import_pack("expired", "claude", None, true)
        .is_err());
    assert!(h.service.save_export("expired", &path).is_err());
    assert!(h.service.export_filename("expired").is_err());
    let mut corrupt = bytes;
    let last = corrupt.len() - 1;
    corrupt[last] ^= 0x40;
    assert!(pack::unpack_chat(&corrupt).is_err());
}

#[cfg(unix)]
#[test]
fn backup_symlinks_cannot_delete_or_restore_outside_provider_root() {
    use std::os::unix::fs::symlink;
    let h = Harness::new();
    let provider = h.service.provider("antigravity").unwrap();
    let backup_root = &provider.paths().backup_root;
    let external = h.root.path().join("external/keep");
    fs::create_dir_all(&external).unwrap();
    fs::write(external.join("important.txt"), b"untouched").unwrap();
    fs::create_dir_all(backup_root.join("session")).unwrap();
    let link = backup_root.join("session/escape");
    symlink(&external, &link).unwrap();
    assert!(h
        .service
        .rpc(
            "delete_backup",
            Some("antigravity"),
            json!({"backup_dir":link,"confirm":true})
        )
        .is_err());
    assert!(h
        .service
        .rpc(
            "restore",
            Some("antigravity"),
            json!({"backup_dir":link,"confirm":true})
        )
        .is_err());
    assert_eq!(
        fs::read(external.join("important.txt")).unwrap(),
        b"untouched"
    );
    assert!(h
        .service
        .rpc(
            "entries",
            Some("antigravity"),
            json!({"cid":"../../external"})
        )
        .is_err());
    assert!(h
        .service
        .rpc(
            "entries",
            Some("antigravity"),
            json!({"cid":"..\\external"})
        )
        .is_err());
    assert!(h
        .service
        .rpc(
            "apply_cut",
            Some("antigravity"),
            json!({"cid":"safe","key":"0","mode":"sideways","confirm":true})
        )
        .is_err());
    assert!(external.is_dir());
}

fn claude_session_path(h: &Harness, cid: &str) -> std::path::PathBuf {
    let root = h.service.provider("claude").unwrap().paths().root.clone();
    walkdir::WalkDir::new(root.join("projects"))
        .into_iter()
        .filter_map(Result::ok)
        .find(|entry| {
            entry.file_type().is_file()
                && entry.path().file_stem().and_then(|s| s.to_str()) == Some(cid)
        })
        .unwrap()
        .into_path()
}

#[test]
fn claude_notice_jobs_add_remove_and_restore_without_changing_regular_messages() {
    let mut h = Harness::new();
    let cid = h.seed("claude");
    let path = claude_session_path(&h, &cid);
    let original_entries = items(&h.rpc(
        "entries",
        "claude",
        json!({"cid":cid,"include_tools":true,"limit":150}),
    ));
    let original_texts: Vec<_> = original_entries
        .iter()
        .filter(|entry| entry["kind"] != "system")
        .map(|entry| (entry["kind"].clone(), entry["text"].clone()))
        .collect();
    let before = h.rpc("system_notices_info", "claude", json!({"cid":cid}));
    assert_eq!(before["writable"], true);
    assert!(before["revision"].is_string());
    assert!(before["types"]
        .as_array()
        .unwrap()
        .iter()
        .any(|kind| kind["id"] == "notice"));
    let original_count = before["notice_count"].as_u64().unwrap();
    for (notice_type, text) in [
        ("notice", "Информация для этой сессии 🐈"),
        ("warning", "Предупреждение в локальной истории"),
    ] {
        let info = h.rpc("system_notices_info", "claude", json!({"cid":cid}));
        let job = h.job(
            "add_system_notice",
            "claude",
            json!({"cid":cid,"revision":info["revision"],"notice_type":notice_type,"text":text,"confirm":true}),
        );
        assert_eq!(job["provider"], "claude");
        assert_eq!(job["name"], "add_system_notice");
        assert_eq!(job["result"]["cid"], cid);
        assert!(job["result"]["backup"].is_string());
        assert!(h
            .received
            .iter()
            .any(|event| { event["job"]["id"] == job["id"] && event["job"]["phase"] == "backup" }));
        let entries = h.rpc(
            "entries",
            "claude",
            json!({"cid":cid,"include_tools":true,"limit":150}),
        );
        assert!(items(&entries)
            .iter()
            .any(|entry| entry["kind"] == "system" && entry["text"] == text));
    }
    let info = h.rpc("system_notices_info", "claude", json!({"cid":cid}));
    let safety = h.job(
        "add_system_notice",
        "claude",
        json!({"cid":cid,"revision":info["revision"],"notice_type":"safety_classifier","text":"","confirm":true}),
    );
    assert!(safety["result"]["backup"].is_string());
    let before_delete = fs::read(&path).unwrap();
    let info = h.rpc("system_notices_info", "claude", json!({"cid":cid}));
    assert_eq!(info["notice_count"], original_count + 3);
    let deleted = h.job(
        "delete_system_notices",
        "claude",
        json!({"cid":cid,"revision":info["revision"],"confirm":true}),
    );
    assert_eq!(deleted["name"], "delete_system_notices");
    assert_eq!(deleted["result"]["cid"], cid);
    assert_eq!(deleted["result"]["deleted_count"], original_count + 3);
    assert_eq!(
        h.rpc("system_notices_info", "claude", json!({"cid":cid}))["notice_count"],
        0
    );
    let remaining = items(&h.rpc(
        "entries",
        "claude",
        json!({"cid":cid,"include_tools":true,"limit":150}),
    ));
    let remaining_texts: Vec<_> = remaining
        .iter()
        .filter(|entry| entry["kind"] != "system")
        .map(|entry| (entry["kind"].clone(), entry["text"].clone()))
        .collect();
    assert_eq!(remaining_texts, original_texts);
    h.job(
        "restore",
        "claude",
        json!({"backup_dir":deleted["result"]["backup"],"confirm":true}),
    );
    assert_eq!(fs::read(&path).unwrap(), before_delete);
    assert_eq!(
        h.rpc("system_notices_info", "claude", json!({"cid":cid}))["notice_count"],
        original_count + 3
    );
}

#[test]
fn claude_notice_rpc_requires_confirmation_revision_and_supported_provider_before_jobs() {
    let h = Harness::new();
    let cid = h.seed("claude");
    let path = claude_session_path(&h, &cid);
    let bytes = fs::read(&path).unwrap();
    let info = h.rpc("system_notices_info", "claude", json!({"cid":cid}));
    let payload = json!({"cid":cid,"revision":info["revision"],"notice_type":"notice","text":"A local notice","confirm":true});
    for op in ["add_system_notice", "delete_system_notices"] {
        for missing in ["confirm", "revision"] {
            let mut data = payload.clone();
            data.as_object_mut().unwrap().remove(missing);
            assert!(h.service.rpc(op, Some("claude"), data).is_err());
        }
        let error = h
            .service
            .rpc(op, Some("codex"), payload.clone())
            .unwrap_err();
        assert!(format!("{error:#}").contains("только для диалогов Claude"));
        let mut traversal = payload.clone();
        traversal["cid"] = json!("../other-session");
        assert!(h.service.rpc(op, Some("claude"), traversal).is_err());
    }
    let mut empty = payload;
    empty["text"] = json!(" \n\t");
    assert!(h
        .service
        .rpc("add_system_notice", Some("claude"), empty)
        .is_err());
    assert_eq!(h.service.active_jobs(), 0);
    assert!(h.messages.try_recv().is_err());
    assert_eq!(fs::read(path).unwrap(), bytes);
    assert_eq!(
        h.rpc("backups_summary", "claude", json!({}))["total_count"],
        0
    );
}

#[test]
fn claude_notice_jobs_reject_stale_revisions_and_honor_backup_settings() {
    let mut h = Harness::new();
    let cid = h.seed("claude");
    let path = claude_session_path(&h, &cid);
    let stale = h.rpc("system_notices_info", "claude", json!({"cid":cid}));
    let mut bytes = fs::read(&path).unwrap();
    bytes.push(b'\n');
    fs::write(&path, &bytes).unwrap();
    for op in ["add_system_notice", "delete_system_notices"] {
        let reply = h.rpc(
            op,
            "claude",
            json!({"cid":cid,"revision":stale["revision"],"notice_type":"notice","text":"Stale notice","confirm":true}),
        );
        let job = h.terminal_job(op, "claude", &reply);
        assert_eq!(job["state"], "error");
        assert!(job["error"].as_str().unwrap().contains("изменилась"));
        assert_eq!(fs::read(&path).unwrap(), bytes);
    }
    assert_eq!(
        h.rpc("backups_summary", "claude", json!({}))["total_count"],
        0
    );
    h.rpc("set_settings", "all", json!({"auto_backup":false}));
    let info = h.rpc("system_notices_info", "claude", json!({"cid":cid}));
    let added = h.job(
        "add_system_notice",
        "claude",
        json!({"cid":cid,"revision":info["revision"],"notice_type":"notice","text":"No backup requested","confirm":true}),
    );
    assert!(added["result"]["backup"].is_null());
    assert!(!h
        .received
        .iter()
        .any(|event| { event["job"]["id"] == added["id"] && event["job"]["phase"] == "backup" }));
    let info = h.rpc("system_notices_info", "claude", json!({"cid":cid}));
    let deleted = h.job(
        "delete_system_notices",
        "claude",
        json!({"cid":cid,"revision":info["revision"],"confirm":true}),
    );
    assert!(deleted["result"]["backup"].is_null());
    h.rpc("set_settings", "all", json!({"auto_backup":true}));
    let info = h.rpc("system_notices_info", "claude", json!({"cid":cid}));
    let before_noop = fs::read(&path).unwrap();
    let noop = h.job(
        "delete_system_notices",
        "claude",
        json!({"cid":cid,"revision":info["revision"],"confirm":true}),
    );
    assert_eq!(noop["result"]["no_op"], true);
    assert_eq!(noop["result"]["deleted_count"], 0);
    assert!(noop["result"]["backup"].is_null());
    assert_eq!(fs::read(path).unwrap(), before_noop);
    assert_eq!(
        h.rpc("backups_summary", "claude", json!({}))["total_count"],
        0
    );
}

#[test]
fn antigravity_live_unlock_rpc_flow_works_and_rejects_other_providers() {
    let mut h = Harness::new();
    let cid = h.seed("antigravity");

    // Non-Antigravity providers reject unlock_db / relock_db / restart_hub.
    for op in ["unlock_db", "relock_db", "restart_hub"] {
        assert!(h
            .service
            .rpc(op, Some("codex"), json!({"cid": cid}))
            .is_err());
    }

    #[cfg(unix)]
    let _holder = {
        use fs2::FileExt;
        let p = h.service.provider("antigravity").unwrap();
        let pdir = p.paths().root.join("presence");
        fs::create_dir_all(&pdir).unwrap();
        let holder = fs::File::create(pdir.join(format!("{cid}.lock"))).unwrap();
        holder.lock_exclusive().unwrap();
        holder
    };

    #[cfg(unix)]
    {
        let locked_entries = h.rpc("entries", "antigravity", json!({"cid": cid}));
        assert_eq!(locked_entries["file_locked"], true);
        assert_eq!(locked_entries["presence_locked"], true);
        assert_eq!(locked_entries["live_unlocked"], false);
        assert_eq!(locked_entries["can_unlock_db"], true);

        // Write is blocked before unlock_db.
        assert!(h
            .service
            .rpc(
                "apply_edit",
                Some("antigravity"),
                json!({"cid": cid, "key": "1", "text": "blocked", "confirm": true})
            )
            .is_err());
    }

    // Unlock DB via RPC and verify conv_lock event + live edit + restore + relock.
    let unlocked = h.rpc("unlock_db", "antigravity", json!({"cid": cid}));
    assert_eq!(unlocked["ok"], true);
    assert_eq!(unlocked["locked"], false);
    assert_eq!(unlocked["live_unlocked"], true);

    let ev = h.messages.recv_timeout(Duration::from_secs(2)).unwrap();
    assert_eq!(ev["type"], "conv_lock");
    assert_eq!(ev["provider"], "antigravity");
    assert_eq!(ev["cid"], cid);
    assert_eq!(ev["locked"], false);
    assert_eq!(ev["live_unlocked"], true);

    let edit_job = h.edit_assistant("antigravity", &cid, "Edited via live unlock RPC");
    let backup_dir = edit_job["result"]["backup"].as_str().unwrap().to_owned();
    let entries_after = h.rpc(
        "entries",
        "antigravity",
        json!({"cid": cid, "include_tools": true}),
    );
    assert!(items(&entries_after)
        .iter()
        .any(|it| it["text"] == "Edited via live unlock RPC"));

    h.job(
        "restore",
        "antigravity",
        json!({"backup_dir": backup_dir, "confirm": true}),
    );

    let relocked = h.rpc("relock_db", "antigravity", json!({"cid": cid}));
    assert_eq!(relocked["live_unlocked"], false);
    #[cfg(unix)]
    assert_eq!(relocked["locked"], true);
}
