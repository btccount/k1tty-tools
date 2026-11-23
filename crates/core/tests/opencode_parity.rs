use k1tty_core::{
    opencode::OpenCodeProvider,
    provider::{OpContext, Provider, ProviderPaths},
};
use rusqlite::Connection;
use serde_json::{json, Value};
use std::fs;

fn setup() -> (tempfile::TempDir, OpenCodeProvider) {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("data");
    fs::create_dir_all(&root).unwrap();
    Connection::open(root.join("opencode.db"))
        .unwrap()
        .execute_batch(include_str!("fixtures/opencode/fixture.sql"))
        .unwrap();
    let p = OpenCodeProvider::new(ProviderPaths {
        id: "opencode".into(),
        name: "OpenCode".into(),
        short: "OC".into(),
        root,
        backup_root: tmp.path().join("backups"),
    });
    (tmp, p)
}

fn all_items(p: &OpenCodeProvider) -> Value {
    let result = p
        .call(
            "entries",
            &json!({"cid":"fixture-session","include_tools":true,"limit":150}),
            &OpContext::default(),
        )
        .unwrap();
    if result["turns"].as_array().unwrap().is_empty() {
        json!([])
    } else {
        result["turns"][0]["items"].clone()
    }
}

#[test]
fn matches_83_upstream_python_opencode_cases() {
    let cases: Value = serde_json::from_str(include_str!("fixtures/opencode/oracle.json")).unwrap();
    for (index, case) in cases.as_array().unwrap().iter().enumerate() {
        let (_tmp, p) = setup();
        let op = case["op"].as_str().unwrap();
        let ctx = OpContext {
            auto_backup: false,
            ..Default::default()
        };
        let result = p
            .call(op, &case["data"], &ctx)
            .unwrap_or_else(|e| panic!("case {index} {op}: {e:#}"));
        if let Some(expected) = case.get("items_after") {
            assert_eq!(
                all_items(&p),
                *expected,
                "case {index}: {op} {}",
                case["data"]
            );
        } else {
            assert_eq!(
                result, case["result"],
                "case {index}: {op} {}",
                case["data"]
            );
        }
    }
}

#[test]
fn restores_upstream_python_snapshot_without_lowering_sequence() {
    let (_tmp, p) = setup();
    let expected = all_items(&p);
    let dir = p.paths().backup_root.join("fixture-session/old-python");
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("session_backup.json"),
        include_bytes!("fixtures/opencode/session_backup.json"),
    )
    .unwrap();
    fs::write(
        dir.join("backup.meta.json"),
        include_bytes!("fixtures/opencode/backup.meta.json"),
    )
    .unwrap();
    let c = Connection::open(p.paths().root.join("opencode.db")).unwrap();
    c.execute_batch("DELETE FROM session_message;UPDATE event_sequence SET seq=200;")
        .unwrap();
    p.call("restore", &json!({"backup_dir":dir}), &OpContext::default())
        .unwrap();
    assert_eq!(all_items(&p), expected);
    let sequence: i64 = c
        .query_row(
            "SELECT seq FROM event_sequence WHERE aggregate_id='fixture-session'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(sequence, 200);
}
