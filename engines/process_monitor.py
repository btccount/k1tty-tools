from __future__ import annotations

import json
import os
import subprocess
import threading
import time
from pathlib import Path

try:
    import psutil
except ImportError:
    psutil = None

_LOCK = threading.Lock()
_CACHE_TS: float = 0.0
_CACHE_TTL: float = 0.8
_CACHE_DATA: dict[str, list[str]] = {
    "antigravity": [],
    "antigravity_cli": [],
    "codex": [],
    "claude": [],
    "opencode": [],
}
_PID_CLASS_CACHE: dict[int, tuple[str, float, str | None, str]] = {}
_CODEX_BUNDLE_CACHE: dict[str, bool] = {}
_SELF_PID = os.getpid()

_HELPER_FLAGS = (
    "--type=renderer",
    "--type=gpu-process",
    "--type=utility",
    "--type=crashpad-handler",
    "--type=zygote",
    "--type=ppapi",
    "--type=broker",
    "--type=sandbox",
)

_CANDIDATE_NAMES = frozenset({
    "antigravity.exe",
    "antigravity",
    "agy.exe",
    "agy",
    "antigravity-cli.exe",
    "antigravity-cli",
    "language_server.exe",
    "language_server",
    "codex.exe",
    "codex",
    "chatgpt.exe",
    "chatgpt",
    "claude.exe",
    "claude",
    "opencode.exe",
    "opencode",
    "opencode-cli.exe",
    "opencode-cli",
    "opencode-desktop",
    "opencode-deskto",
    "node.exe",
    "node",
})

_CLAUDE_CLI_FLAGS = (
    "--output-format",
    "--input-format",
    "--permission-prompt-tool",
    "--resume",
    "--print",
)

if os.name == "nt":
    import ctypes
    from ctypes import wintypes

    _TH32CS_SNAPPROCESS = 0x00000002
    _INVALID_HANDLE_VALUE = ctypes.c_void_p(-1).value
    _PROCESS_QUERY_LIMITED_INFORMATION = 0x1000

    class _PROCESSENTRY32W(ctypes.Structure):
        _fields_ = [
            ("dwSize", wintypes.DWORD),
            ("cntUsage", wintypes.DWORD),
            ("th32ProcessID", wintypes.DWORD),
            ("th32DefaultHeapID", ctypes.c_size_t),
            ("th32ModuleID", wintypes.DWORD),
            ("cntThreads", wintypes.DWORD),
            ("th32ParentProcessID", wintypes.DWORD),
            ("pcPriClassBase", wintypes.LONG),
            ("dwFlags", wintypes.DWORD),
            ("szExeFile", wintypes.WCHAR * 260),
        ]

    class _FILETIME(ctypes.Structure):
        _fields_ = [
            ("dwLowDateTime", wintypes.DWORD),
            ("dwHighDateTime", wintypes.DWORD),
        ]

    _k32 = ctypes.WinDLL("kernel32", use_last_error=True)
    _k32.CreateToolhelp32Snapshot.argtypes = [wintypes.DWORD, wintypes.DWORD]
    _k32.CreateToolhelp32Snapshot.restype = wintypes.HANDLE
    _k32.Process32FirstW.argtypes = [wintypes.HANDLE, ctypes.POINTER(_PROCESSENTRY32W)]
    _k32.Process32FirstW.restype = wintypes.BOOL
    _k32.Process32NextW.argtypes = [wintypes.HANDLE, ctypes.POINTER(_PROCESSENTRY32W)]
    _k32.Process32NextW.restype = wintypes.BOOL
    _k32.CloseHandle.argtypes = [wintypes.HANDLE]
    _k32.CloseHandle.restype = wintypes.BOOL
    _k32.OpenProcess.argtypes = [wintypes.DWORD, wintypes.BOOL, wintypes.DWORD]
    _k32.OpenProcess.restype = wintypes.HANDLE
    _k32.GetProcessTimes.argtypes = [
        wintypes.HANDLE,
        ctypes.POINTER(_FILETIME),
        ctypes.POINTER(_FILETIME),
        ctypes.POINTER(_FILETIME),
        ctypes.POINTER(_FILETIME),
    ]
    _k32.GetProcessTimes.restype = wintypes.BOOL
else:
    _k32 = None


def _win32_candidates() -> list[tuple[int, str]] | None:
    if _k32 is None:
        return None
    snap = _k32.CreateToolhelp32Snapshot(_TH32CS_SNAPPROCESS, 0)
    if not snap or snap == _INVALID_HANDLE_VALUE:
        return None
    out: list[tuple[int, str]] = []
    try:
        pe = _PROCESSENTRY32W()
        pe.dwSize = ctypes.sizeof(_PROCESSENTRY32W)
        if _k32.Process32FirstW(snap, ctypes.byref(pe)):
            while True:
                pid = int(pe.th32ProcessID)
                if pid and pid != _SELF_PID:
                    low = pe.szExeFile.lower()
                    if low in _CANDIDATE_NAMES:
                        out.append((pid, low))
                if not _k32.Process32NextW(snap, ctypes.byref(pe)):
                    break
    finally:
        _k32.CloseHandle(snap)
    return out


def _win32_create_time(pid: int) -> float:
    if _k32 is None:
        return 0.0
    h = _k32.OpenProcess(_PROCESS_QUERY_LIMITED_INFORMATION, False, pid)
    if not h:
        return 0.0
    try:
        ct, et, kt, ut = _FILETIME(), _FILETIME(), _FILETIME(), _FILETIME()
        if _k32.GetProcessTimes(h, ctypes.byref(ct), ctypes.byref(et), ctypes.byref(kt), ctypes.byref(ut)):
            return float((ct.dwHighDateTime << 32) | ct.dwLowDateTime)
        return 0.0
    finally:
        _k32.CloseHandle(h)


def _is_electron_helper(cmd_low: str) -> bool:
    return any(flag in cmd_low for flag in _HELPER_FLAGS)


def _is_macos_codex_bundle(exe_low: str, exe_orig: str = "") -> bool:
    if "/chatgpt.app/contents/macos/" not in exe_low:
        return False
    cached = _CODEX_BUNDLE_CACHE.get(exe_low)
    if cached is not None:
        return cached
    found = False
    try:
        contents = Path(exe_orig or exe_low).parent.parent
        found = (
            (contents / "Frameworks" / "Codex Framework.framework").is_dir()
            or (contents / "Resources" / "codex-cli").is_dir()
        )
    except Exception:
        found = False
    _CODEX_BUNDLE_CACHE[exe_low] = found
    return found


def _proc_details(proc) -> tuple[str, str, str, str]:
    exe = ""
    cmd = ""
    parent_low = ""
    try:
        exe = proc.exe() or ""
    except Exception:
        pass
    try:
        parts = proc.cmdline() or []
        cmd = " ".join(parts)
    except Exception:
        pass
    try:
        par = proc.parent()
        if par is not None:
            parent_low = (par.name() or "").lower()
    except Exception:
        pass
    return exe.lower(), cmd.lower(), parent_low, exe


def _posix_ps_snapshot() -> dict[int, tuple[str, str, str, str]]:
    """Return {pid: (low_name, exe_orig, cmd_low, parent_low)} using POSIX ps."""
    rows: dict[int, tuple[int, str, str]] = {}
    try:
        out = subprocess.run(
            ["ps", "-axo", "pid=,ppid=,comm="],
            capture_output=True, text=True, errors="replace", timeout=8,
        ).stdout
    except Exception:
        return {}
    for line in out.splitlines():
        parts = line.strip().split(None, 2)
        if len(parts) < 3:
            continue
        try:
            pid = int(parts[0])
            ppid = int(parts[1])
        except ValueError:
            continue
        comm = parts[2].strip()
        rows[pid] = (ppid, comm, Path(comm).name.lower())

    cand_pids = [
        pid for pid, (_ppid, _comm, low) in rows.items()
        if pid != _SELF_PID and low in _CANDIDATE_NAMES
    ]
    cmds: dict[int, str] = {}
    if cand_pids:
        try:
            out_args = subprocess.run(
                ["ps", "-p", ",".join(str(p) for p in cand_pids), "-o", "pid=,args="],
                capture_output=True, text=True, errors="replace", timeout=8,
            ).stdout
            for line in out_args.splitlines():
                parts = line.strip().split(None, 1)
                if len(parts) == 2:
                    try:
                        cmds[int(parts[0])] = parts[1].strip().lower()
                    except ValueError:
                        pass
        except Exception:
            pass

    out_map: dict[int, tuple[str, str, str, str]] = {}
    for pid in cand_pids:
        ppid, comm, low = rows[pid]
        parent_low = rows[ppid][2] if ppid in rows else ""
        out_map[pid] = (low, comm, cmds.get(pid, ""), parent_low)
    return out_map


def _classify(
    pid: int,
    low: str,
    exe_low: str,
    cmd_low: str,
    parent_low: str = "",
    exe_orig: str = "",
) -> tuple[str | None, str]:
    if low in ("agy.exe", "agy", "antigravity-cli.exe", "antigravity-cli"):
        return "antigravity_cli", f"Antigravity CLI (PID {pid})"

    if low in ("antigravity.exe", "antigravity"):
        if _is_electron_helper(cmd_low) or "helper" in exe_low or "sidecar_sdk" in cmd_low or "--import=" in cmd_low:
            return None, ""
        return "antigravity", f"Antigravity Desktop (PID {pid})"

    if low in ("language_server.exe", "language_server"):
        if "antigravity-cli" in exe_low or "antigravity-cli" in cmd_low:
            return "antigravity_cli", f"Antigravity CLI Hub (PID {pid})"
        if "antigravity" in exe_low or "antigravity" in cmd_low:
            return "antigravity", f"Antigravity Hub (PID {pid})"
        return None, ""

    if low in ("chatgpt.exe", "chatgpt"):
        if _is_electron_helper(cmd_low):
            return None, ""
        if "openai.codex" in exe_low or _is_macos_codex_bundle(exe_low, exe_orig):
            return "codex", f"Codex Desktop (PID {pid})"
        return None, ""

    if low in ("codex.exe", "codex"):
        if _is_electron_helper(cmd_low):
            return None, ""
        if " sandbox " in f" {cmd_low} " or cmd_low.endswith(" sandbox"):
            return None, ""
        if "app-server" in cmd_low:
            if parent_low in ("tunnel-client.exe", "tunnel-client"):
                return None, ""
            return "codex", f"Codex App Server (PID {pid})"
        if "exec-server" in cmd_low:
            if parent_low in ("tunnel-client.exe", "tunnel-client"):
                return None, ""
            return "codex", f"Codex Exec Server (PID {pid})"
        if "/codex.app/contents/macos/" in exe_low:
            return "codex", f"Codex Desktop (PID {pid})"
        return "codex", f"Codex CLI (PID {pid})"

    if low in ("claude.exe", "claude"):
        if _is_electron_helper(cmd_low):
            return None, ""
        is_desktop = (
            (
                "windowsapps\\claude_" in exe_low
                or "anthropicclaude" in exe_low
                or "/claude.app/contents/macos/" in exe_low
            )
            and "claude-code" not in exe_low
            and not any(f in cmd_low for f in _CLAUDE_CLI_FLAGS)
        )
        if is_desktop:
            return "claude", f"Claude Desktop (PID {pid})"
        return "claude", f"Claude Code (PID {pid})"

    if low in ("opencode.exe", "opencode", "opencode-desktop", "opencode-deskto"):
        if _is_electron_helper(cmd_low):
            return None, ""
        if (
            "ai.opencode.desktop" in exe_low
            or "/opencode.app/contents/macos/" in exe_low
            or low in ("opencode-desktop", "opencode-deskto")
            or ("opencode.exe" in exe_low and "cli" not in exe_low)
        ):
            return "opencode", f"OpenCode Desktop (PID {pid})"
        return "opencode", f"OpenCode CLI (PID {pid})"

    if low in ("opencode-cli.exe", "opencode-cli"):
        if "--service" in cmd_low:
            return None, ""
        if "serve" in cmd_low:
            return "opencode", f"OpenCode Server (PID {pid})"
        return "opencode", f"OpenCode CLI (PID {pid})"

    if low in ("node.exe", "node"):
        if "@anthropic-ai\\claude-code" in cmd_low or "@anthropic-ai/claude-code" in cmd_low:
            return "claude", f"Claude Code (PID {pid})"
        if ("@openai\\codex" in cmd_low or "@openai/codex" in cmd_low) and "computer-use" not in cmd_low:
            return "codex", f"Codex CLI (PID {pid})"
        if "opencode-ai" in cmd_low or "\\opencode\\" in cmd_low or "/opencode/" in cmd_low:
            return "opencode", f"OpenCode CLI (PID {pid})"

    return None, ""


def format_running(items: list[str]) -> str:
    counts: dict[str, int] = {}
    for entry in items:
        label = entry.split(" (", 1)[0]
        counts[label] = counts.get(label, 0) + 1
    return ", ".join(f"{k} x{v}" if v > 1 else k for k, v in counts.items())


def snapshot_processes(ag_daemon_dir: Path | None = None, force: bool = False) -> dict[str, list[str]]:
    global _CACHE_TS, _CACHE_DATA
    with _LOCK:
        now = time.monotonic()
        if not force and (now - _CACHE_TS) < _CACHE_TTL:
            return {k: list(v) for k, v in _CACHE_DATA.items()}

        buckets: dict[str, list[str]] = {
            "antigravity": [],
            "antigravity_cli": [],
            "codex": [],
            "claude": [],
            "opencode": [],
        }
        seen_by_bucket: dict[str, set[int]] = {k: set() for k in buckets}

        candidates = _win32_candidates()
        posix_snap: dict[int, tuple[str, str, str, str]] | None = None
        if candidates is None and psutil is not None:
            candidates = []
            for proc in psutil.process_iter(attrs=["pid", "name"]):
                try:
                    pid = proc.info["pid"]
                    if pid == _SELF_PID:
                        continue
                    low = (proc.info.get("name") or "").lower()
                    if low in _CANDIDATE_NAMES:
                        candidates.append((pid, low))
                except Exception:
                    continue
        elif candidates is None and os.name != "nt":
            posix_snap = _posix_ps_snapshot()
            candidates = [(pid, info[0]) for pid, info in posix_snap.items()]

        if candidates is None:
            candidates = []

        alive_pids: set[int] = set()
        for pid, low in candidates:
            alive_pids.add(pid)
            cached = _PID_CLASS_CACHE.get(pid)
            if cached is not None and cached[0] == low:
                ctime = _win32_create_time(pid) if os.name == "nt" else cached[1]
                if ctime == 0.0 or cached[1] == 0.0 or ctime == cached[1]:
                    bname, label = cached[2], cached[3]
                    if bname and pid not in seen_by_bucket[bname]:
                        seen_by_bucket[bname].add(pid)
                        buckets[bname].append(label)
                    continue

            exe_low, cmd_low, parent_low, exe_orig = "", "", "", ""
            ctime = _win32_create_time(pid) if os.name == "nt" else 0.0
            if psutil is not None:
                try:
                    proc = psutil.Process(pid)
                    if ctime == 0.0:
                        try:
                            ctime = float(proc.create_time())
                        except Exception:
                            ctime = 0.0
                    exe_low, cmd_low, parent_low, exe_orig = _proc_details(proc)
                except Exception:
                    continue
            elif posix_snap is not None and pid in posix_snap:
                _low_n, exe_orig, cmd_low, parent_low = posix_snap[pid]
                exe_low = exe_orig.lower()
            bname, label = _classify(pid, low, exe_low, cmd_low, parent_low, exe_orig=exe_orig)
            _PID_CLASS_CACHE[pid] = (low, ctime, bname, label)
            if bname and pid not in seen_by_bucket[bname]:
                seen_by_bucket[bname].add(pid)
                buckets[bname].append(label)

        for stale in list(_PID_CLASS_CACHE.keys()):
            if stale not in alive_pids:
                _PID_CLASS_CACHE.pop(stale, None)

        if ag_daemon_dir and ag_daemon_dir.exists():
            for hub in ag_daemon_dir.glob("hub_*.json"):
                try:
                    info = json.loads(hub.read_text(encoding="utf-8"))
                    pid = int(info.get("pid", 0))
                    if not pid or pid == _SELF_PID or pid in seen_by_bucket["antigravity"] or pid in seen_by_bucket["antigravity_cli"]:
                        continue
                    low_name = ""
                    exe_low, cmd_low = "", ""
                    if psutil is not None:
                        if not psutil.pid_exists(pid):
                            continue
                        p = psutil.Process(pid)
                        low_name = (p.name() or "").lower()
                        if low_name in ("language_server.exe", "language_server", "antigravity.exe", "antigravity"):
                            exe_low, cmd_low, _parent_low, _exe_orig = _proc_details(p)
                    elif os.name != "nt":
                        out_h = subprocess.run(
                            ["ps", "-p", str(pid), "-o", "comm="],
                            capture_output=True, text=True, errors="replace", timeout=5,
                        ).stdout.strip()
                        if not out_h:
                            continue
                        low_name = Path(out_h).name.lower()
                        if low_name in ("language_server", "antigravity"):
                            exe_low = out_h.lower()
                            cmd_low = subprocess.run(
                                ["ps", "-p", str(pid), "-o", "args="],
                                capture_output=True, text=True, errors="replace", timeout=5,
                            ).stdout.strip().lower()
                    if low_name in ("language_server.exe", "language_server", "antigravity.exe", "antigravity"):
                        if "antigravity-cli" in exe_low or "antigravity-cli" in cmd_low:
                            seen_by_bucket["antigravity_cli"].add(pid)
                            buckets["antigravity_cli"].append(f"Antigravity CLI Hub (PID {pid})")
                        elif "antigravity" in exe_low or "antigravity" in cmd_low:
                            seen_by_bucket["antigravity"].add(pid)
                            buckets["antigravity"].append(f"Antigravity Hub (PID {pid})")
                except Exception:
                    continue

        _CACHE_DATA = buckets
        _CACHE_TS = now
        return {k: list(v) for k, v in _CACHE_DATA.items()}
