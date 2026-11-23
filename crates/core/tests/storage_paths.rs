use k1tty_core::storage::atomic_write;
use std::{fs, io, path::Path, process::Command};

#[test]
fn atomic_write_creates_missing_unicode_directories() {
    let root = tempfile::tempdir().unwrap();
    let target = root
        .path()
        .join("Новый архив")
        .join("Проект с пробелами")
        .join("диалог.jsonl");
    let contents = "{\"message\":\"Привет, k1ttyf\"}\n";

    atomic_write(&target, contents.as_bytes()).unwrap();

    assert_eq!(fs::read_to_string(&target).unwrap(), contents);
    assert_eq!(fs::read_dir(target.parent().unwrap()).unwrap().count(), 1);
}

#[test]
fn atomic_write_replaces_existing_file_and_preserves_permissions() {
    let root = tempfile::tempdir().unwrap();
    let target = root.path().join("existing.jsonl");
    fs::write(&target, b"previous contents").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&target, fs::Permissions::from_mode(0o640)).unwrap();
    }
    let before = fs::metadata(&target).unwrap().permissions();

    atomic_write(&target, b"replacement contents").unwrap();

    assert_eq!(fs::read(&target).unwrap(), b"replacement contents");
    let after = fs::metadata(&target).unwrap().permissions();
    assert_eq!(after.readonly(), before.readonly());
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(after.mode(), before.mode());
    }
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 1);
}

#[test]
fn atomic_write_reports_blocked_parent_without_overwriting_it() {
    let root = tempfile::tempdir().unwrap();
    let blocked = root.path().join("занятая папка");
    fs::write(&blocked, b"existing file must survive").unwrap();
    let target = blocked.join("dialog.jsonl");

    let error = atomic_write(&target, b"new contents").unwrap_err();

    let details = format!("{error:#}");
    assert!(details.contains("Не удалось создать папку"), "{details}");
    assert!(
        details.contains(&blocked.display().to_string()),
        "{details}"
    );
    assert!(details.contains(&target.display().to_string()), "{details}");
    assert!(error.chain().any(|cause| cause.is::<io::Error>()));
    assert_eq!(fs::read(&blocked).unwrap(), b"existing file must survive");
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 1);
}

#[test]
fn atomic_write_failed_replace_preserves_destination_and_removes_temporary_file() {
    let root = tempfile::tempdir().unwrap();
    let target = root.path().join("dialog.jsonl");
    fs::create_dir(&target).unwrap();
    let existing = target.join("existing.txt");
    fs::write(&existing, b"original data").unwrap();

    let error = atomic_write(&target, b"replacement data").unwrap_err();

    let details = format!("{error:#}");
    assert!(
        details.contains("Не удалось атомарно сохранить файл"),
        "{details}"
    );
    assert!(details.contains(&target.display().to_string()), "{details}");
    assert!(error.chain().any(|cause| cause.is::<io::Error>()));
    assert_eq!(fs::read(existing).unwrap(), b"original data");
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 1);
}

#[test]
fn atomic_write_and_jsonl_updates_support_deep_paths_beyond_windows_max_path() {
    let root = tempfile::tempdir().unwrap();
    let mut folder = root.path().to_path_buf();
    for index in 0..8 {
        folder.push(format!("project-{index}-{}", "x".repeat(36)));
    }
    let target = folder.join("диалог с пробелами.jsonl");
    assert!(target.as_os_str().len() > 320);

    atomic_write(&target, b"first version").unwrap();
    atomic_write(&target, b"second version").unwrap();
    k1tty_core::jsonl::atomic_write(&target, b"edited version").unwrap();

    assert_eq!(fs::read(&target).unwrap(), b"edited version");
    assert_eq!(fs::read_dir(&folder).unwrap().count(), 1);
}

#[test]
fn atomic_write_accepts_bare_relative_filename() {
    let child_marker = "K1TTY_STORAGE_RELATIVE_PATH_TEST";
    let filename = "диалог с пробелами.jsonl";
    if std::env::var_os(child_marker).is_some() {
        atomic_write(Path::new(filename), b"relative contents").unwrap();
        return;
    }
    let root = tempfile::tempdir().unwrap();
    let result = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "atomic_write_accepts_bare_relative_filename",
            "--nocapture",
        ])
        .current_dir(root.path())
        .env(child_marker, "1")
        .output()
        .unwrap();

    assert!(
        result.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(
        fs::read(root.path().join(filename)).unwrap(),
        b"relative contents"
    );
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 1);
}

#[cfg(windows)]
#[test]
fn atomic_write_accepts_extended_windows_paths() {
    let root = tempfile::tempdir().unwrap();
    let extended = root.path().canonicalize().unwrap();
    let target = extended.join("Новая папка").join("диалог.jsonl");
    assert!(target.to_string_lossy().starts_with(r"\\?\"));

    atomic_write(&target, b"extended path contents").unwrap();

    assert_eq!(fs::read(&target).unwrap(), b"extended path contents");
}
