use k1tty_core::{
    antigravity::AntigravityProvider,
    claude::ClaudeProvider,
    codex::CodexProvider,
    converter::{convert_chat, export_provider_chat, import_doc_to_provider},
    opencode::OpenCodeProvider,
    provider::{OpContext, Provider, ProviderPaths},
};
use serde_json::{json, Value};
use std::{fs, path::Path};

const REFERENCE: &str = include_str!("fixtures/converter/reference_matrix.json");
const SAMPLE: &str = include_str!("fixtures/converter/sample.json");
const PROVIDERS: &[&str] = &[
    "antigravity",
    "antigravity_cli",
    "codex",
    "claude",
    "opencode",
];

fn provider(id: &str, root: &Path) -> Box<dyn Provider> {
    let (name, short) = match id {
        "antigravity" => ("Antigravity", "AG"),
        "antigravity_cli" => ("Antigravity CLI", "AGC"),
        "codex" => ("Codex", "CX"),
        "claude" => ("Claude", "CL"),
        _ => ("OpenCode", "OC"),
    };
    let paths = ProviderPaths {
        id: id.into(),
        name: name.into(),
        short: short.into(),
        root: root.join("sandbox"),
        backup_root: root.join("backups"),
    };
    match id {
        "antigravity" | "antigravity_cli" => Box::new(AntigravityProvider::new(paths)),
        "codex" => Box::new(CodexProvider::new(paths)),
        "claude" => Box::new(ClaudeProvider::new(paths)),
        _ => Box::new(OpenCodeProvider::new(paths)),
    }
}
fn normalize(doc: &Value) -> Value {
    let tool = regex::Regex::new(r"toolu_[0-9a-f]{24}").unwrap();
    let uuid =
        regex::Regex::new(r"[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}").unwrap();
    let entries: Vec<Value> = doc["entries"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| {
            let mut out = json!({});
            for key in ["kind", "subtype", "label", "text", "tool", "status", "code"] {
                out[key] = if let Some(s) = e[key].as_str() {
                    json!(uuid
                        .replace_all(&tool.replace_all(s, "toolu_<id>"), "<uuid>")
                        .to_string())
                } else {
                    e[key].clone()
                };
            }
            out["ts_ms"] = json!(e["ts"]
                .as_str()
                .and_then(|t| chrono::DateTime::parse_from_rfc3339(t).ok())
                .map(|d| d.timestamp_millis())
                .unwrap_or(0));
            out
        })
        .collect();
    json!({"title":doc["meta"]["title"],"entries":entries})
}

#[test]
fn all_twenty_directions_match_python_with_and_without_tools() {
    let expected: Value = serde_json::from_str(REFERENCE).unwrap();
    let temp = tempfile::tempdir().unwrap();
    let project = temp.path().join("project");
    fs::create_dir(&project).unwrap();
    let mut sample: Value = serde_json::from_str(SAMPLE).unwrap();
    sample["meta"]["directory"] = json!(project);
    for source in PROVIDERS {
        let src = provider(source, &temp.path().join(format!("src-{source}")));
        let seeded = import_doc_to_provider(&sample, src.as_ref(), None, true).unwrap();
        let cid = seeded["cid"].as_str().unwrap();
        let export = export_provider_chat(src.as_ref(), cid, true).unwrap();
        assert_eq!(
            normalize(&export),
            expected[format!("{source}|source")],
            "source export: {source}"
        );
        for target in PROVIDERS {
            if target == source {
                continue;
            }
            for tools in [false, true] {
                let dst = provider(
                    target,
                    &temp.path().join(format!("{source}-{target}-{tools}")),
                );
                let result = convert_chat(
                    src.as_ref(),
                    cid,
                    dst.as_ref(),
                    None,
                    tools,
                    &OpContext::default(),
                )
                .unwrap();
                let target_id = result["cid"].as_str().unwrap();
                let exported = export_provider_chat(dst.as_ref(), target_id, true).unwrap();
                let key = format!("{source}|{target}|{tools}");
                assert_eq!(normalize(&exported), expected[&key], "conversion {key}");
                let view = dst
                    .call(
                        "entries",
                        &json!({"cid":target_id,"include_tools":true,"limit":80}),
                        &OpContext::default(),
                    )
                    .unwrap();
                let turns = view["turns"].as_array().unwrap();
                assert!(!turns.is_empty(), "empty view after {key}");
                for item in turns
                    .iter()
                    .flat_map(|turn| turn["items"].as_array().unwrap())
                {
                    if item["actions"]
                        .as_array()
                        .is_some_and(|a| a.iter().any(|v| v == "edit"))
                    {
                        let preview = dst
                            .call(
                                "edit_preview",
                                &json!({"cid":target_id,"key":item["key"]}),
                                &OpContext::default(),
                            )
                            .unwrap_or_else(|e| panic!("preview {key}: {e}"));
                        assert!(
                            preview["text"].is_string(),
                            "missing editor text after {key}"
                        );
                    }
                }
            }
        }
    }
}
