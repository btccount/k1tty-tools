use anyhow::{bail, Result};
use serde_json::Value;
use std::{path::PathBuf, sync::Arc};

#[derive(Clone, Debug)]
pub struct ProviderPaths {
    pub id: String,
    pub name: String,
    pub short: String,
    pub root: PathBuf,
    pub backup_root: PathBuf,
}

#[derive(Clone)]
pub struct OpContext {
    pub auto_backup: bool,
    pub auto_restart_hub: bool,
    pub on_phase: Arc<dyn Fn(&str) + Send + Sync>,
}

impl Default for OpContext {
    fn default() -> Self {
        Self {
            auto_backup: true,
            auto_restart_hub: false,
            on_phase: Arc::new(|_| {}),
        }
    }
}

impl OpContext {
    pub fn phase(&self, phase: &str) {
        (self.on_phase)(phase);
    }
}

#[derive(Clone, Copy, Debug)]
pub struct EntryRef<'a> {
    pub key: &'a str,
    pub kind: &'a str,
    pub subtype: &'a str,
    pub label: &'a str,
    pub ts: &'a str,
    pub text: &'a str,
}

pub fn floor_char_boundary(s: &str, max: usize) -> usize {
    if max >= s.len() {
        return s.len();
    }
    let mut idx = max;
    while idx > 0 && !s.is_char_boundary(idx) {
        idx -= 1;
    }
    idx
}

pub fn truncate_preview_item(item: &mut Value) {
    let Some(obj) = item.as_object_mut() else {
        return;
    };
    let is_tool = obj.get("kind").and_then(Value::as_str) == Some("tool")
        || obj.get("subtype").and_then(Value::as_str) == Some("thinking");
    let limit = if is_tool { 32 * 1024 } else { 128 * 1024 };
    let Some(orig_len) = obj.get("text").and_then(Value::as_str).map(str::len) else {
        return;
    };
    if orig_len <= limit {
        return;
    }
    if let Some(Value::String(text)) = obj.get_mut("text") {
        let cut = floor_char_boundary(text, limit);
        text.truncate(cut);
        text.shrink_to_fit();
    }
    obj.insert("text_truncated".into(), Value::Bool(true));
    obj.insert("full_bytes".into(), Value::from(orig_len));
}

pub trait Provider: Send + Sync {
    fn paths(&self) -> &ProviderPaths;
    fn call(&self, op: &str, data: &Value, ctx: &OpContext) -> Result<Value>;
    fn visit_entries(&self, cid: &str, f: &mut dyn FnMut(EntryRef<'_>) -> bool) -> Result<()> {
        let ctx = OpContext::default();
        let mut pages: Vec<Vec<Value>> = Vec::new();
        let mut before: Option<String> = None;
        loop {
            let mut req = serde_json::json!({"cid": cid, "include_tools": true, "limit": 150});
            if let Some(cur) = &before {
                req["before"] = serde_json::json!(cur);
            }
            let resp = self.call("entries", &req, &ctx)?;
            let mut items = Vec::new();
            if let Some(turns) = resp["turns"].as_array() {
                for turn in turns {
                    if let Some(arr) = turn["items"].as_array() {
                        items.extend(arr.iter().cloned());
                    }
                }
            } else if let Some(arr) = resp["entries"].as_array() {
                items.extend(arr.iter().cloned());
            }
            if items.is_empty() {
                break;
            }
            pages.push(items);
            if resp["has_more"] != true {
                break;
            }
            let next = resp["cursor"]
                .as_str()
                .or_else(|| resp["next_cursor"].as_str())
                .unwrap_or("")
                .to_owned();
            if next.is_empty() || before.as_deref() == Some(next.as_str()) {
                break;
            }
            before = Some(next);
        }
        for page in pages.into_iter().rev() {
            for item in &page {
                let entry = EntryRef {
                    key: item["key"].as_str().unwrap_or(""),
                    kind: item["kind"].as_str().unwrap_or(""),
                    subtype: item["subtype"].as_str().unwrap_or(""),
                    label: item["label"].as_str().unwrap_or(""),
                    ts: item["ts"].as_str().unwrap_or(""),
                    text: item["text"].as_str().unwrap_or(""),
                };
                if !f(entry) {
                    return Ok(());
                }
            }
        }
        Ok(())
    }
    fn read_entry_text(&self, _cid: &str, _key: &str) -> Result<Option<String>> {
        Ok(None)
    }
    fn probe_lock(&self, _cid: Option<&str>) -> Result<Option<String>> {
        Ok(None)
    }
    fn lock_status(&self, cid: Option<&str>) -> Result<Value> {
        let reason = self.probe_lock(cid)?;
        Ok(serde_json::json!({
            "locked": reason.is_some(),
            "reason": reason.unwrap_or_default(),
        }))
    }
    fn set_auto_restart_hub(&self, _enabled: bool) {}
    fn content_signature(&self, _cid: &str) -> Option<String> {
        None
    }
    fn after_external_import(&self, _cid: &str, _ctx: &OpContext) -> Result<()> {
        Ok(())
    }
    fn assert_writable(&self, cid: Option<&str>) -> Result<()> {
        if let Some(reason) = self.probe_lock(cid)? {
            bail!(reason);
        }
        Ok(())
    }
}

pub fn required<'a>(v: &'a Value, key: &str) -> Result<&'a str> {
    v.get(key)
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| anyhow::anyhow!("Требуется {key}."))
}

pub fn valid_id(id: &str) -> Result<()> {
    if id.is_empty()
        || matches!(id, "." | "..")
        || id.chars().any(|c| matches!(c, '/' | '\\' | '\0' | ':'))
    {
        bail!("Недопустимый ID диалога.");
    }
    Ok(())
}
