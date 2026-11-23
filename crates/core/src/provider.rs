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
    pub on_phase: Arc<dyn Fn(&str) + Send + Sync>,
}

impl Default for OpContext {
    fn default() -> Self {
        Self {
            auto_backup: true,
            on_phase: Arc::new(|_| {}),
        }
    }
}

impl OpContext {
    pub fn phase(&self, phase: &str) {
        (self.on_phase)(phase);
    }
}

pub trait Provider: Send + Sync {
    fn paths(&self) -> &ProviderPaths;
    fn call(&self, op: &str, data: &Value, ctx: &OpContext) -> Result<Value>;
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
