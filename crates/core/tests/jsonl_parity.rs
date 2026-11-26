use anyhow::{Context, Result};
use k1tty_core::{
    claude::ClaudeProvider,
    codex::CodexProvider,
    jsonl,
    provider::{OpContext, Provider, ProviderPaths},
};
use serde_json::{json, Value};
use std::{
    fs,
    path::{Path, PathBuf},
};

const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/jsonl");
struct Fixture {
    _dir: tempfile::TempDir,
    provider: Box<dyn Provider>,
    file: PathBuf,
    backup_root: PathBuf,
}
fn fixture(kind: &str) -> Result<Fixture> {
    let dir = tempfile::tempdir()?;
    let root = dir.path().join(if kind == "codex" {
        "codex_sandbox"
    } else {
        "claude_test"
    });
    let relative = if kind == "codex" {
        "sessions/2026/01/01/rollout-oracle.jsonl"
    } else {
        "projects/-tmp-Проект/cl-oracle.jsonl"
    };
    let file = root.join(relative);
    fs::create_dir_all(file.parent().unwrap())?;
    fs::copy(Path::new(FIXTURES).join(format!("{kind}.jsonl")), &file)?;
    let backup_root = dir.path().join("backups");
    if kind == "codex" {
        fs::write(
            root.join("session_index.jsonl"),
            "{\"id\":\"cx-oracle\",\"thread_name\":\"Oracle title\"}\n",
        )?;
    }
    let paths = ProviderPaths {
        id: kind.into(),
        name: kind.into(),
        short: kind.into(),
        root,
        backup_root: backup_root.clone(),
    };
    let provider: Box<dyn Provider> = if kind == "codex" {
        Box::new(CodexProvider::new(paths))
    } else {
        Box::new(ClaudeProvider::new(paths))
    };
    Ok(Fixture {
        _dir: dir,
        provider,
        file,
        backup_root,
    })
}
fn expected_subset(actual: &Value, expected: &Value) -> bool {
    match expected {
        Value::Object(obj) => obj
            .iter()
            .all(|(k, v)| actual.get(k).is_some_and(|a| expected_subset(a, v))),
        _ => actual == expected,
    }
}
#[test]
fn matches_71_upstream_python_cases() -> Result<()> {
    let cases: Value =
        serde_json::from_slice(&fs::read(Path::new(FIXTURES).join("expected.json"))?)?;
    let mut failures = Vec::new();
    for case in cases.as_array().context("Expected cases array")? {
        let name = case["name"].as_str().unwrap();
        let f = fixture(case["provider"].as_str().unwrap())?;
        let op = case["op"].as_str().unwrap();
        let mut data = case["data"].clone();
        let ctx = OpContext {
            auto_backup: false,
            ..Default::default()
        };
        if matches!(op, "apply_edit" | "apply_cut") {
            let preview = f.provider.call(
                if op == "apply_edit" {
                    "edit_preview"
                } else {
                    "preview_cut"
                },
                &data,
                &ctx,
            )?;
            data["revision"] = preview["revision"].clone();
        }
        match f.provider.call(op, &data, &ctx) {
            Err(e) => failures.push(format!("{name}: {e:#}")),
            Ok(mut actual) => {
                if case["provider"] == "claude" && op == "entries" {
                    if let Some(turns) = actual["turns"].as_array_mut() {
                        for turn in turns {
                            if let Some(items) = turn["items"].as_array_mut() {
                                for item in items {
                                    if let Some(item) = item.as_object_mut() {
                                        item.remove("notice_type");
                                        item.remove("level");
                                    }
                                }
                            }
                        }
                    }
                }
                if name == "claude_edit_preview_9" {
                    assert_eq!(actual["kind"], json!("Системная команда / контекст"));
                    if let Some(obj) = actual.as_object_mut() {
                        obj.insert("kind".into(), json!("Сообщение пользователя"));
                    }
                }
                if let Some(expected) = case.get("expected_records") {
                    let mut records_vec: Vec<Value> = jsonl::read_lines(&f.file)?
                        .iter()
                        .filter_map(|l| l.obj().ok())
                        .collect();
                    if name == "claude_edit_8" {
                        assert_eq!(records_vec[8]["isMeta"], json!(true));
                        records_vec[8].as_object_mut().unwrap().remove("isMeta");
                    } else if name == "claude_edit_9" {
                        assert_eq!(records_vec[9]["isMeta"], json!(true));
                        records_vec[9].as_object_mut().unwrap().remove("isMeta");
                    }
                    let records = json!(records_vec);
                    if &records != expected {
                        failures.push(format!(
                            "{name}: records mismatch\nactual: {records}\nexpected: {expected}"
                        ));
                    }
                } else {
                    let expected = &case["expected"];
                    let equal = if case["subset"] == true {
                        expected_subset(&actual, expected)
                    } else {
                        &actual == expected
                    };
                    if !equal {
                        failures.push(format!(
                            "{name}: response mismatch\nactual: {actual}\nexpected: {expected}"
                        ));
                    }
                }
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} parity discrepancies:\n{}",
        failures.len(),
        failures.join("\n\n")
    );
    Ok(())
}
fn copy_tree(src: &Path, dst: &Path) -> Result<()> {
    for e in walkdir::WalkDir::new(src).follow_links(false) {
        let e = e?;
        let path = dst.join(e.path().strip_prefix(src)?);
        if e.file_type().is_dir() {
            fs::create_dir_all(path)?;
        } else if e.file_type().is_file() {
            fs::copy(e.path(), path)?;
        }
    }
    Ok(())
}
#[test]
fn restores_old_python_codex_and_claude_backups() -> Result<()> {
    for kind in ["codex", "claude"] {
        let f = fixture(kind)?;
        let original = fs::read(&f.file)?;
        let cid = if kind == "codex" {
            "cx-oracle"
        } else {
            "cl-oracle"
        };
        let bdir = f.backup_root.join(cid).join("python-generated-backup");
        copy_tree(
            &Path::new(FIXTURES).join(format!("{kind}_legacy_backup")),
            &bdir,
        )?;
        fs::write(&f.file, b"{\"type\":\"changed\"}\n")?;
        f.provider.call(
            "restore",
            &json!({"backup_dir":bdir}),
            &OpContext::default(),
        )?;
        assert_eq!(
            fs::read(&f.file)?,
            original,
            "{kind} backup did not restore the original bytes"
        );
    }
    Ok(())
}

#[test]
fn restores_legacy_partial_tail_and_checksumless_lock_backups() -> Result<()> {
    for variant in ["tail", "lock"] {
        let f = fixture("codex")?;
        let original = fs::read(&f.file)?;
        let bdir = f
            .backup_root
            .join("cx-oracle")
            .join(format!("legacy-{variant}"));
        copy_tree(
            &Path::new(FIXTURES).join(format!("codex_legacy_{variant}_backup")),
            &bdir,
        )?;
        f.provider.call(
            "unlock_chat",
            &json!({"cid":"cx-oracle"}),
            &OpContext {
                auto_backup: false,
                ..Default::default()
            },
        )?;
        assert_ne!(fs::read(&f.file)?, original);
        f.provider.call(
            "restore",
            &json!({"backup_dir":bdir}),
            &OpContext::default(),
        )?;
        assert_eq!(
            fs::read(&f.file)?,
            original,
            "legacy {variant} backup failed"
        );
    }
    Ok(())
}

#[test]
fn multisegment_codex_stays_readonly_and_allows_delete_restore() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let root = dir.path().join("codex-real");
    let sessions = root.join("sessions");
    fs::create_dir_all(&sessions)?;
    let input = fs::read(Path::new(FIXTURES).join("codex.jsonl"))?;
    fs::write(sessions.join("rollout-one.jsonl"), &input)?;
    fs::write(sessions.join("rollout-two.jsonl"), &input)?;
    let p = CodexProvider::new(ProviderPaths {
        id: "codex".into(),
        name: "Codex".into(),
        short: "CX".into(),
        root,
        backup_root: dir.path().join("backups"),
    });
    let entries = p.call(
        "entries",
        &json!({"cid":"cx-oracle","include_tools":true}),
        &OpContext::default(),
    )?;
    assert_eq!(entries["writable"], false);
    assert!(entries["turns"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|t| t["items"].as_array().unwrap())
        .all(|it| it["actions"].as_array().unwrap().is_empty()));
    let key = entries["turns"][0]["items"][0]["key"].clone();
    let preview = p.call(
        "edit_preview",
        &json!({"cid":"cx-oracle","key":key}),
        &OpContext::default(),
    )?;
    assert_eq!(preview["writable"], false);
    assert!(p
        .call(
            "apply_edit",
            &json!({"cid":"cx-oracle","key":key,"revision":preview["revision"],"text":"forbidden"}),
            &OpContext::default()
        )
        .is_err());
    assert_eq!(fs::read(sessions.join("rollout-one.jsonl"))?, input);
    let deletion = p.call(
        "delete_conv",
        &json!({"cid":"cx-oracle"}),
        &OpContext::default(),
    )?;
    assert_eq!(deletion["files_removed"], 2);
    p.call(
        "restore",
        &json!({"backup_dir":deletion["backup"]}),
        &OpContext::default(),
    )?;
    assert_eq!(fs::read(sessions.join("rollout-one.jsonl"))?, input);
    assert_eq!(fs::read(sessions.join("rollout-two.jsonl"))?, input);
    Ok(())
}

#[cfg(unix)]
#[test]
fn codex_honors_the_native_thread_writer_lock() -> Result<()> {
    use fs2::FileExt;
    let f = fixture("codex")?;
    let sessions = f.provider.paths().root.join("sessions");
    let tid = "12345678-1234-1234-1234-123456789abc";
    let renamed = sessions.join(format!("rollout-{tid}.jsonl"));
    fs::rename(&f.file, &renamed)?;
    let lock_dir = f.provider.paths().root.join("thread-writer-locks");
    fs::create_dir_all(&lock_dir)?;
    let writer = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(lock_dir.join(format!("{tid}.lock")))?;
    writer.try_lock_exclusive()?;
    assert!(f.provider.probe_lock(Some("cx-oracle"))?.is_some());
    assert!(f
        .provider
        .call(
            "unlock_chat",
            &json!({"cid":"cx-oracle"}),
            &OpContext::default()
        )
        .is_err());
    FileExt::unlock(&writer)?;
    assert!(f.provider.probe_lock(Some("cx-oracle"))?.is_none());
    Ok(())
}
