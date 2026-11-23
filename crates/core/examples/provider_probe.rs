use k1tty_core::{
    antigravity::AntigravityProvider,
    claude::ClaudeProvider,
    codex::CodexProvider,
    opencode::OpenCodeProvider,
    provider::{OpContext, Provider, ProviderPaths},
};
use std::{
    io::{self, BufRead},
    path::PathBuf,
};
fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let id = args
        .next()
        .ok_or_else(|| anyhow::anyhow!("provider and data-root arguments required"))?;
    let root = PathBuf::from(
        args.next()
            .ok_or_else(|| anyhow::anyhow!("Explicit data-root required"))?,
    );
    let paths = ProviderPaths {
        id: id.clone(),
        name: id.clone(),
        short: id.clone(),
        backup_root: root.parent().unwrap().join(format!("{id}_backups")),
        root,
    };
    let provider: Box<dyn Provider> = match id.as_str() {
        "antigravity" | "antigravity_cli" => Box::new(AntigravityProvider::new(paths)),
        "codex" => Box::new(CodexProvider::new(paths)),
        "claude" => Box::new(ClaudeProvider::new(paths)),
        "opencode" => Box::new(OpenCodeProvider::new(paths)),
        _ => anyhow::bail!("Unknown provider"),
    };
    for line in io::stdin().lock().lines() {
        let line = line?;
        let value: serde_json::Value = serde_json::from_str(&line)?;
        let ctx = OpContext {
            auto_backup: value["auto_backup"].as_bool().unwrap_or(true),
            ..Default::default()
        };
        let result = provider.call(value["op"].as_str().unwrap_or(""), &value["data"], &ctx);
        match result {
            Ok(result) => println!("{}", serde_json::json!({"ok":true,"result":result})),
            Err(e) => println!(
                "{}",
                serde_json::json!({"ok":false,"error":format!("{e:#}")})
            ),
        }
    }
    Ok(())
}
