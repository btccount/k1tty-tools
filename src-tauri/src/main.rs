#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use k1tty_core::service::{AppPaths, Service};
use serde_json::Value;
use std::{
    path::{Path, PathBuf},
    process::Command,
    sync::Arc,
};
use tauri::{Emitter, Manager, State};
use tauri_plugin_dialog::DialogExt;

type Core = Arc<Service>;

fn error(e: impl std::fmt::Display) -> String {
    e.to_string()
}

#[cfg(target_os = "windows")]
fn apply_windows_frame(window: &tauri::WebviewWindow) {
    #[link(name = "dwmapi")]
    extern "system" {
        fn DwmSetWindowAttribute(
            hwnd: isize,
            dwattribute: u32,
            pvattribute: *const std::ffi::c_void,
            cbattribute: u32,
        ) -> i32;
    }
    if let Ok(hwnd) = window.hwnd() {
        let raw = hwnd.0 as isize;
        let dark: u32 = 1;
        let round: u32 = 2;
        unsafe {
            let _ = DwmSetWindowAttribute(
                raw,
                20,
                (&dark as *const u32).cast(),
                std::mem::size_of::<u32>() as u32,
            );
            let _ = DwmSetWindowAttribute(
                raw,
                33,
                (&round as *const u32).cast(),
                std::mem::size_of::<u32>() as u32,
            );
        }
    }
}

#[tauri::command]
async fn initialize(core: State<'_, Core>) -> Result<Value, String> {
    Ok(core.hello())
}

#[tauri::command]
async fn rpc(
    core: State<'_, Core>,
    op: String,
    provider: Option<String>,
    data: Value,
) -> Result<Value, String> {
    let core = core.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        core.rpc(&op, provider.as_deref(), data)
            .map_err(|e| format!("{e:#}"))
    })
    .await
    .map_err(error)?
}

#[tauri::command]
async fn export_pack(core: State<'_, Core>, data: Value) -> Result<Value, String> {
    let core = core.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        core.export_pack(&data).map_err(|e| format!("{e:#}"))
    })
    .await
    .map_err(error)?
}

#[tauri::command]
async fn save_export(
    app: tauri::AppHandle,
    core: State<'_, Core>,
    token: String,
) -> Result<Value, String> {
    let core = core.inner().clone();
    let name = core.export_filename(&token).map_err(error)?;
    tauri::async_runtime::spawn_blocking(move || {
        let file = app
            .dialog()
            .file()
            .add_filter("K1tty Archive", &["k1tty"])
            .set_file_name(name)
            .blocking_save_file();
        let Some(file) = file else {
            return Ok(serde_json::json!({"cancelled":true}));
        };
        let path = file.into_path().map_err(error)?;
        core.save_export(&token, &path).map_err(error)?;
        Ok(serde_json::json!({"saved":true,"path":path}))
    })
    .await
    .map_err(error)?
}

#[tauri::command]
async fn export_batch(
    app: tauri::AppHandle,
    core: State<'_, Core>,
    items: Vec<Value>,
    preset: Option<String>,
    include_tools: Option<bool>,
) -> Result<Value, String> {
    if items.is_empty() {
        return Err("Не выбрано ни одного диалога.".into());
    }
    let core = core.inner().clone();
    let preset = preset.unwrap_or_else(|| "balanced".into());
    let include_tools = include_tools.unwrap_or(true);
    tauri::async_runtime::spawn_blocking(move || {
        let folder = app.dialog().file().blocking_pick_folder();
        let Some(folder) = folder else {
            return Ok(serde_json::json!({"cancelled":true}));
        };
        let dir = folder.into_path().map_err(error)?;
        let mut saved = 0usize;
        let mut failed: Vec<Value> = Vec::new();
        for item in &items {
            let provider = item["provider"].as_str().unwrap_or("");
            let cid = item["cid"].as_str().unwrap_or("");
            if provider.is_empty() || cid.is_empty() {
                continue;
            }
            let req = serde_json::json!({
                "provider": provider,
                "cid": cid,
                "preset": preset,
                "include_tools": include_tools
            });
            match core.export_pack(&req) {
                Ok(pack) => {
                    let token = pack["token"].as_str().unwrap_or("");
                    let raw_name = pack["filename"].as_str().unwrap_or("chat.k1tty");
                    let mut dest = dir.join(raw_name);
                    if dest.exists() {
                        let stem = dest
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("chat")
                            .to_owned();
                        for n in 2..=999 {
                            let cand = dir.join(format!("{stem}_{n}.k1tty"));
                            if !cand.exists() {
                                dest = cand;
                                break;
                            }
                        }
                    }
                    match core.save_export(token, &dest) {
                        Ok(_) => saved += 1,
                        Err(e) => failed.push(serde_json::json!({"provider":provider,"cid":cid,"error":format!("{e:#}")})),
                    }
                }
                Err(e) => failed.push(serde_json::json!({"provider":provider,"cid":cid,"error":format!("{e:#}")})),
            }
        }
        Ok(serde_json::json!({"saved":saved,"failed":failed,"folder":dir}))
    })
    .await
    .map_err(error)?
}

#[tauri::command]
async fn inspect_pack(
    core: State<'_, Core>,
    request: tauri::ipc::Request<'_>,
) -> Result<Value, String> {
    let bytes = match request.body() {
        tauri::ipc::InvokeBody::Raw(bytes) => bytes.clone(),
        _ => return Err("Ожидалось бинарное содержимое .k1tty".into()),
    };
    let core = core.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        core.inspect_pack(bytes).map_err(|e| format!("{e:#}"))
    })
    .await
    .map_err(error)?
}

#[tauri::command]
async fn import_pack(
    core: State<'_, Core>,
    token: String,
    target: String,
    title: Option<String>,
    include_tools: bool,
) -> Result<Value, String> {
    let core = core.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        core.import_pack(&token, &target, title.as_deref(), include_tools)
            .map_err(|e| format!("{e:#}"))
    })
    .await
    .map_err(error)?
}

fn decode_percent(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hi = (bytes[i + 1] as char).to_digit(16);
            let lo = (bytes[i + 2] as char).to_digit(16);
            if let (Some(h), Some(l)) = (hi, lo) {
                out.push(((h << 4) | l) as u8);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn strip_line_suffix(s: &str) -> &str {
    let base = s.split('#').next().unwrap_or(s);
    let trimmed = base.trim_end();
    if let Some((head, tail)) = trimmed.rsplit_once(':') {
        if !tail.is_empty()
            && tail.chars().all(|c| c.is_ascii_digit() || c == '-')
            && head.len() > 2
        {
            if let Some((h2, t2)) = head.rsplit_once(':') {
                if !t2.is_empty() && t2.chars().all(|c| c.is_ascii_digit()) && h2.len() > 2 {
                    return h2;
                }
            }
            return head;
        }
    }
    trimmed
}

fn expand_user_prefix(input: &str) -> String {
    if let Some(rest) = input
        .strip_prefix("~/")
        .or_else(|| input.strip_prefix("~\\"))
    {
        let home = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME"));
        if let Some(h) = home {
            let mut base = PathBuf::from(h);
            base.push(rest);
            return base.to_string_lossy().into_owned();
        }
    }
    #[cfg(target_os = "windows")]
    {
        if input.starts_with('%') {
            for var in ["USERPROFILE", "APPDATA", "LOCALAPPDATA", "TEMP", "TMP"] {
                let token = format!("%{var}%");
                if input
                    .get(..token.len())
                    .is_some_and(|prefix| prefix.eq_ignore_ascii_case(&token))
                {
                    if let Some(val) = std::env::var_os(var) {
                        return format!("{}{}", val.to_string_lossy(), &input[token.len()..]);
                    }
                }
            }
        }
    }
    input.to_owned()
}

fn normalize_reveal_path(raw: &str) -> PathBuf {
    let trimmed = raw
        .trim()
        .trim_matches(|c| c == '"' || c == '\'' || c == '`');
    let no_frag = strip_line_suffix(trimmed);
    let mut decoded = decode_percent(no_frag);
    if let Some(rest) = decoded
        .strip_prefix("file:///")
        .or_else(|| decoded.strip_prefix("FILE:///"))
    {
        #[cfg(target_os = "windows")]
        {
            decoded = rest.to_owned();
        }
        #[cfg(not(target_os = "windows"))]
        {
            decoded = format!("/{rest}");
        }
    } else if let Some(rest) = decoded
        .strip_prefix("file://")
        .or_else(|| decoded.strip_prefix("FILE://"))
    {
        #[cfg(target_os = "windows")]
        {
            if rest.len() >= 2 && rest.as_bytes()[1] == b':' {
                decoded = rest.to_owned();
            } else {
                decoded = format!("\\\\{}", rest.replace('/', "\\"));
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            decoded = format!("/{rest}");
        }
    }
    decoded = expand_user_prefix(&decoded);
    #[cfg(target_os = "windows")]
    {
        if decoded.starts_with('/') && decoded.len() >= 3 && decoded.as_bytes()[2] == b':' {
            decoded = decoded[1..].to_owned();
        }
        decoded = decoded.replace('/', "\\");
    }
    PathBuf::from(decoded)
}

fn open_in_file_manager(path: &Path, is_file: bool) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        let clean = path.to_string_lossy().replace('"', "");
        if is_file {
            Command::new("explorer.exe")
                .raw_arg(format!("/select,\"{clean}\""))
                .spawn()
                .map_err(error)?;
        } else {
            Command::new("explorer.exe")
                .arg(clean)
                .spawn()
                .map_err(error)?;
        }
        Ok(())
    }
    #[cfg(target_os = "macos")]
    {
        let mut cmd = Command::new("open");
        if is_file {
            cmd.arg("-R");
        }
        cmd.arg(path).spawn().map_err(error)?;
        Ok(())
    }
    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    {
        let target = if is_file {
            path.parent().unwrap_or(path)
        } else {
            path
        };
        Command::new("xdg-open")
            .arg(target)
            .spawn()
            .map_err(error)?;
        Ok(())
    }
}

#[tauri::command]
async fn reveal_path(path: String) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let target = normalize_reveal_path(&path);
        if target.as_os_str().is_empty() {
            return Err("Пустой путь.".into());
        }
        if target.is_file() {
            open_in_file_manager(&target, true)?;
            return Ok(serde_json::json!({"opened":true,"kind":"file","path":target}));
        }
        if target.is_dir() {
            open_in_file_manager(&target, false)?;
            return Ok(serde_json::json!({"opened":true,"kind":"dir","path":target}));
        }
        let mut parent = target.parent();
        for _ in 0..2 {
            if let Some(dir) = parent {
                if dir.is_dir() {
                    open_in_file_manager(dir, false)?;
                    return Ok(serde_json::json!({"opened":true,"kind":"parent","path":dir}));
                }
                parent = dir.parent();
            }
        }
        Err(format!("Путь не найден: {}", target.display()))
    })
    .await
    .map_err(error)?
}

fn main() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .setup(|app| {
            let handle = app.handle().clone();
            let paths = if let Some(root) = std::env::var_os("K1TTY_TEST_ROOT") {
                AppPaths::isolated(&PathBuf::from(root))
            } else {
                AppPaths::discover(app.path().app_data_dir()?)?
            };
            let core = Service::new(
                paths,
                Arc::new(move |message| {
                    let _ = handle.emit("k1tty-message", message);
                }),
            )?;
            core.start_watcher();
            app.manage(core);
            #[cfg(target_os = "windows")]
            if let Some(window) = app.get_webview_window("main") {
                apply_windows_frame(&window);
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            initialize,
            rpc,
            export_pack,
            save_export,
            export_batch,
            inspect_pack,
            import_pack,
            reveal_path
        ])
        .build(tauri::generate_context!())
        .expect("Failed to start K1tty Tools");
    app.run(|handle, event| {
        if let tauri::RunEvent::ExitRequested { api, .. } = event {
            if let Some(core) = handle.try_state::<Core>() {
                if core.active_jobs() > 0 {
                    api.prevent_exit();
                    let core = core.inner().clone();
                    let handle = handle.clone();
                    std::thread::spawn(move || {
                        while core.active_jobs() > 0 {
                            std::thread::sleep(std::time::Duration::from_millis(100));
                        }
                        handle.exit(0);
                    });
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_reveal_path_decodes_file_uris_and_line_suffixes() {
        #[cfg(target_os = "windows")]
        {
            let p = normalize_reveal_path(
                "file:///C:/Users/demo/Desktop/project%20dir/k1tty-tools/build.bat",
            );
            assert_eq!(
                p,
                PathBuf::from(r"C:\Users\demo\Desktop\project dir\k1tty-tools\build.bat")
            );

            let p2 = normalize_reveal_path(
                "file:///C:/Users/demo/Desktop/project%20dir/k1tty-tools/build.bat#L12-L40",
            );
            assert_eq!(
                p2,
                PathBuf::from(r"C:\Users\demo\Desktop\project dir\k1tty-tools\build.bat")
            );

            let p3 = normalize_reveal_path(r"C:\Users\demo\Desktop\project dir\main.rs:128:4");
            assert_eq!(
                p3,
                PathBuf::from(r"C:\Users\demo\Desktop\project dir\main.rs")
            );

            let p4 = normalize_reveal_path("file:///C:/%D0%BF%D0%B0%D0%BF%D0%BA%D0%B0/test.txt");
            assert_eq!(p4, PathBuf::from(r"C:\папка\test.txt"));
        }

        #[cfg(not(target_os = "windows"))]
        {
            let p = normalize_reveal_path("file:///home/demo/project%20dir/main.rs#L10-L20");
            assert_eq!(p, PathBuf::from("/home/demo/project dir/main.rs"));

            let p2 = normalize_reveal_path("/home/demo/project dir/main.rs:128:4");
            assert_eq!(p2, PathBuf::from("/home/demo/project dir/main.rs"));

            let p3 = normalize_reveal_path("file:///tmp/%D0%BF%D0%B0%D0%BF%D0%BA%D0%B0/test.txt");
            assert_eq!(p3, PathBuf::from("/tmp/папка/test.txt"));
        }
    }
}
