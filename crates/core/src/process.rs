use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque},
    fs,
    path::Path,
    sync::{Mutex, OnceLock},
    time::{Duration, Instant},
};
use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, Signal, System, UpdateKind};

pub type ProcessMap = BTreeMap<String, Vec<String>>;

fn empty_map() -> ProcessMap {
    [
        "antigravity",
        "antigravity_cli",
        "codex",
        "claude",
        "opencode",
    ]
    .into_iter()
    .map(|s| (s.to_owned(), Vec::new()))
    .collect()
}

fn has_token(cmd: &str, target: &str) -> bool {
    cmd.split_whitespace()
        .any(|raw| raw.trim_matches(|c| c == '"' || c == '\'') == target)
}

fn is_shell(name: &str) -> bool {
    matches!(
        name,
        "cmd.exe"
            | "cmd"
            | "powershell.exe"
            | "powershell"
            | "pwsh.exe"
            | "pwsh"
            | "wt.exe"
            | "windowsterminal.exe"
            | "conhost.exe"
            | "openconsole.exe"
            | "bash.exe"
            | "bash"
            | "sh.exe"
            | "sh"
            | "zsh"
            | "fish"
            | "nu.exe"
            | "nu"
            | "mintty.exe"
            | "alacritty.exe"
            | "alacritty"
            | "wezterm-gui.exe"
            | "wezterm-gui"
            | "kitty"
            | "iterm2"
            | "terminal"
            | "tmux"
            | "screen"
    )
}

fn is_ignored_wrapper(name: &str) -> bool {
    matches!(
        name,
        "tunnel-client.exe"
            | "tunnel-client"
            | "codex web gpt.exe"
            | "codex-web-gpt.exe"
            | "codex-web-gpt"
            | "update.exe"
            | "squirrel.exe"
    )
}

fn is_desktop_label(label: &str) -> bool {
    matches!(
        label,
        "Antigravity Desktop" | "Codex Desktop" | "Claude Desktop" | "OpenCode Desktop"
    )
}

fn is_cli_utility(cmd: &str) -> bool {
    [
        "--version",
        "-v",
        "--help",
        "-h",
        "help",
        "completion",
        "update",
        "upgrade",
        "doctor",
        "login",
        "logout",
        "setup-token",
    ]
    .iter()
    .any(|flag| has_token(cmd, flag))
}

fn has_claude_cli_flags(cmd: &str) -> bool {
    [
        "--output-format",
        "--input-format",
        "--permission-prompt-tool",
        "--resume",
        "--print",
    ]
    .iter()
    .any(|s| cmd.contains(s))
}

fn electron_helper(cmd: &str) -> bool {
    cmd.split_whitespace().any(|raw| {
        let tok = raw.trim_matches(|c| c == '"' || c == '\'');
        tok.starts_with("--type=")
            || tok.starts_with("/prefetch:")
            || tok.starts_with("--squirrel")
            || tok.starts_with("--crash-reporter")
            || matches!(
                tok,
                "--hidden"
                    | "--background"
                    | "--start-in-tray"
                    | "--autostart"
                    | "--minimized"
                    | "--headless"
                    | "--tray"
                    | "--no-window"
                    | "--silent"
                    | "--updated"
            )
    })
}

pub fn classify(
    name: &str,
    exe: &str,
    cmd: &str,
    parent: &str,
) -> Option<(&'static str, &'static str)> {
    let helper = electron_helper(cmd);
    match name {
        "agy.exe" | "agy" | "antigravity-cli.exe" | "antigravity-cli"
            if !helper && !is_cli_utility(cmd) =>
        {
            Some(("antigravity_cli", "Antigravity CLI"))
        }
        "antigravity.exe" | "antigravity"
            if !helper
                && parent != name
                && !exe.contains("helper")
                && !cmd.contains("sidecar_sdk")
                && !cmd.contains("--import=") =>
        {
            Some(("antigravity", "Antigravity Desktop"))
        }
        "language_server.exe" | "language_server"
            if exe.contains("antigravity-cli") || cmd.contains("antigravity-cli") =>
        {
            Some(("antigravity_cli", "Antigravity CLI Hub"))
        }
        "language_server.exe" | "language_server"
            if exe.contains("antigravity") || cmd.contains("antigravity") =>
        {
            Some(("antigravity", "Antigravity Hub"))
        }
        "chatgpt.exe" | "chatgpt"
            if !helper
                && parent != name
                && (exe.contains("openai.codex") || exe.contains("/codex.app/")) =>
        {
            Some(("codex", "Codex Desktop"))
        }
        "codex.exe" | "codex" if !helper => {
            if is_cli_utility(cmd) || has_token(cmd, "sandbox") || has_token(cmd, "windows-sandbox")
            {
                return None;
            }
            if cmd.contains("app-server") || cmd.contains("exec-server") {
                if is_ignored_wrapper(parent) {
                    return None;
                }
                return Some((
                    "codex",
                    if cmd.contains("app-server") {
                        "Codex App Server"
                    } else {
                        "Codex Exec Server"
                    },
                ));
            }
            Some((
                "codex",
                if exe.contains("/codex.app/contents/macos/") {
                    "Codex Desktop"
                } else {
                    "Codex CLI"
                },
            ))
        }
        "claude.exe" | "claude" if !helper => {
            if is_cli_utility(cmd) {
                return None;
            }
            let cli_flags = has_claude_cli_flags(cmd);
            if parent == name && !exe.contains("claude-code") && !cli_flags {
                return None;
            }
            let unreadable_desktop =
                name == "claude.exe" && exe.is_empty() && cmd.is_empty() && !is_shell(parent);
            let desktop = (exe.contains("windowsapps\\claude_")
                || exe.contains("anthropicclaude")
                || exe.contains("/claude.app/contents/macos/")
                || unreadable_desktop)
                && !exe.contains("claude-code")
                && !cli_flags;
            Some((
                "claude",
                if desktop {
                    "Claude Desktop"
                } else {
                    "Claude Code"
                },
            ))
        }
        "opencode.exe" | "opencode" | "opencode-desktop" | "opencode-deskto" if !helper => {
            if parent == name || is_cli_utility(cmd) {
                return None;
            }
            let unreadable_desktop =
                name == "opencode.exe" && exe.is_empty() && cmd.is_empty() && !is_shell(parent);
            let desktop = exe.contains("ai.opencode.desktop")
                || exe.contains("/opencode.app/contents/macos/")
                || name.starts_with("opencode-deskt")
                || (exe.contains("opencode.exe") && !exe.contains("cli"))
                || unreadable_desktop;
            Some((
                "opencode",
                if desktop {
                    "OpenCode Desktop"
                } else {
                    "OpenCode CLI"
                },
            ))
        }
        "opencode-cli.exe" | "opencode-cli"
            if !helper && !cmd.contains("--service") && !is_cli_utility(cmd) =>
        {
            Some((
                "opencode",
                if has_token(cmd, "serve") {
                    "OpenCode Server"
                } else {
                    "OpenCode CLI"
                },
            ))
        }
        "node.exe" | "node" => {
            if is_cli_utility(cmd)
                || [
                    "computer-use",
                    "cua-repl",
                    "host-daemon",
                    "sidecar_sdk",
                    "npm-cli.js",
                    "npx-cli.js",
                    "pnpm",
                    "yarn",
                    "esbuild",
                    "vite",
                    "webpack",
                    "tsserver",
                    "eslint",
                    "prettier",
                ]
                .iter()
                .any(|skip| cmd.contains(skip))
            {
                return None;
            }
            if cmd.contains("@anthropic-ai\\claude-code")
                || cmd.contains("@anthropic-ai/claude-code")
            {
                Some(("claude", "Claude Code"))
            } else if cmd.contains("@openai\\codex") || cmd.contains("@openai/codex") {
                Some(("codex", "Codex CLI"))
            } else if cmd.contains("opencode-ai")
                || cmd.contains("node_modules\\opencode")
                || cmd.contains("node_modules/opencode")
                || cmd.contains("\\bin\\opencode")
                || cmd.contains("/bin/opencode")
            {
                Some(("opencode", "OpenCode CLI"))
            } else {
                None
            }
        }
        _ => None,
    }
}

fn codex_bundle(exe: &Path) -> bool {
    let path = exe
        .to_string_lossy()
        .replace('\\', "/")
        .to_ascii_lowercase();
    if !path.contains("/chatgpt.app/contents/macos/") {
        return false;
    }
    let Some(contents) = exe.parent().and_then(Path::parent) else {
        return false;
    };
    if contents
        .join("Frameworks/Codex Framework.framework")
        .is_dir()
        || contents.join("Resources/codex-cli").is_dir()
    {
        return true;
    }
    let Ok(v) = plist::Value::from_file(contents.join("Info.plist")) else {
        return false;
    };
    v.as_dictionary()
        .and_then(|d| d.get("CFBundleIdentifier"))
        .and_then(plist::Value::as_string)
        .is_some_and(|s| s.to_ascii_lowercase().contains("codex"))
}

#[cfg(windows)]
mod win {
    use std::collections::HashSet;

    #[repr(C)]
    struct Rect {
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
    }

    type Hwnd = isize;
    type EnumProc = unsafe extern "system" fn(Hwnd, isize) -> i32;

    const GW_OWNER: u32 = 4;
    const GWL_EXSTYLE: i32 = -20;
    const WS_EX_TOOLWINDOW: u32 = 0x0000_0080;
    const WS_EX_APPWINDOW: u32 = 0x0004_0000;
    const DWMWA_CLOAKED: u32 = 14;
    const DWM_CLOAKED_APP: u32 = 1;

    #[link(name = "user32")]
    extern "system" {
        fn EnumWindows(lp_enum_func: EnumProc, l_param: isize) -> i32;
        fn IsWindowVisible(hwnd: Hwnd) -> i32;
        fn IsIconic(hwnd: Hwnd) -> i32;
        fn GetWindow(hwnd: Hwnd, cmd: u32) -> Hwnd;
        fn GetWindowLongW(hwnd: Hwnd, index: i32) -> i32;
        fn GetWindowRect(hwnd: Hwnd, rect: *mut Rect) -> i32;
        fn GetWindowThreadProcessId(hwnd: Hwnd, pid: *mut u32) -> u32;
    }

    #[link(name = "dwmapi")]
    extern "system" {
        fn DwmGetWindowAttribute(hwnd: Hwnd, attribute: u32, value: *mut u32, size: u32) -> i32;
    }

    unsafe extern "system" fn enum_proc(hwnd: Hwnd, l_param: isize) -> i32 {
        if IsWindowVisible(hwnd) == 0 {
            return 1;
        }
        let mut cloaked: u32 = 0;
        if DwmGetWindowAttribute(
            hwnd,
            DWMWA_CLOAKED,
            &mut cloaked,
            std::mem::size_of::<u32>() as u32,
        ) == 0
            && (cloaked & DWM_CLOAKED_APP) != 0
        {
            return 1;
        }
        let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE) as u32;
        if (ex_style & WS_EX_TOOLWINDOW) != 0 && (ex_style & WS_EX_APPWINDOW) == 0 {
            return 1;
        }
        if GetWindow(hwnd, GW_OWNER) != 0 && (ex_style & WS_EX_APPWINDOW) == 0 {
            return 1;
        }
        if IsIconic(hwnd) == 0 {
            let mut rect = Rect {
                left: 0,
                top: 0,
                right: 0,
                bottom: 0,
            };
            if GetWindowRect(hwnd, &mut rect) == 0 {
                return 1;
            }
            if rect.right - rect.left < 80 || rect.bottom - rect.top < 40 {
                return 1;
            }
        }
        let mut pid: u32 = 0;
        GetWindowThreadProcessId(hwnd, &mut pid);
        if pid != 0 {
            let set = &mut *(l_param as *mut HashSet<u32>);
            set.insert(pid);
        }
        1
    }

    pub fn visible_window_pids() -> HashSet<u32> {
        let mut pids = HashSet::new();
        unsafe {
            EnumWindows(enum_proc, (&mut pids as *mut HashSet<u32>) as isize);
        }
        pids
    }
}

struct ProcEntry {
    pid: u32,
    parent_pid: Option<u32>,
    name: String,
    exe: String,
    cmd: String,
    codex_bundle: bool,
}

fn subtree_has_visible(
    root: u32,
    children: &HashMap<u32, Vec<u32>>,
    visible: &HashSet<u32>,
) -> bool {
    let mut queue = VecDeque::from([root]);
    let mut visited = HashSet::new();
    while let Some(pid) = queue.pop_front() {
        if !visited.insert(pid) || visited.len() > 128 {
            continue;
        }
        if visible.contains(&pid) {
            return true;
        }
        if let Some(kids) = children.get(&pid) {
            for &kid in kids {
                queue.push_back(kid);
            }
        }
    }
    false
}

fn has_ignored_ancestor(entry: &ProcEntry, by_pid: &HashMap<u32, &ProcEntry>) -> bool {
    let mut cur = entry.parent_pid;
    let mut depth = 0;
    while let Some(ppid) = cur {
        if depth >= 12 {
            break;
        }
        let Some(anc) = by_pid.get(&ppid) else {
            break;
        };
        if is_ignored_wrapper(&anc.name) {
            return true;
        }
        cur = anc.parent_pid;
        depth += 1;
    }
    false
}

fn has_same_desktop_ancestor(entry: &ProcEntry, by_pid: &HashMap<u32, &ProcEntry>) -> bool {
    let desktop_bin = matches!(
        entry.name.as_str(),
        "antigravity.exe"
            | "antigravity"
            | "chatgpt.exe"
            | "chatgpt"
            | "claude.exe"
            | "claude"
            | "opencode.exe"
            | "opencode"
            | "opencode-desktop"
            | "opencode-deskto"
    );
    if !desktop_bin {
        return false;
    }
    if matches!(entry.name.as_str(), "claude.exe" | "claude")
        && (entry.exe.contains("claude-code") || has_claude_cli_flags(&entry.cmd))
    {
        return false;
    }
    let mut cur = entry.parent_pid;
    let mut depth = 0;
    while let Some(ppid) = cur {
        if depth >= 12 {
            break;
        }
        let Some(anc) = by_pid.get(&ppid) else {
            break;
        };
        if anc.name == entry.name {
            return true;
        }
        cur = anc.parent_pid;
        depth += 1;
    }
    false
}

fn resolve_entries(entries: &[ProcEntry], visible_pids: Option<&HashSet<u32>>) -> ProcessMap {
    let mut by_pid = HashMap::<u32, &ProcEntry>::with_capacity(entries.len());
    let mut children = HashMap::<u32, Vec<u32>>::new();
    for entry in entries {
        by_pid.insert(entry.pid, entry);
        if let Some(ppid) = entry.parent_pid {
            children.entry(ppid).or_default().push(entry.pid);
        }
    }
    let enforce_windows = visible_pids.is_some_and(|set| !set.is_empty());
    let mut classified = Vec::<(&ProcEntry, &'static str, &'static str)>::new();
    let mut desktop_roots = HashMap::<u32, (&'static str, bool)>::new();
    let mut visible_desktop_buckets = HashSet::<&'static str>::new();

    for entry in entries {
        if has_ignored_ancestor(entry, &by_pid) || has_same_desktop_ancestor(entry, &by_pid) {
            continue;
        }
        let parent_name = entry
            .parent_pid
            .and_then(|pp| by_pid.get(&pp))
            .map(|p| p.name.as_str())
            .unwrap_or("");
        let class = classify(&entry.name, &entry.exe, &entry.cmd, parent_name).or_else(|| {
            if entry.name == "chatgpt"
                && parent_name != "chatgpt"
                && !electron_helper(&entry.cmd)
                && entry.codex_bundle
            {
                Some(("codex", "Codex Desktop"))
            } else {
                None
            }
        });
        let Some((bucket, label)) = class else {
            continue;
        };
        if is_desktop_label(label) {
            let visible = !enforce_windows
                || subtree_has_visible(entry.pid, &children, visible_pids.unwrap());
            desktop_roots.insert(entry.pid, (bucket, visible));
            if visible {
                visible_desktop_buckets.insert(bucket);
                classified.push((entry, bucket, label));
            }
        } else {
            classified.push((entry, bucket, label));
        }
    }

    let mut result = empty_map();
    for (entry, bucket, label) in classified {
        if !is_desktop_label(label) {
            let mut suppressed = false;
            let mut crossed_shell = false;
            let mut cur = entry.parent_pid;
            let mut depth = 0;
            while let Some(ppid) = cur {
                if depth >= 12 {
                    break;
                }
                let Some(anc) = by_pid.get(&ppid) else {
                    break;
                };
                if is_shell(&anc.name) {
                    crossed_shell = true;
                }
                if let Some(&(root_bucket, root_visible)) = desktop_roots.get(&anc.pid) {
                    if root_bucket == bucket {
                        if !root_visible || !crossed_shell {
                            suppressed = true;
                        }
                        break;
                    }
                }
                cur = anc.parent_pid;
                depth += 1;
            }
            if suppressed {
                continue;
            }
            if enforce_windows && !crossed_shell && !visible_desktop_buckets.contains(bucket) {
                let bundled_codex = bucket == "codex"
                    && (entry.exe.contains("windowsapps\\openai.codex")
                        || entry.exe.contains("/codex.app/contents/"));
                let bundled_claude = bucket == "claude"
                    && entry.exe.contains("claude-data")
                    && (entry.cmd.contains("--await-initialize")
                        || (entry.cmd.contains("--input-format")
                            && entry.cmd.contains("stream-json")));
                let bundled_ag_hub = bucket == "antigravity"
                    && label == "Antigravity Hub"
                    && entry.cmd.contains("--host_bridge_url");
                if bundled_codex || bundled_claude || bundled_ag_hub {
                    continue;
                }
            }
        }
        result
            .get_mut(bucket)
            .unwrap()
            .push(format!("{label} (PID {})", entry.pid));
    }

    for values in result.values_mut() {
        values.sort();
    }
    result
}

pub fn snapshot() -> ProcessMap {
    snapshot_force(false)
}

pub fn snapshot_force(force: bool) -> ProcessMap {
    static CACHE: OnceLock<Mutex<Option<(Instant, ProcessMap)>>> = OnceLock::new();
    let mut cache = CACHE
        .get_or_init(|| Mutex::new(None))
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    if !force {
        if let Some((at, data)) = &*cache {
            if at.elapsed() < Duration::from_millis(800) {
                return data.clone();
            }
        }
    }
    let mut system = System::new();
    system.refresh_processes_specifics(
        ProcessesToUpdate::All,
        true,
        ProcessRefreshKind::nothing()
            .with_cmd(UpdateKind::Always)
            .with_exe(UpdateKind::Always),
    );
    let self_pid = std::process::id();
    let mut entries = Vec::with_capacity(system.processes().len());
    for (pid, p) in system.processes() {
        let uid = pid.as_u32();
        if uid == self_pid {
            continue;
        }
        let name = p.name().to_string_lossy().to_lowercase();
        let exe = p
            .exe()
            .map(|v| v.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        let cmd = p
            .cmd()
            .iter()
            .map(|s| s.to_string_lossy())
            .collect::<Vec<_>>()
            .join(" ")
            .to_lowercase();
        let is_codex_bundle = name == "chatgpt" && p.exe().is_some_and(codex_bundle);
        entries.push(ProcEntry {
            pid: uid,
            parent_pid: p.parent().map(|pp| pp.as_u32()),
            name,
            exe,
            cmd,
            codex_bundle: is_codex_bundle,
        });
    }
    #[cfg(windows)]
    let visible = Some(win::visible_window_pids());
    #[cfg(not(windows))]
    let visible: Option<HashSet<u32>> = None;
    let result = resolve_entries(&entries, visible.as_ref());
    *cache = Some((Instant::now(), result.clone()));
    result
}

pub fn format_running(items: &[String]) -> String {
    let mut counts = BTreeMap::<&str, usize>::new();
    for item in items {
        *counts
            .entry(item.split(" (").next().unwrap_or(item))
            .or_default() += 1;
    }
    counts
        .iter()
        .map(|(k, n)| {
            if *n > 1 {
                format!("{k} x{n}")
            } else {
                k.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn is_matching_ag_hub(
    provider_id: &str,
    name: &str,
    exe: &str,
    cmd: &str,
    from_daemon_file: bool,
) -> bool {
    if !matches!(name, "language_server" | "language_server.exe") {
        return false;
    }
    let is_cli = exe.contains("antigravity-cli") || cmd.contains("antigravity-cli");
    match provider_id {
        "antigravity_cli" => is_cli || from_daemon_file,
        "antigravity" => {
            !is_cli
                && (from_daemon_file || exe.contains("antigravity") || cmd.contains("antigravity"))
        }
        _ => false,
    }
}

pub fn stop_antigravity_hubs(provider_id: &str, daemon_dir: &Path, scan_global: bool) -> Vec<u32> {
    if !matches!(provider_id, "antigravity" | "antigravity_cli") {
        return Vec::new();
    }
    let mut daemon_pids = HashSet::<u32>::new();
    if let Ok(entries) = fs::read_dir(daemon_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let Some(fname) = path.file_name().and_then(|s| s.to_str()) else {
                continue;
            };
            if !fname.starts_with("hub_") || !fname.ends_with(".json") {
                continue;
            }
            if let Ok(bytes) = fs::read(&path) {
                if let Ok(val) = serde_json::from_slice::<serde_json::Value>(&bytes) {
                    if let Some(pid) = val.get("pid").and_then(|v| v.as_u64()) {
                        if let Ok(uid) = u32::try_from(pid) {
                            daemon_pids.insert(uid);
                        }
                    }
                }
            }
        }
    }

    if !scan_global && daemon_pids.is_empty() {
        return Vec::new();
    }

    let mut system = System::new();
    system.refresh_processes_specifics(
        ProcessesToUpdate::All,
        true,
        ProcessRefreshKind::nothing()
            .with_cmd(UpdateKind::Always)
            .with_exe(UpdateKind::Always),
    );

    let self_pid = std::process::id();
    let mut signaled = BTreeSet::<u32>::new();
    for (pid, proc_info) in system.processes() {
        let uid = pid.as_u32();
        if uid == self_pid {
            continue;
        }
        let from_daemon = daemon_pids.contains(&uid);
        if !scan_global && !from_daemon {
            continue;
        }
        let name = proc_info.name().to_string_lossy().to_lowercase();
        let exe = proc_info
            .exe()
            .map(|v| v.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        let cmd = proc_info
            .cmd()
            .iter()
            .map(|s| s.to_string_lossy())
            .collect::<Vec<_>>()
            .join(" ")
            .to_lowercase();
        if is_matching_ag_hub(provider_id, &name, &exe, &cmd, from_daemon) {
            let sent = match proc_info.kill_with(Signal::Term) {
                Some(ok) => ok,
                None => proc_info.kill(),
            };
            if sent {
                signaled.insert(uid);
            }
        }
    }
    signaled.into_iter().collect()
}

pub fn any_pid_alive(pids: &[u32]) -> bool {
    if pids.is_empty() {
        return false;
    }
    let targets: Vec<Pid> = pids.iter().copied().map(Pid::from_u32).collect();
    let mut system = System::new();
    system.refresh_processes_specifics(
        ProcessesToUpdate::Some(&targets),
        true,
        ProcessRefreshKind::nothing(),
    );
    targets.iter().any(|pid| system.process(*pid).is_some())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn proc_entry(
        pid: u32,
        parent_pid: Option<u32>,
        name: &str,
        exe: &str,
        cmd: &str,
    ) -> ProcEntry {
        ProcEntry {
            pid,
            parent_pid,
            name: name.to_lowercase(),
            exe: exe.to_lowercase(),
            cmd: cmd.to_lowercase(),
            codex_bundle: false,
        }
    }

    #[test]
    fn detects_codex_markers_inside_legacy_chatgpt_bundle() {
        let temp = tempfile::tempdir().unwrap();
        let contents = temp.path().join("ChatGPT.app/Contents");
        let exe = contents.join("MacOS/ChatGPT");
        std::fs::create_dir_all(exe.parent().unwrap()).unwrap();
        assert!(
            !codex_bundle(&exe),
            "an ordinary bundle without metadata is not Codex"
        );
        std::fs::write(contents.join("Info.plist"),
            b"<?xml version=\"1.0\" encoding=\"UTF-8\"?><plist version=\"1.0\"><dict><key>CFBundleIdentifier</key><string>com.openai.chat</string></dict></plist>").unwrap();
        assert!(
            !codex_bundle(&exe),
            "ordinary ChatGPT must not block Codex history"
        );
        let framework = contents.join("Frameworks/Codex Framework.framework");
        std::fs::create_dir_all(&framework).unwrap();
        assert!(
            codex_bundle(&exe),
            "the framework marker overrides a generic bundle identifier"
        );
        std::fs::remove_dir(&framework).unwrap();
        let cli = contents.join("Resources/codex-cli");
        std::fs::create_dir_all(&cli).unwrap();
        std::fs::remove_file(contents.join("Info.plist")).unwrap();
        assert!(
            codex_bundle(&exe),
            "the CLI marker works without Info.plist"
        );
        let other = temp.path().join("Unrelated.app/Contents");
        std::fs::create_dir_all(other.join("Resources/codex-cli")).unwrap();
        assert!(
            !codex_bundle(&other.join("MacOS/ChatGPT")),
            "the bundle path guard is retained"
        );
        assert!(electron_helper("--type=renderer"));
        assert!(electron_helper("/prefetch:4 --monitor-self"));
        assert!(electron_helper("app.exe --hidden"));
        assert_eq!(
            classify(
                "chatgpt",
                "/Applications/Codex.app/Contents/MacOS/ChatGPT",
                "--type=renderer",
                ""
            ),
            None
        );
    }

    #[test]
    fn distinguishes_desktop_cli_helpers_and_tunnels() {
        assert_eq!(
            classify("antigravity", "/opt/antigravity", "--type=renderer", ""),
            None
        );
        assert_eq!(
            classify("codex", "/bin/codex", "app-server", "tunnel-client"),
            None
        );
        assert_eq!(
            classify("codex", "/bin/codex", "exec-server", ""),
            Some(("codex", "Codex Exec Server"))
        );
        assert_eq!(
            classify(
                "language_server",
                "/antigravity-cli/language_server",
                "",
                ""
            ),
            Some(("antigravity_cli", "Antigravity CLI Hub"))
        );
        assert_eq!(
            classify(
                "node",
                "/bin/node",
                "/lib/@anthropic-ai/claude-code/cli.js",
                ""
            ),
            Some(("claude", "Claude Code"))
        );
        assert_eq!(
            classify(
                "node.exe",
                "c:\\program files\\nodejs\\node.exe",
                "node.exe c:\\projects\\opencode\\server.js",
                "cmd.exe"
            ),
            None
        );
        assert_eq!(
            classify(
                "node.exe",
                "c:\\program files\\nodejs\\node.exe",
                "node.exe c:\\users\\u\\appdata\\roaming\\npm\\node_modules\\opencode-ai\\bin\\opencode.js",
                "cmd.exe"
            ),
            Some(("opencode", "OpenCode CLI"))
        );
        assert_eq!(classify("claude.exe", "", "", "claude.exe"), None);
        assert_eq!(
            classify("claude.exe", "", "", "explorer.exe"),
            Some(("claude", "Claude Desktop"))
        );
        assert_eq!(
            classify("claude.exe", "", "", "powershell.exe"),
            Some(("claude", "Claude Code"))
        );
    }

    #[test]
    fn suppresses_hidden_tray_and_autostart_windows_apps_with_children() {
        let entries = vec![
            proc_entry(100, None, "explorer.exe", "c:\\windows\\explorer.exe", ""),
            proc_entry(101, None, "sihost.exe", "c:\\windows\\system32\\sihost.exe", ""),
            proc_entry(
                200,
                Some(101),
                "ChatGPT.exe",
                "c:\\program files\\windowsapps\\openai.codex_26\\app\\chatgpt.exe",
                "\"c:\\program files\\windowsapps\\openai.codex_26\\app\\chatgpt.exe\"",
            ),
            proc_entry(
                201,
                Some(200),
                "ChatGPT.exe",
                "c:\\program files\\windowsapps\\openai.codex_26\\app\\chatgpt.exe",
                "",
            ),
            proc_entry(
                202,
                Some(201),
                "codex.exe",
                "c:\\program files\\windowsapps\\openai.codex_26\\app\\resources\\codex.exe",
                "codex.exe -c features.code_mode_host=true app-server",
            ),
            proc_entry(
                203,
                Some(200),
                "codex.exe",
                "c:\\program files\\windowsapps\\openai.codex_26\\app\\resources\\codex.exe",
                "codex.exe exec-server --remote https://codex.example",
            ),
            proc_entry(
                300,
                Some(100),
                "Claude.exe",
                "c:\\program files\\windowsapps\\claude_6\\app\\claude.exe",
                "\"c:\\program files\\windowsapps\\claude_6\\app\\claude.exe\"",
            ),
            proc_entry(
                301,
                Some(300),
                "Claude.exe",
                "",
                "",
            ),
            proc_entry(
                302,
                Some(300),
                "claude.exe",
                "c:\\users\\u\\appdata\\local\\claude-data\\claude_x\\claude-code\\2.1\\claude.exe",
                "claude.exe --output-format stream-json --input-format stream-json --await-initialize",
            ),
            proc_entry(
                400,
                Some(100),
                "Antigravity.exe",
                "c:\\users\\u\\appdata\\local\\programs\\antigravity\\antigravity.exe",
                "\"antigravity.exe\"",
            ),
            proc_entry(
                401,
                Some(400),
                "language_server.exe",
                "c:\\users\\u\\appdata\\local\\programs\\antigravity\\resources\\bin\\language_server.exe",
                "language_server.exe --standalone --override_ide_name antigravity --host_bridge_url=http://127.0.0.1:9999",
            ),
            proc_entry(
                500,
                Some(100),
                "Codex Web GPT.exe",
                "c:\\users\\u\\appdata\\local\\programs\\codex web gpt\\codex web gpt.exe",
                "\"codex web gpt.exe\" --hidden",
            ),
            proc_entry(501, Some(500), "tunnel-client.exe", "c:\\tunnel-client.exe", ""),
            proc_entry(502, Some(501), "cmd.exe", "c:\\windows\\system32\\cmd.exe", ""),
            proc_entry(503, Some(502), "codex.exe", "c:\\bin\\codex.exe", "codex app-server"),
        ];
        let visible = HashSet::from([100]);
        let map = resolve_entries(&entries, Some(&visible));
        for (bucket, items) in &map {
            assert!(
                items.is_empty(),
                "expected no false positives for {bucket}, got {items:?}"
            );
        }
    }

    #[test]
    fn detects_visible_desktop_apps_and_standalone_terminal_clis() {
        let entries = vec![
            proc_entry(100, None, "explorer.exe", "c:\\windows\\explorer.exe", ""),
            proc_entry(
                150,
                Some(100),
                "powershell.exe",
                "c:\\windows\\powershell.exe",
                "",
            ),
            proc_entry(
                200,
                Some(100),
                "ChatGPT.exe",
                "c:\\program files\\windowsapps\\openai.codex_26\\app\\chatgpt.exe",
                "\"chatgpt.exe\"",
            ),
            proc_entry(
                201,
                Some(200),
                "ChatGPT.exe",
                "c:\\program files\\windowsapps\\openai.codex_26\\app\\chatgpt.exe",
                "--type=utility",
            ),
            proc_entry(
                202,
                Some(201),
                "codex.exe",
                "c:\\program files\\windowsapps\\openai.codex_26\\app\\resources\\codex.exe",
                "codex.exe app-server",
            ),
            proc_entry(
                300,
                Some(150),
                "claude.exe",
                "c:\\users\\u\\.local\\bin\\claude.exe",
                "claude.exe",
            ),
            proc_entry(
                400,
                Some(150),
                "codex.exe",
                "c:\\users\\u\\appdata\\local\\programs\\openai\\codex\\bin\\codex.exe",
                "codex.exe",
            ),
        ];
        let visible = HashSet::from([100, 150, 200]);
        let map = resolve_entries(&entries, Some(&visible));
        assert_eq!(
            map["codex"],
            vec!["Codex CLI (PID 400)", "Codex Desktop (PID 200)"]
        );
        assert_eq!(map["claude"], vec!["Claude Code (PID 300)"]);
        assert!(map["antigravity"].is_empty());
        assert!(map["opencode"].is_empty());
    }

    #[test]
    fn suppresses_orphaned_bundled_desktop_servers_without_visible_window() {
        let entries = vec![
            proc_entry(100, None, "explorer.exe", "c:\\windows\\explorer.exe", ""),
            proc_entry(
                202,
                None,
                "codex.exe",
                "c:\\program files\\windowsapps\\openai.codex_26\\app\\resources\\codex.exe",
                "codex.exe app-server",
            ),
            proc_entry(
                302,
                None,
                "claude.exe",
                "c:\\users\\u\\appdata\\local\\claude-data\\claude_x\\claude-code\\2.1\\claude.exe",
                "claude.exe --output-format stream-json --input-format stream-json --await-initialize",
            ),
            proc_entry(
                401,
                None,
                "language_server.exe",
                "c:\\users\\u\\appdata\\local\\programs\\antigravity\\resources\\bin\\language_server.exe",
                "language_server.exe --standalone --override_ide_name antigravity --host_bridge_url=http://127.0.0.1:9999",
            ),
        ];
        let visible = HashSet::from([100]);
        let map = resolve_entries(&entries, Some(&visible));
        assert!(map["codex"].is_empty());
        assert!(map["claude"].is_empty());
        assert!(map["antigravity"].is_empty());
    }

    #[test]
    fn matches_only_target_antigravity_hub_processes() {
        assert!(is_matching_ag_hub(
            "antigravity",
            "language_server",
            "/applications/antigravity.app/contents/resources/bin/language_server",
            "language_server --standalone",
            false
        ));
        assert!(!is_matching_ag_hub(
            "antigravity",
            "antigravity",
            "/applications/antigravity.app/contents/macos/antigravity",
            "/applications/antigravity.app/contents/macos/antigravity",
            true
        ));
        assert!(!is_matching_ag_hub(
            "antigravity",
            "language_server",
            "/users/eth/.gemini/antigravity-cli/bin/language_server",
            "language_server --app_data_dir antigravity-cli",
            true
        ));
        assert!(is_matching_ag_hub(
            "antigravity_cli",
            "language_server",
            "/users/eth/.gemini/antigravity-cli/bin/language_server",
            "language_server --app_data_dir antigravity-cli",
            false
        ));
        assert!(!is_matching_ag_hub(
            "codex",
            "language_server",
            "/applications/antigravity.app/contents/resources/bin/language_server",
            "language_server",
            true
        ));
    }
}
