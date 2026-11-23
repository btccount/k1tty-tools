use anyhow::{Context, Result};
use k1tty_core::pack::{pack_chat, unpack_chat};
use serde_json::Value;
use std::{env, fs, path::PathBuf};
fn main() -> Result<()> {
    let args: Vec<_> = env::args_os().collect();
    let input = args.get(1).context("Expected document.json")?;
    let output = PathBuf::from(args.get(2).context("Expected output directory")?);
    let doc: Value = serde_json::from_slice(&fs::read(input)?)?;
    fs::create_dir_all(&output)?;
    for preset in ["fast", "balanced", "ultra"] {
        let (bytes, _) = pack_chat(&doc, preset, true)?;
        let decoded = unpack_chat(&bytes)?;
        fs::write(output.join(format!("rust_{preset}.k1tty")), &bytes)?;
        fs::write(
            output.join(format!("rust_{preset}.json")),
            serde_json::to_vec(&decoded)?,
        )?;
    }
    Ok(())
}
