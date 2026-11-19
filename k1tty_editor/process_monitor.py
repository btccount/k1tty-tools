from __future__ import annotations

import ctypes
from ctypes import wintypes
import json
import os
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
    "claude.exe",
    "claude",
    "opencode.exe",
    "opencode",
    "opencode-cli.exe",
    "opencode-cli",
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


if os.name == "nt":
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


def _proc_details(proc) -> tuple[str, str, str]:
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
    return exe.lower(), cmd.lower(), parent_low


def _classify(pid: int, low: str, exe_low: str, cmd_low: str, parent_low: str = "") -> tuple[str | None, str]:
    if low in ("agy.exe", "agy", "antigravity-cli.exe", "antigravity-cli"):
        return "antigravity_cli", f"Antigravity CLI (PID {pid})"

    if low in ("antigravity.exe", "antigravity"):
        if _is_electron_helper(cmd_low) or "sidecar_sdk" in cmd_low or "--import=" in cmd_low:
            return None, ""
        return "antigravity", f"Antigravity Desktop (PID {pid})"

    if low in ("language_server.exe", "language_server"):
        if "antigravity-cli" in exe_low or "antigravity-cli" in cmd_low:
            return "antigravity_cli", f"Antigravity CLI Hub (PID {pid})"
        if "antigravity" in exe_low or "antigravity" in cmd_low:
            return "antigravity", f"Antigravity Hub (PID {pid})"
        return None, ""

    if low == "chatgpt.exe":
        if "openai.codex" in exe_low and not _is_electron_helper(cmd_low):
            return "codex", f"Codex Desktop (PID {pid})"
        return None, ""

    if low in ("codex.exe", "codex"):
        if "app-server" in cmd_low:
            if parent_low in ("tunnel-client.exe", "tunnel-client"):
                return None, ""
            return "codex", f"Codex App Server (PID {pid})"
        if "exec-server" in cmd_low:
            if parent_low in ("tunnel-client.exe", "tunnel-client"):
                return None, ""
            return "codex", f"Codex Exec Server (PID {pid})"
        return "codex", f"Codex CLI (PID {pid})"

    if low in ("claude.exe", "claude"):
        if _is_electron_helper(cmd_low):
            return None, ""
        is_desktop = (
            ("windowsapps\\claude_" in exe_low or "anthropicclaude" in exe_low)
            and "claude-code" not in exe_low
            and not any(f in cmd_low for f in _CLAUDE_CLI_FLAGS)
        )
        if is_desktop:
            return "claude", f"Claude Desktop (PID {pid})"
        return "claude", f"Claude Code (PID {pid})"

    if low in ("opencode.exe", "opencode"):
        if _is_electron_helper(cmd_low):
            return None, ""
        if "ai.opencode.desktop" in exe_low or ("opencode.exe" in exe_low and "cli" not in exe_low):
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

            exe_low, cmd_low, parent_low = "", "", ""
            ctime = _win32_create_time(pid) if os.name == "nt" else 0.0
            if psutil is not None:
                try:
                    proc = psutil.Process(pid)
                    if ctime == 0.0:
                        try:
                            ctime = float(proc.create_time())
                        except Exception:
                            ctime = 0.0
                    exe_low, cmd_low, parent_low = _proc_details(proc)
                except Exception:
                    continue
            bname, label = _classify(pid, low, exe_low, cmd_low, parent_low)
            _PID_CLASS_CACHE[pid] = (low, ctime, bname, label)
            if bname and pid not in seen_by_bucket[bname]:
                seen_by_bucket[bname].add(pid)
                buckets[bname].append(label)

        for stale in list(_PID_CLASS_CACHE.keys()):
            if stale not in alive_pids:
                _PID_CLASS_CACHE.pop(stale, None)

        if psutil is not None and ag_daemon_dir and ag_daemon_dir.exists():
            for hub in ag_daemon_dir.glob("hub_*.json"):
                try:
                    info = json.loads(hub.read_text(encoding="utf-8"))
                    pid = int(info.get("pid", 0))
                    if not pid or pid == _SELF_PID or pid in seen_by_bucket["antigravity"] or pid in seen_by_bucket["antigravity_cli"]:
                        continue
                    if not psutil.pid_exists(pid):
                        continue
                    p = psutil.Process(pid)
                    low_name = (p.name() or "").lower()
                    if low_name in ("language_server.exe", "language_server", "antigravity.exe", "antigravity"):
                        exe_low, cmd_low, _parent_low = _proc_details(p)
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
