use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path
        .parent()
        .with_context(|| format!("У файла «{}» нет родительской папки", path.display()))?;
    let parent = if parent.as_os_str().is_empty() {
        Path::new(".")
    } else {
        parent
    };
    fs::create_dir_all(parent).with_context(|| {
        format!(
            "Не удалось создать папку «{}» для записи файла «{}»",
            parent.display(),
            path.display()
        )
    })?;
    #[cfg(windows)]
    let parent = parent.canonicalize().with_context(|| {
        format!(
            "Не удалось определить полный путь папки «{}» для записи файла «{}»",
            parent.display(),
            path.display()
        )
    })?;
    #[cfg(windows)]
    let destination =
        parent
            .join(path.file_name().with_context(|| {
                format!("Не удалось определить имя файла «{}»", path.display())
            })?);
    #[cfg(not(windows))]
    let destination = path;
    let mut tmp = tempfile::NamedTempFile::new_in(&parent).with_context(|| {
        format!(
            "Не удалось создать временный файл в папке «{}» для записи файла «{}»",
            parent.display(),
            path.display()
        )
    })?;
    if let Ok(meta) = fs::metadata(&destination) {
        if meta.is_file() {
            tmp.as_file()
                .set_permissions(meta.permissions())
                .with_context(|| {
                    format!(
                        "Не удалось сохранить права доступа файла «{}»",
                        path.display()
                    )
                })?;
        }
    }
    tmp.write_all(bytes).with_context(|| {
        format!(
            "Не удалось записать временный файл «{}» для файла «{}»",
            tmp.path().display(),
            path.display()
        )
    })?;
    tmp.as_file().sync_all().with_context(|| {
        format!(
            "Не удалось синхронизировать временный файл «{}» для файла «{}»",
            tmp.path().display(),
            path.display()
        )
    })?;
    tmp.persist(&destination)
        .map_err(|e| e.error)
        .with_context(|| format!("Не удалось атомарно сохранить файл «{}»", path.display()))?;
    #[cfg(unix)]
    fs::File::open(parent)
        .and_then(|directory| directory.sync_all())
        .with_context(|| {
            format!(
                "Не удалось синхронизировать папку «{}» после записи файла «{}»",
                parent.display(),
                path.display()
            )
        })?;
    Ok(())
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

pub fn dir_size(path: &Path) -> u64 {
    walkdir::WalkDir::new(path)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
        .filter_map(|e| e.metadata().ok())
        .map(|m| m.len())
        .sum()
}

pub fn read_json(path: &Path) -> Result<Value> {
    serde_json::from_slice(
        &fs::read(path).with_context(|| format!("Не удалось прочитать {}", path.display()))?,
    )
    .map_err(Into::into)
}

pub fn read_jsonl(path: &Path) -> Result<Vec<Value>> {
    use std::io::BufRead;
    let input = std::io::BufReader::new(fs::File::open(path)?);
    let mut out = Vec::new();
    for line in input.split(b'\n') {
        let line = line?;
        if let Ok(value) = serde_json::from_slice(&line) {
            out.push(value);
        }
    }
    Ok(out)
}

pub fn timestamp() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Micros, true)
}

pub fn backup_path(root: &Path, requested: &str) -> Result<PathBuf> {
    let root = root
        .canonicalize()
        .context("Папка резервных копий не найдена")?;
    let p = Path::new(requested)
        .canonicalize()
        .context("Резервная копия не найдена")?;
    if !p.starts_with(&root) || p == root || p.parent() == Some(root.as_path()) || !p.is_dir() {
        bail!("Недопустимый путь резервной копии.");
    }
    Ok(p)
}

pub fn backup_summary(paths: &crate::provider::ProviderPaths) -> Value {
    let mut count = 0u64;
    let mut bytes = 0u64;
    if let Ok(dirs) = fs::read_dir(&paths.backup_root) {
        for dir in dirs.flatten() {
            if dir.file_type().is_ok_and(|t| t.is_dir()) {
                if let Ok(entries) = fs::read_dir(dir.path()) {
                    for entry in entries.flatten() {
                        if entry.file_type().is_ok_and(|t| t.is_dir()) {
                            count += 1;
                            bytes += dir_size(&entry.path());
                        }
                    }
                }
            }
        }
    }
    json!({"provider":paths.id,"name":paths.name,"short":paths.short,"root":paths.backup_root,"count":count,"bytes":bytes})
}

pub fn delete_backup(paths: &crate::provider::ProviderPaths, requested: &str) -> Result<Value> {
    let p = backup_path(&paths.backup_root, requested)?;
    let bytes = dir_size(&p);
    fs::remove_dir_all(&p)?;
    if let Some(parent) = p.parent() {
        let _ = fs::remove_dir(parent);
    }
    Ok(
        json!({"provider":paths.id,"name":paths.name,"deleted_count":1,"freed_bytes":bytes,"backup_dir":p}),
    )
}

pub fn delete_all_backups(paths: &crate::provider::ProviderPaths) -> Result<Value> {
    let mut count = 0u64;
    let mut freed = 0u64;
    if paths.backup_root.is_dir() {
        for dir in fs::read_dir(&paths.backup_root)?.flatten() {
            if !dir.file_type()?.is_dir() {
                continue;
            }
            for entry in fs::read_dir(dir.path())?.flatten() {
                if !entry.file_type()?.is_dir() {
                    continue;
                }
                let report = delete_backup(paths, &entry.path().to_string_lossy())?;
                count += 1;
                freed += report["freed_bytes"].as_u64().unwrap_or(0);
            }
            let _ = fs::remove_dir(dir.path());
        }
    }
    Ok(json!({"provider":paths.id,"name":paths.name,"deleted_count":count,"freed_bytes":freed}))
}
