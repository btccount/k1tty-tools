from __future__ import annotations

import json
import os
import re
import shutil
import sqlite3
from datetime import datetime, timezone
from pathlib import Path

def _home() -> Path:
    return Path(os.environ.get("CODEX_HOME") or (Path.home() / ".codex"))

CODEX_HOME = _home()
SESSIONS = CODEX_HOME / "sessions"
PROJECTION_DB = CODEX_HOME / "thread_history_1.sqlite"
HISTORY_JSONL = CODEX_HOME / "history.jsonl"
SESSION_INDEX = CODEX_HOME / "session_index.jsonl"

AUTO_INJECT_RE = re.compile(r"^\s*<(environment_context|app-context|user_instructions)")

def db_write_probe(db_path) -> tuple[bool, str]:
    try:
        con = sqlite3.connect(str(db_path), timeout=2.0)
        try:
            con.execute("BEGIN IMMEDIATE")
            con.execute("ROLLBACK")
        finally:
            con.close()
        return True, ""
    except Exception as e:
        return False, str(e)


def codex_running() -> list:
    running = []
    try:
        import subprocess
        if os.name != "nt":
            out = subprocess.run(
                ["ps", "-axo", "pid=,comm="],
                capture_output=True, text=True, errors="replace", timeout=15,
            ).stdout
            for line in out.splitlines():
                parts = line.strip().split(None, 1)
                if len(parts) < 2:
                    continue
                pid_s, comm = parts[0], parts[1].strip()
                low = comm.lower()
                if "codex" in low or "/chatgpt.app/contents/macos/chatgpt" in low:
                    running.append(f"{Path(comm).name} (PID {pid_s})")
            return running
        out = subprocess.run(["tasklist", "/FO", "CSV", "/NH"], capture_output=True,
                             text=True, errors="replace", timeout=20).stdout
        seen = set()
        for line in out.splitlines():
            low = line.lower()
            if "codex" in low and '"' in line:
                parts = [p.strip('"') for p in line.split('","')]
                if parts and parts[0] not in seen:
                    seen.add(parts[0])
                    running.append(f"{parts[0]} (PID {parts[1] if len(parts) > 1 else '?'})")
    except Exception:
        pass
    return running


def require_writable():
    if CODEX_HOME.resolve().name.lower() not in ("codex_sandbox", "sandbox"):
        raise PermissionError("Правка настоящей истории пока отключена: восстановление проекции в Codex не проверено. Используйте только песочницу.")
    ok, msg = db_write_probe(PROJECTION_DB)
    if not ok:
        raise RuntimeError(
            f"Файлы Codex прямо сейчас заняты другим процессом (Codex запущен?): {msg}. "
            "Закройте Codex и повторите.")
    return codex_running()


_FILE_CACHE: dict[str, tuple[int, str]] = {}
_LIST_CACHE: dict = {}


def _file_meta(path: Path) -> tuple[str, str]:
    with open(path, "rb") as fh:
        first = fh.readline(200_000)
    o = json.loads(first.decode("utf-8", "replace"))
    pl = o.get("payload") or {}
    source = pl.get("source")
    if source == "subagent" or (isinstance(source, dict) and "subagent" in source):
        return "", o.get("timestamp", "")
    tid = pl.get("session_id") or pl.get("parent_thread_id") or pl.get("id") or ""
    return tid, o.get("timestamp", "")


def thread_files(tid: str) -> list[Path]:
    out = []
    for path in sorted(SESSIONS.rglob("rollout-*.jsonl")):
        s = str(path)
        m = _FILE_CACHE.get(s)
        if m is None or m[0] != path.stat().st_mtime_ns:
            t, _ts = _file_meta(path)
            m = (path.stat().st_mtime_ns, t)
            _FILE_CACHE[s] = m
        if m[1] == tid:
            out.append(path)
    return out


def _titles() -> dict:
    if _LIST_CACHE.get("titles"):
        return _LIST_CACHE["titles"]
    titles = {}
    if SESSION_INDEX.exists():
        for line in SESSION_INDEX.read_text(encoding="utf-8", errors="replace").splitlines():
            try:
                o = json.loads(line)
                titles[o.get("id", "")] = o.get("thread_name") or ""
            except Exception:
                continue
    _LIST_CACHE["titles"] = titles
    return titles


def list_threads() -> list:
    groups: dict[str, dict] = {}
    for path in sorted(SESSIONS.rglob("rollout-*.jsonl")):
        s = str(path)
        st = path.stat()
        key_hit = _FILE_CACHE.get(s)
        if key_hit is None or key_hit[0] != st.st_mtime_ns:
            tid, _ts = _file_meta(path)
            _FILE_CACHE[s] = (st.st_mtime_ns, tid)
            tid = _FILE_CACHE[s][1]
        else:
            tid = key_hit[1]
        if not tid:
            continue
        g = groups.get(tid)
        if g is None:
            groups[tid] = {"id": tid, "files": [], "size": 0, "mtime": 0.0, "start": None}
            g = groups[tid]
        g["files"].append(path)
        g["size"] += st.st_size
        g["mtime"] = max(g["mtime"], st.st_mtime)
    titles = _titles()
    UUID_RE = re.compile(r"[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}")
    out = []
    for tid, g in groups.items():
        name = None
        for p in g["files"]:
            m = UUID_RE.findall(p.stem)
            if m:
                name = titles.get(m[-1])
                if name:
                    break
        name = name or titles.get(tid)
        t = {
            "id": tid,
            "title": name or "(без названия)",
            "files": len(g["files"]),
            "size": g["size"],
            "mtime": datetime.fromtimestamp(g["mtime"]),
        }
        out.append(t)
    out.sort(key=lambda x: x["mtime"], reverse=True)
    return out


def thread_title(tid: str) -> str:
    for t in list_threads():
        if t["id"] == tid:
            return t["title"]
    return tid


def _join_content_text(content) -> str:
    parts = [x.get("text", "") for x in (content or []) if isinstance(x, dict) and x.get("text")]
    return "\n\n".join(parts)


def _history_item(o, fi, position):
    t = o.get("type")
    pl = o.get("payload") or {}
    pt = pl.get("type")
    base = {"key": f"{fi}:{position}", "ts": o.get("timestamp", ""), "fi": fi, "line": position}
    if t == "event_msg":
        if pt == "task_complete" and pl.get("error"):
            err = pl.get("error")
            raw_msg = err.get("message") if isinstance(err, dict) else str(err)
            raw_code = (err.get("codex_error_info") or err.get("codexErrorInfo")) if isinstance(err, dict) else "error"
            meta = describe_codex_error(raw_code, raw_msg)
            base.update(
                kind="error",
                subtype="codex_error",
                label=meta["category"],
                code=meta["code"],
                text=meta["message"],
                turn_id=pl.get("turn_id"),
            )
            return base
        if pt == "turn_aborted":
            reason = pl.get("reason") or "interrupted"
            txt = "Генерация прервана пользователем" if reason == "interrupted" else str(reason)
            base.update(kind="system", subtype="aborted", label="Ход прерван", text=txt, turn_id=pl.get("turn_id"))
            return base
        return None
    if t != "response_item":
        return None
    if pt == "message":
        role = pl.get("role")
        txt = _join_content_text(pl.get("content"))
        kind = ("system" if AUTO_INJECT_RE.match(txt) else "user") if role == "user" else ("assistant" if role == "assistant" else "system")
        base.update(kind=kind, text=txt, id=pl.get("id"), role=role)
    elif pt == "reasoning":
        parts = [c.get("text", "") for c in (pl.get("summary") or []) if isinstance(c, dict) and c.get("text")]
        base.update(kind="tool", label="reasoning", text="\n\n".join(parts))
    elif pt in ("custom_tool_call", "function_call"):
        args = pl.get("input") if "input" in pl else pl.get("arguments")
        txt = "" if args is None else (args if isinstance(args, str) else json.dumps(args, ensure_ascii=False, indent=2))
        base.update(kind="tool", label=pl.get("name") or pt, text=txt)
    elif pt in ("custom_tool_call_output", "function_call_output"):
        out = pl.get("output")
        txt = "" if out is None else (out if isinstance(out, str) else json.dumps(out, ensure_ascii=False, indent=2))
        base.update(kind="tool", label="вывод", text=txt, status=pl.get("status"))
    else:
        return None
    return base


def _reverse_lines(path: Path, end=None, block_size=65536):
    with path.open("rb") as fh:
        fh.seek(0, 2)
        size = fh.tell()
        pos = size if end is None else end
        if pos < 0 or pos > size:
            raise ValueError("Некорректный курсор истории")
        fragments = []
        while pos:
            n = min(block_size, pos)
            pos -= n
            fh.seek(pos)
            chunk = fh.read(n)
            end_in_chunk = len(chunk)
            while True:
                newline = chunk.rfind(b"\n", 0, end_in_chunk)
                if newline < 0:
                    if end_in_chunk:
                        fragments.append(chunk[:end_in_chunk])
                    break
                line = chunk[newline + 1:end_in_chunk]
                if fragments:
                    line += b"".join(reversed(fragments))
                    fragments.clear()
                if line:
                    yield pos + newline + 1, line.rstrip(b"\r")
                end_in_chunk = newline
        if fragments:
            yield 0, b"".join(reversed(fragments)).rstrip(b"\r")


def read_history_page(tid: str, limit=100, before=None, include_tools=False):
    files = thread_files(tid)
    if not files:
        return [], None
    last, end = len(files) - 1, None
    if before:
        fi, location = before.split(":", 1)
        if not location.startswith("b"):
            raise ValueError("Некорректный курсор истории")
        last, end = int(fi), int(location[1:])
        if last < 0 or last >= len(files):
            raise ValueError("Курсор больше не существует")
    items, messages = [], 0
    for fi in range(last, -1, -1):
        for offset, raw in _reverse_lines(files[fi], end if fi == last else None):
            if not include_tools:
                is_msg = b'"response_item"' in raw and b'"message"' in raw
                is_err = b'"event_msg"' in raw and b'"task_complete"' in raw and b'"error"' in raw
                if not (is_msg or is_err):
                    continue
            try:
                item = _history_item(json.loads(raw), fi, f"b{offset}")
            except (ValueError, UnicodeDecodeError):
                continue
            if item is None:
                continue
            if not include_tools and item["kind"] not in ("user", "assistant", "error"):
                continue
            items.append(item)
            messages += item["kind"] in ("user", "assistant")
            if messages >= limit or len(items) >= 2000:
                cursor = item["key"] if fi > 0 or offset > 0 else None
                return list(reversed(items)), cursor
    return list(reversed(items)), None


def resolve_key(tid: str, key):
    if not isinstance(key, str) or ":b" not in key:
        return key
    fi_text, byte_text = key.split(":b", 1)
    fi, offset = int(fi_text), int(byte_text)
    files = thread_files(tid)
    if fi < 0 or fi >= len(files) or offset < 0:
        raise ValueError("Ключ сообщения больше не существует")
    with files[fi].open("rb") as fh:
        remaining, line_count, last_byte = offset, 0, b"\n"
        while remaining:
            chunk = fh.read(min(remaining, 1024 * 1024))
            if not chunk:
                raise ValueError("Ключ сообщения больше не существует")
            line_count += chunk.count(b"\n")
            last_byte = chunk[-1:]
            remaining -= len(chunk)
        if last_byte != b"\n" or not fh.read(1):
            raise ValueError("Ключ сообщения больше не существует")
    return f"{fi}:{line_count}"


def read_history(tid: str) -> list:
    files = thread_files(tid)
    items = []
    for fi, path in enumerate(files):
        with open(path, encoding="utf-8", errors="replace") as fh:
            lines = fh.read().splitlines()
        for ln, line in enumerate(lines):
            try:
                o = json.loads(line)
            except Exception:
                continue
            item = _history_item(o, fi, ln)
            if item is not None:
                items.append(item)
    return items


def message_before_key(tid: str, key: str) -> str | None:
    key = resolve_key(tid, key)
    items = read_history(tid)
    idx = next((n for n, it in enumerate(items) if it["key"] == key), None)
    if idx is None:
        raise LookupError(f"ключ {key} не найден в треде")
    if idx == 0:
        return None
    return items[idx - 1]["key"]


def turns_from_history(items: list) -> list:
    turns = []
    cur = None
    for it in items:
        if it["kind"] == "user":
            cur = {"user": it, "items": [it]}
            turns.append(cur)
        else:
            if cur is None:
                cur = {"user": None, "items": []}
                turns.append(cur)
            cur["items"].append(it)
    return turns


def parse_key(key) -> tuple[int, int]:
    if key in (None, "", "start", "all"):
        return -1, -1
    fi, ln = str(key).split(":")
    return int(fi), int(ln)


def _byte_key(files, key):
    fi_text, pos = str(key).split(":", 1)
    fi = int(fi_text)
    if fi < 0 or fi >= len(files):
        raise ValueError("Ключ сообщения больше не существует")
    with files[fi].open("rb") as f:
        if pos.startswith("b"):
            offset = int(pos[1:])
            if offset < 0:
                raise ValueError("Недопустимый ключ сообщения")
            if offset:
                f.seek(offset - 1)
                if f.read(1) != b"\n":
                    raise ValueError("Ключ сообщения больше не существует")
            f.seek(offset)
        else:
            line = int(pos)
            if line < 0:
                raise ValueError("Недопустимый ключ сообщения")
            for _ in range(line):
                if not f.readline():
                    raise ValueError("Ключ сообщения больше не существует")
            offset = f.tell()
        try:
            item = _history_item(json.loads(f.readline()), fi, f"b{offset}")
        except (ValueError, UnicodeDecodeError):
            item = None
        if item is None:
            raise ValueError("Ключ сообщения больше не существует")
    return fi, offset


_ITEM_EVENT_TYPE = {"user": "UserMessage", "assistant": "AgentMessage"}


def _extend_item_events(f, kind):
    want = _ITEM_EVENT_TYPE.get(kind)
    if not want:
        return f.tell()
    while True:
        pos = f.tell()
        raw = f.readline()
        if not raw or b'"item_completed"' not in raw:
            return pos
        try:
            event = json.loads(raw)
        except ValueError:
            return pos
        item = (event.get("payload") or {}).get("item") or {}
        if (event.get("payload") or {}).get("type") != "item_completed" or item.get("type") != want:
            return pos


def preview_cut_fast(tid: str, key: str, mode: str, *, delete_selected=False, scan_limit=16 * 1024 * 1024):
    if mode not in ("head", "tail") or not key:
        raise ValueError("Недопустимый ключ или режим обрезки")
    files = thread_files(tid)
    fi, offset = _byte_key(files, key)
    sizes = [p.stat().st_size for p in files]
    ranges = []
    if mode == "tail":
        keep_fi, keep_end = fi, offset
        if delete_selected:
            keep_fi, keep_end = -1, 0
            for j in range(fi, -1, -1):
                for start, raw in _reverse_lines(files[j], offset if j == fi else None):
                    try:
                        item = _history_item(json.loads(raw), j, f"b{start}")
                    except (ValueError, UnicodeDecodeError):
                        continue
                    if item is not None:
                        with files[j].open("rb") as f:
                            f.seek(start)
                            f.readline()
                            keep_end = _extend_item_events(f, item["kind"])
                        keep_fi = j
                        break
                if keep_fi >= 0:
                    break
        else:
            with files[fi].open("rb") as f:
                f.seek(offset)
                raw = f.readline()
                try:
                    kind = _history_item(json.loads(raw), fi, f"b{offset}")
                except (ValueError, UnicodeDecodeError):
                    kind = None
                keep_end = _extend_item_events(f, kind["kind"] if kind else None)
        for j, size in enumerate(sizes):
            if j > keep_fi:
                ranges.append((j, 0, size))
            elif j == keep_fi and keep_end < size:
                ranges.append((j, keep_end, size))
    else:
        for j, size in enumerate(sizes):
            if j < fi:
                ranges.append((j, 0, size))
            elif j == fi:
                with files[j].open("rb") as f:
                    meta_end = len(f.readline())
                if offset < meta_end:
                    raise ValueError("Нельзя удалить метаданные сессии")
                if offset > meta_end:
                    ranges.append((j, meta_end, offset))

    removed_bytes = sum(end - start for _, start, end in ranges)
    exact = removed_bytes <= scan_limit
    lines_count, message_count, previews = 0, 0, []
    if exact:
        for j, start, end in ranges:
            with files[j].open("rb") as f:
                f.seek(start)
                while f.tell() < end:
                    pos = f.tell()
                    raw = f.readline(end - pos)
                    if not raw:
                        break
                    lines_count += 1
                    if b'"response_item"' not in raw or b'"message"' not in raw:
                        continue
                    try:
                        item = _history_item(json.loads(raw), j, f"b{pos}")
                    except (ValueError, UnicodeDecodeError):
                        continue
                    if item and item["kind"] in ("user", "assistant"):
                        message_count += 1
                        previews.append({"key": item["key"], "label": "Вы" if item["kind"] == "user" else "Ассистент", "text": item["text"][:300]})
                        if len(previews) > 30:
                            previews.pop(0)
    return {
        "thread": tid, "mode": mode, "files_total": len(files),
        "rollout_lines": lines_count if exact else None,
        "rollout_files": sum(start == 0 and end == sizes[j] for j, start, end in ranges),
        "bytes": removed_bytes, "count_incomplete": not exact,
        "will_delete": {"steps_preview": previews, "steps_preview_count": message_count if exact else None},
    }


def plan_cut(tid: str, key, mode: str = "tail") -> dict:
    files = thread_files(tid)
    fi, ln = parse_key(key)
    rep = {"thread": tid, "mode": mode, "files_total": len(files),
           "rollout_lines": 0, "rollout_files": 0, "bytes": 0}
    for j, path in enumerate(files):
        with open(path, encoding="utf-8", errors="replace") as fh:
            lines = fh.read().splitlines()
        n = len(lines)
        if fi < 0:
            cut_from = 0
        elif mode == "tail":
            if j < fi:
                continue
            cut_from = (ln + 1) if j == fi else 0
        else:
            if j > fi:
                continue
            if j == fi:
                removed = lines[1:ln]
                rep["rollout_lines"] += len(removed)
                rep["bytes"] += sum(len(x.encode("utf-8")) + 1 for x in removed)
                continue
            cut_from = 0
        if cut_from >= n:
            removed_lines = 0
        else:
            removed_lines = n - cut_from
            rep["bytes"] += sum(len(x.encode("utf-8")) + 1 for x in lines[cut_from:])
        rep["rollout_lines"] += removed_lines
        if cut_from == 0:
            rep["rollout_files"] += 1
    try:
        con = _proj_ro()
        if con:
            rep["projection_items"] = con.execute(
                "select count(*) from thread_items where thread_id=?", (tid,)).fetchone()[0]
            rep["projection_turns"] = con.execute(
                "select count(*) from thread_turns where thread_id=?", (tid,)).fetchone()[0]
            con.close()
    except Exception:
        pass
    return rep


def _proj_ro():
    if not PROJECTION_DB.exists():
        return None
    try:
        return sqlite3.connect(f"file:{PROJECTION_DB}?mode=ro", uri=True)
    except Exception:
        return None


def purge_projection(tid: str) -> dict:
    require_writable()
    con = sqlite3.connect(str(PROJECTION_DB), timeout=3.0)
    try:
        con.execute("BEGIN IMMEDIATE")
        n_items = con.execute("delete from thread_items where thread_id=?", (tid,)).rowcount
        n_turns = con.execute("delete from thread_turns where thread_id=?", (tid,)).rowcount
        n_real = con.execute("delete from thread_realtime_items where thread_id=?", (tid,)).rowcount
        con.execute("delete from thread_history_projection_state where thread_id=?", (tid,))
        con.execute("commit")
    except Exception:
        if con.in_transaction:
            con.execute("rollback")
        raise
    finally:
        con.close()
    return {"items": n_items, "turns": n_turns, "realtime": n_real}


def _patch_message_obj(pl: dict, new_text: str) -> dict:
    content = pl.get("content") or []
    newc = []
    did = False
    for c in content:
        if isinstance(c, dict) and c.get("type") in (
                "text", "input_text", "output_text") and "text" in c:
            if not did:
                newc.append({**c, "text": new_text})
                did = True
        else:
            newc.append(c)
    if not did:
        t = "output_text" if pl.get("role") == "assistant" else "input_text"
        if content and isinstance(content[0], dict) and content[0].get("type") in ("text", "input_text", "output_text"):
            t = content[0]["type"]
        newc.append({"type": t, "text": new_text})
    pl["content"] = newc
    return pl


def extract_item_edit_info(msg: dict) -> dict:
    if msg.get("type") != "response_item":
        raise ValueError("Ключ не указывает на редактируемую запись истории")
    pl = msg.get("payload") or {}
    pt = pl.get("type")
    if pt == "message":
        role = pl.get("role") or "system"
        text = _join_content_text(pl.get("content"))
        is_auto = role == "user" and bool(AUTO_INJECT_RE.match(text))
        if is_auto:
            kind, label, mirror = "environment", "Контекст окружения", False
        elif role == "user":
            kind, label, mirror = "user", "Сообщение пользователя", True
        elif role == "assistant":
            kind, label, mirror = "assistant", "Ответ Codex", True
        elif role == "developer":
            kind, label, mirror = "developer", "Системные инструкции", False
        else:
            kind, label, mirror = "system", "Системное сообщение", False
        return {"old": text, "kind": kind, "label": label, "role": role, "pt": pt, "mirror_events": mirror}
    if pt == "reasoning":
        parts = [c.get("text", "") for c in (pl.get("summary") or []) if isinstance(c, dict) and c.get("text")]
        return {"old": "\n\n".join(parts), "kind": "reasoning", "label": "Размышления", "role": None, "pt": pt, "mirror_events": False}
    if pt in ("custom_tool_call", "function_call"):
        field = "input" if "input" in pl else "arguments"
        raw_val = pl.get(field)
        text = "" if raw_val is None else (raw_val if isinstance(raw_val, str) else json.dumps(raw_val, ensure_ascii=False, indent=2))
        tname = pl.get("name") or pt
        return {"old": text, "kind": "tool_call", "label": f"Вызов инструмента: {tname}", "role": None, "pt": pt, "mirror_events": False}
    if pt in ("custom_tool_call_output", "function_call_output"):
        raw_val = pl.get("output")
        text = "" if raw_val is None else (raw_val if isinstance(raw_val, str) else json.dumps(raw_val, ensure_ascii=False, indent=2))
        return {"old": text, "kind": "tool_output", "label": "Вывод инструмента", "role": None, "pt": pt, "mirror_events": False}
    raise ValueError(f"Запись типа {pt or 'unknown'} не поддерживает редактирование текста")


def patch_response_item(pl: dict, new_text: str) -> dict:
    pt = pl.get("type")
    if pt == "message":
        return _patch_message_obj(pl, new_text)
    if pt == "reasoning":
        summary = pl.get("summary") or []
        kept = [c for c in summary if not (isinstance(c, dict) and "text" in c)]
        first_type = "summary_text"
        for c in summary:
            if isinstance(c, dict) and c.get("type"):
                first_type = c["type"]
                break
        pl["summary"] = [{"type": first_type, "text": new_text}] + kept
        return pl
    if pt in ("custom_tool_call", "function_call"):
        field = "input" if "input" in pl else "arguments"
        orig = pl.get(field)
        if isinstance(orig, (dict, list)):
            try:
                pl[field] = json.loads(new_text)
            except Exception:
                pl[field] = new_text
        else:
            pl[field] = new_text
        return pl
    if pt in ("custom_tool_call_output", "function_call_output"):
        orig = pl.get("output")
        if isinstance(orig, (dict, list)):
            try:
                pl["output"] = json.loads(new_text)
            except Exception:
                pl["output"] = new_text
        else:
            pl["output"] = new_text
        return pl
    raise ValueError(f"Запись типа {pt or 'unknown'} не поддерживает редактирование")


_PROJECTION_TABLES = ("thread_items", "thread_turns", "thread_realtime_items", "thread_history_projection_state")


def _backup_projection(tid: str, destination: Path):
    if not PROJECTION_DB.exists():
        return
    src = sqlite3.connect(PROJECTION_DB.resolve().as_uri() + "?mode=ro", uri=True)
    dst = sqlite3.connect(str(destination))
    try:
        src.execute("BEGIN")
        dst.execute("BEGIN")
        for table in _PROJECTION_TABLES:
            schema = src.execute("select sql from sqlite_master where type='table' and name=?", (table,)).fetchone()
            if not schema or not schema[0]:
                raise ValueError(f"Не найдена таблица проекции: {table}")
            dst.execute(schema[0])
            cursor = src.execute(f'SELECT * FROM "{table}" WHERE thread_id=?', (tid,))
            placeholders = ",".join("?" for _ in cursor.description)
            while True:
                rows = cursor.fetchmany(100)
                if not rows:
                    break
                dst.executemany(f'INSERT INTO "{table}" VALUES ({placeholders})', rows)
        dst.commit()
    finally:
        src.close()
        dst.close()


def _restore_projection(tid: str, snapshot: Path):
    con = sqlite3.connect(str(PROJECTION_DB), timeout=3.0)
    try:
        con.execute("ATTACH DATABASE ? AS saved", (str(snapshot),))
        if con.execute("PRAGMA saved.quick_check").fetchone()[0] != "ok":
            raise ValueError("Повреждена резервная копия проекции")
        for table in _PROJECTION_TABLES:
            live_cols = [r[1] for r in con.execute(f'PRAGMA main.table_info("{table}")')]
            saved_cols = [r[1] for r in con.execute(f'PRAGMA saved.table_info("{table}")')]
            if not live_cols or live_cols != saved_cols or "thread_id" not in live_cols:
                raise ValueError(f"Схема резервной копии {table} не совпадает с текущей")
        con.execute("BEGIN IMMEDIATE")
        for table in _PROJECTION_TABLES:
            con.execute(f'DELETE FROM main."{table}" WHERE thread_id=?', (tid,))
            con.execute(f'INSERT INTO main."{table}" SELECT * FROM saved."{table}" WHERE thread_id=?', (tid,))
        con.commit()
    except Exception:
        if con.in_transaction:
            con.rollback()
        raise
    finally:
        con.close()


def full_backup(tid: str, tag: str = "", backups_root: Path | None = None) -> Path:
    require_writable()
    broot = backups_root or (CODEX_HOME.parent / "codex_backups")
    ts = datetime.now().strftime("%Y%m%d-%H%M%S-%f")
    bdir = broot / tid / (ts + ("-" + tag if tag else ""))
    bdir.mkdir(parents=True, exist_ok=False)
    nfiles = 0
    for p in thread_files(tid):
        dst = bdir / "sessions" / p.relative_to(SESSIONS)
        dst.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(p, dst)
        nfiles += 1
    _backup_projection(tid, bdir / "projection.sqlite")
    (bdir / "backup.meta.json").write_text(
        json.dumps({"tid": tid, "created": ts, "files": nfiles, "tag": tag},
                   ensure_ascii=False, indent=1), encoding="utf-8")
    return bdir


def restore_backup(bdir: str | Path):
    bdir = Path(bdir)
    require_writable()
    meta = json.loads((bdir / "backup.meta.json").read_text(encoding="utf-8"))
    tid = meta["tid"]
    sroot = bdir / "sessions"
    files = list(sroot.rglob("rollout-*.jsonl")) if sroot.exists() else []
    if len(files) != meta["files"] or any(_file_meta(p)[0] != tid for p in files):
        raise ValueError("Резервная копия неполная или содержит посторонние файлы")
    destinations = {SESSIONS / p.relative_to(sroot) for p in files}
    for p in files:
        dst = SESSIONS / p.relative_to(sroot)
        dst.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(p, dst)
    for p in thread_files(tid):
        if p not in destinations:
            p.unlink()
    _FILE_CACHE.clear()
    snapshot = bdir / "projection.sqlite"
    if snapshot.exists():
        _restore_projection(tid, snapshot)
    else:
        purge_projection(tid)


def check_leak(tid: str, removed_texts, backups: Path | None = None) -> list:
    leaks = []
    frags = []
    for t in removed_texts:
        for s in (t or "").split("\n"):
            s2 = re.sub(r"[\\\"`]+", "", s).strip()
            if len(s2) >= 70:
                c2 = re.sub(r"\s+", " ", s2)
                frags.append(c2[10:95])
                break
    if not frags:
        return leaks
    targets = []
    for p in thread_files(tid):
        if p.exists() and (p.is_file()):
            targets.append(p)
    if HISTORY_JSONL.exists() and (backups is None or HISTORY_JSONL != backups):
        targets.append(HISTORY_JSONL)
    blobs = [(p, p.read_bytes()) for p in targets]
    for p, data in blobs:
        for frag in frags:
            if frag.encode("utf-8", "ignore") in data:
                leaks.append((str(p), frag))
    return leaks


def cut(tid: str, key, preview_only: bool = False, mode: str = "tail") -> dict:
    key = resolve_key(tid, key)
    files = thread_files(tid)
    fi, ln = parse_key(key)
    if mode not in ("head", "tail") or (fi < 0 and mode != "tail"):
        raise ValueError("Недопустимый режим обрезки")
    if fi >= 0 and (fi >= len(files) or ln < 0 or not any(x["key"] == key for x in read_history(tid))):
        raise ValueError("Ключ сообщения больше не существует")
    if mode == "head" and fi >= 0 and len(files) != 1:
        with files[fi].open("rb") as f:
            if "ordinal" in json.loads(f.readline()):
                raise ValueError("Обрезка начала многосегментного треда пока не поддерживается")
    if not preview_only:
        require_writable()
    rep = {"thread": tid, "mode": mode, "rollout_lines": 0,
           "rollout_files": 0, "projection": None, "leaks": None}
    removed_texts = []

    for j, path in enumerate(files):
        with open(path, encoding="utf-8", errors="replace") as fh:
            lines = fh.read().splitlines()
        size = path.stat().st_size
        if fi < 0 or (mode == "tail" and j > fi) or (mode == "head" and j < fi):
            removed_texts.extend(_extract_texts(lines))
            rep["rollout_lines"] += len(lines)
            rep["rollout_files"] += 1
            rep["bytes"] = rep.get("bytes", 0) + size
            if not preview_only:
                path.unlink()
            continue
        if j != fi:
            continue
        if mode == "tail":
            new = lines[:ln + 1]
            cut_lines = lines[ln + 1:]
        else:
            new = [lines[0]] + lines[ln:]
            cut_lines = lines[1:ln]
            meta = json.loads(lines[0])
            if "ordinal" in meta:
                mbody = meta.get("payload") or {}
                start = int(meta["ordinal"])
                if (start != 0 and not (mbody.get("history_base") or mbody.get("forked_from_id"))) or len(files) != 1:
                    raise ValueError("Неизвестный формат нумерации: обрезка отменена")
                previous = None
                for n in range(1, len(new)):
                    row = json.loads(new[n])
                    old_ordinal = row.get("ordinal")
                    if not isinstance(old_ordinal, int) or (previous is not None and old_ordinal != previous + 1):
                        raise ValueError("Нарушена нумерация rollout: обрезка отменена")
                    previous = old_ordinal
                    row["ordinal"] = start + n
                    new[n] = json.dumps(row, ensure_ascii=False)
        removed_texts.extend(_extract_texts(cut_lines))
        rep["rollout_lines"] += len(cut_lines)
        rep["bytes"] = rep.get("bytes", 0) + sum(len(x.encode("utf-8")) + 1 for x in cut_lines)
        if preview_only:
            continue
        if len(new) <= 1:
            path.unlink()
            rep["rollout_files"] += 1
        else:
            tmp = path.with_suffix(".tmp")
            tmp.write_text("\n".join(new) + "\n", encoding="utf-8")
            tmp.replace(path)
    if preview_only:
        rep.update(plan_cut(tid, key, mode))
        return rep
    rep["projection"] = purge_projection(tid)
    rep["leaks"] = check_leak(tid, removed_texts)
    _FILE_CACHE.clear()
    return rep


def _extract_texts(lines) -> list:
    texts = []
    for l in lines:
        try:
            o = json.loads(l)
        except Exception:
            continue
        pl = o.get("payload") or {}
        if o.get("type") == "response_item":
            if pl.get("type") == "message":
                txt = "".join(x.get("text", "") for x in (pl.get("content") or [])
                              if isinstance(x, dict) and x.get("text"))
                if txt:
                    texts.append(txt)
            elif pl.get("type") == "custom_tool_call":
                args = pl.get("input") or ""
                if isinstance(args, str) and len(args) > 100:
                    texts.append(args)
            elif pl.get("type") == "reasoning":
                s = "".join(c.get("text", "") for c in (pl.get("summary") or [])
                            if isinstance(c, dict))
                if len(s) > 120:
                    texts.append(s)
    return texts


def _json_escape_frag(text: str) -> str:
    return json.dumps(text, ensure_ascii=False)[1:-1]


def _deep_replace_text(obj, old_text: str, new_text: str) -> bool:
    hit = False
    if isinstance(obj, str):
        return obj == old_text
    if isinstance(obj, list):
        for i, v in enumerate(obj):
            if _deep_replace_text(v, old_text, new_text):
                obj[i] = new_text if isinstance(v, str) else v
                hit = True
        return hit
    if isinstance(obj, dict):
        for k, v in list(obj.items()):
            if _deep_replace_text(v, old_text, new_text):
                obj[k] = new_text if isinstance(v, str) else v
                hit = True
        return hit
    return hit


def edit_message(tid: str, key: str, new_text: str, preview_only: bool = True) -> dict:
    files = thread_files(tid)
    if preview_only and isinstance(key, str) and ":b" in key:
        fi_str, offset_str = key.split(":b", 1)
        fi, offset = int(fi_str), int(offset_str)
        if fi < 0 or fi >= len(files) or offset < 0:
            raise ValueError("Ключ сообщения больше не существует")
        with files[fi].open("rb") as fh:
            fh.seek(offset)
            msg = json.loads(fh.readline())
        info = extract_item_edit_info(msg)
        return {"tid": tid, "key": key, "old": info["old"], "kind": info["kind"], "label": info["label"]}
    key = resolve_key(tid, key)
    fi, ln = parse_key(key)
    path = files[fi]
    with open(path, encoding="utf-8", errors="replace") as fh:
        lines = fh.read().splitlines()
    msg = json.loads(lines[ln])
    pl = msg.get("payload") or {}
    info = extract_item_edit_info(msg)
    cur = info["old"]
    if preview_only:
        return {"tid": tid, "key": key, "old": cur, "kind": info["kind"], "label": info["label"]}
    if not new_text or not new_text.strip():
        raise ValueError("Пустой текст сообщения")
    mirror = info["mirror_events"] and bool(cur.strip())
    duplicate_messages = 0
    if mirror:
        for raw in lines:
            try:
                row = json.loads(raw)
                payload = row.get("payload") or {}
                if row.get("type") == "response_item" and payload.get("type") == "message":
                    text = _join_content_text(payload.get("content"))
                    duplicate_messages += text == cur
            except (ValueError, TypeError):
                continue
    patch_response_item(pl, new_text)
    msg["payload"] = pl
    lines[ln] = json.dumps(msg, ensure_ascii=False)
    changed = [ln]
    if mirror:
        for i, raw in enumerate(lines):
            if i == ln:
                continue
            try:
                o = json.loads(raw)
            except Exception:
                continue
            if o.get("type") != "event_msg":
                continue
            event = o.get("payload") or {}
            item = event.get("item") or {}
            item_id = item.get("id") if isinstance(item, dict) else None
            if item_id and item_id != pl.get("id"):
                expected = "UserMessage" if pl.get("role") == "user" else "AgentMessage"
                if duplicate_messages != 1 or item.get("type") != expected:
                    continue
            if not item_id and (duplicate_messages != 1 or event.get("type") not in ("task_complete", "agent_message", "user_message")):
                continue
            if _deep_replace_text(o, cur, new_text):
                lines[i] = json.dumps(o, ensure_ascii=False)
                changed.append(i)
    require_writable()
    tmp = path.with_suffix(".tmp")
    tmp.write_text("\n".join(lines) + "\n", encoding="utf-8")
    tmp.replace(path)
    proj = purge_projection(tid)
    leaks = check_leak(tid, [cur]) if mirror else []
    _FILE_CACHE.clear()
    return {"tid": tid, "key": key, "old": cur, "new": new_text,
            "lines_changed": changed, "projection": proj, "leaks": leaks}


CODEX_BLOCK_PRESETS = [
    {
        "id": "safety",
        "title": "Safety Guard",
        "code": "misalignmentPolicyViolation",
        "rollout_code": "misalignment_policy_violation",
        "message": "This request was blocked by our safety systems. Reason: Potentially unintended activity.",
    },
    {
        "id": "cyber",
        "title": "Cyber Policy",
        "code": "cyberPolicy",
        "rollout_code": "cyber_policy",
        "message": "This content was flagged for possible cybersecurity risk. If this seems wrong, try rephrasing your request.",
    },
    {
        "id": "usage",
        "title": "Usage Limit",
        "code": "usageLimitExceeded",
        "rollout_code": "usage_limit_exceeded",
        "message": "You've hit your usage limit. Upgrade to Pro or try again later.",
    },
    {
        "id": "overload",
        "title": "Overloaded",
        "code": "serverOverloaded",
        "rollout_code": "server_overloaded",
        "message": "Selected model is at capacity. Please try a different model.",
    },
    {
        "id": "unauth",
        "title": "Unauthorized",
        "code": "unauthorized",
        "rollout_code": "unauthorized",
        "message": "Your access token could not be refreshed.",
    },
    {
        "id": "badreq",
        "title": "Bad Request",
        "code": "other",
        "rollout_code": "other",
        "message": '{"detail":"Bad Request"}',
    },
]

_LIGHT_BACKUP_THRESHOLD = 256 * 1024 * 1024


def _normalize_error_code(raw_code) -> str:
    if isinstance(raw_code, dict):
        if raw_code:
            first_key = next(iter(raw_code.keys()))
            return str(first_key)
        return "other"
    return str(raw_code or "").strip() or "failed"


def _clean_error_message(raw_message) -> str:
    msg = str(raw_message or "").strip()
    if msg.startswith("{") and msg.endswith("}"):
        try:
            parsed = json.loads(msg)
            if isinstance(parsed, dict) and parsed.get("detail"):
                return str(parsed["detail"])
        except Exception:
            pass
    return msg


def describe_codex_error(raw_code, raw_message) -> dict:
    norm_raw = _normalize_error_code(raw_code)
    camel_code, _ = _code_variants(norm_raw)
    low_code = camel_code.lower()
    msg = _clean_error_message(raw_message)
    low_msg = msg.lower()

    if "misalignment" in low_code or "blocked by our safety" in low_msg or "safety system" in low_msg:
        cat = "Блокировка Safety Guard"
        prompt = "Этот диалог заблокирован системой безопасности (Safety Guard). Хотите снять блокировку?"
    elif "cyber" in low_code or "cybersecurity" in low_msg:
        cat = "Ограничение Cyber Policy"
        prompt = "В диалоге сработало ограничение Cyber Policy. Хотите снять его?"
    elif "usagelimit" in low_code or "usage limit" in low_msg:
        cat = "Лимит использования"
        prompt = "Последний ход остановлен из-за лимита использования. Хотите очистить ограничение?"
    elif "overload" in low_code or "at capacity" in low_msg:
        cat = "Перегрузка сервера"
        prompt = "Последний ход прерван из-за перегрузки сервера. Хотите очистить ошибку?"
    elif "unauth" in low_code or "refresh token" in low_msg or "access token" in low_msg:
        cat = "Ошибка авторизации"
        prompt = "Последний ход завершился ошибкой авторизации. Хотите очистить ошибку?"
    elif "connection" in low_code or "http" in low_code or "stream disconnected" in low_msg or "connection failed" in low_msg:
        cat = "Сбой соединения"
        prompt = "Последний ход прерван из-за обрыва соединения. Хотите очистить ошибку?"
    elif "bad request" in low_msg or "not supported" in low_msg:
        cat = "Ошибка запроса"
        prompt = "Последний ход завершился ошибкой запроса. Хотите очистить ошибку?"
    else:
        cat = "Ограничение Codex"
        prompt = "Этот диалог заблокирован или завершился с ошибкой. Хотите снять блокировку?"

    return {
        "code": camel_code,
        "category": cat,
        "prompt": prompt,
        "message": msg or cat,
    }


def _code_variants(code: str) -> tuple[str, str]:
    raw = _normalize_error_code(code) or "misalignmentPolicyViolation"
    for p in CODEX_BLOCK_PRESETS:
        if raw.lower() in (p["code"].lower(), p["rollout_code"].lower(), p["id"]):
            return p["code"], p["rollout_code"]
    if "_" in raw:
        parts = [x for x in raw.split("_") if x]
        camel = parts[0].lower() + "".join(x.capitalize() for x in parts[1:]) if parts else raw
        return camel, raw.lower()
    snake = re.sub(r"([a-z0-9])([A-Z])", r"\1_\2", raw).lower()
    return raw, snake


def detect_thread_block(tid: str) -> dict | None:
    if PROJECTION_DB.is_file():
        try:
            con = sqlite3.connect(PROJECTION_DB.resolve().as_uri() + "?mode=ro", uri=True)
            try:
                row = con.execute("""
                    SELECT turn_id, status, error_json, rollout_ordinal
                    FROM thread_turns
                    WHERE thread_id = ?
                    ORDER BY rollout_ordinal DESC LIMIT 1
                """, (tid,)).fetchone()
                if row:
                    status = row[1]
                    err_raw = row[2]
                    if status == "failed" or err_raw:
                        err_data = {}
                        if err_raw:
                            try:
                                err_data = json.loads(err_raw)
                            except Exception:
                                pass
                        raw_msg = err_data.get("message") or f"Статус хода: {status}"
                        raw_code = err_data.get("codexErrorInfo") or err_data.get("codex_error_info") or "failed"
                        meta = describe_codex_error(raw_code, raw_msg)
                        return {
                            "is_blocked": True,
                            "turn_id": row[0],
                            "status": status,
                            "message": meta["message"],
                            "code": meta["code"],
                            "category": meta["category"],
                            "prompt": meta["prompt"],
                        }
            finally:
                con.close()
        except Exception:
            pass

    files = thread_files(tid)
    if files:
        try:
            for idx, (_offset, raw) in enumerate(_reverse_lines(files[-1])):
                if idx >= 40:
                    break
                if b'"task_complete"' not in raw:
                    continue
                try:
                    obj = json.loads(raw)
                    pl = obj.get("payload") or {}
                    if pl.get("type") == "task_complete":
                        err = pl.get("error")
                        if err:
                            raw_msg = err.get("message") if isinstance(err, dict) else str(err)
                            raw_code = (err.get("codex_error_info") or err.get("codexErrorInfo")) if isinstance(err, dict) else "error"
                            meta = describe_codex_error(raw_code, raw_msg)
                            return {
                                "is_blocked": True,
                                "turn_id": pl.get("turn_id"),
                                "status": "failed",
                                "message": meta["message"],
                                "code": meta["code"],
                                "category": meta["category"],
                                "prompt": meta["prompt"],
                            }
                        return None
                except Exception:
                    continue
        except Exception:
            pass
    return None


def _sync(fh):
    fh.flush()
    try:
        os.fsync(fh.fileno())
    except OSError:
        pass


def _check_lock_writable(allow_real: bool = False):
    if not allow_real and CODEX_HOME.resolve().name.lower() not in ("codex_sandbox", "sandbox"):
        raise PermissionError("Запись разрешена только в песочнице или с явным allow_real=True.")
    if PROJECTION_DB.is_file():
        ok, msg = db_write_probe(PROJECTION_DB)
        if not ok:
            raise RuntimeError(
                f"База Codex занята другим процессом ({msg}). Закройте активный тред в Codex и повторите."
            )


def _open_locked_rollout(path: Path):
    from codex_io import lock_rollouts
    from contextlib import contextmanager

    @contextmanager
    def _locked_ctx():
        with lock_rollouts([path]) as locks:
            yield locks[path]

    return _locked_ctx()


def _create_lock_backup(tid: str, tag: str, tail_file: Path, tail_offset: int, fh=None) -> Path:
    files = thread_files(tid)
    broot = CODEX_HOME.parent / "codex_backups"
    ts = datetime.now().strftime("%Y%m%d-%H%M%S-%f")
    bdir = broot / tid / (ts + "-" + tag)
    bdir.mkdir(parents=True, exist_ok=False)

    total_size = sum(p.stat().st_size for p in files if p.exists())
    if total_size <= _LIGHT_BACKUP_THRESHOLD:
        nfiles = 0
        for fp in files:
            dst = bdir / "sessions" / fp.relative_to(SESSIONS)
            dst.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(fp, dst)
            nfiles += 1
        _backup_projection(tid, bdir / "projection.sqlite")
        (bdir / "backup.meta.json").write_text(
            json.dumps({"tid": tid, "created": ts, "files": nfiles, "tag": tag}, ensure_ascii=False, indent=1),
            encoding="utf-8",
        )
        return bdir

    rel = str(tail_file.relative_to(SESSIONS))
    if fh is not None:
        fh.seek(tail_offset)
        tail_bytes = fh.read()
    else:
        with tail_file.open("rb") as rf:
            rf.seek(tail_offset)
            tail_bytes = rf.read()
    (bdir / "tail.bin").write_bytes(tail_bytes)
    _backup_projection(tid, bdir / "projection.sqlite")
    (bdir / "backup.meta.json").write_text(
        json.dumps({
            "tid": tid,
            "created": ts,
            "files": len(files),
            "tag": tag,
            "tail_backup": {
                "rel": rel,
                "offset": tail_offset,
                "orig_size": tail_file.stat().st_size,
                "tail_len": len(tail_bytes),
            },
        }, ensure_ascii=False, indent=1),
        encoding="utf-8",
    )
    return bdir


def restore_tail_backup(bdir: Path, allow_real: bool = False):
    _check_lock_writable(allow_real=allow_real)
    meta = json.loads((bdir / "backup.meta.json").read_text(encoding="utf-8"))
    tid = meta["tid"]
    tb = meta.get("tail_backup") or {}
    rel = tb["rel"]
    offset = int(tb["offset"])
    target = SESSIONS / rel
    if not target.resolve().is_relative_to(SESSIONS.resolve()):
        raise ValueError("Недопустимый путь восстановления")
    tail_bytes = (bdir / "tail.bin").read_bytes()
    with _open_locked_rollout(target) as fh:
        fh.seek(offset)
        fh.write(tail_bytes)
        fh.truncate(offset + len(tail_bytes))
        _sync(fh)
    snap = bdir / "projection.sqlite"
    if snap.is_file() and PROJECTION_DB.is_file():
        _restore_projection(tid, snap)
    _FILE_CACHE.clear()
    _LIST_CACHE.clear()


def restore_lock_full_backup(bdir: Path, allow_real: bool = False):
    _check_lock_writable(allow_real=allow_real)
    meta = json.loads((bdir / "backup.meta.json").read_text(encoding="utf-8"))
    tid = meta["tid"]
    sroot = bdir / "sessions"
    files = list(sroot.rglob("rollout-*.jsonl")) if sroot.exists() else []
    if len(files) != meta["files"] or any(_file_meta(p)[0] != tid for p in files):
        raise ValueError("Резервная копия неполная или содержит посторонние файлы")
    for p in files:
        dst = SESSIONS / p.relative_to(sroot)
        if not dst.resolve().is_relative_to(SESSIONS.resolve()):
            raise ValueError("Недопустимый путь назначения")
        dst.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(p, dst)
    snap = bdir / "projection.sqlite"
    if snap.is_file() and PROJECTION_DB.is_file():
        _restore_projection(tid, snap)
    _FILE_CACHE.clear()
    _LIST_CACHE.clear()


def unlock_thread(tid: str, set_phase=lambda _: None, allow_real: bool = False, auto_backup: bool = True) -> dict:
    _check_lock_writable(allow_real=allow_real)
    files = thread_files(tid)
    if not files:
        raise FileNotFoundError(f"Тред {tid} не найден")
    p = files[-1]

    offsets_to_patch = []
    for idx, (offset, raw) in enumerate(_reverse_lines(p)):
        if idx >= 80:
            break
        if b'"task_complete"' in raw and b'"error"' in raw:
            offsets_to_patch.append((offset, raw))

    min_off = min((o for o, _ in offsets_to_patch), default=max(0, p.stat().st_size - 4096))
    lines_patched = 0
    turns_unlocked = 0

    with _open_locked_rollout(p) as fh:
        orig_size = p.stat().st_size
        bdir = None
        if auto_backup:
            set_phase("backup")
            bdir = _create_lock_backup(tid, "k1tty_unlock", p, min_off, fh=fh)
        set_phase("unlock")

        if offsets_to_patch:
            for offset, raw in offsets_to_patch:
                try:
                    obj = json.loads(raw)
                    pl = obj.get("payload") or {}
                    if pl.get("type") == "task_complete" and "error" in pl:
                        del pl["error"]
                        fh.seek(offset)
                        disk_line = fh.readline()
                        ending = b"\r\n" if disk_line.endswith(b"\r\n") else (b"\n" if disk_line.endswith(b"\n") else b"")
                        target_len = len(disk_line)
                        new_core = json.dumps(obj, ensure_ascii=False, separators=(",", ":")).encode("utf-8")
                        pad_needed = target_len - len(new_core) - len(ending)
                        if pad_needed >= 0:
                            fh.seek(offset)
                            fh.write(new_core + (b" " * pad_needed) + ending)
                            lines_patched += 1
                            continue
                        rest = fh.read()
                        fh.seek(offset)
                        fh.write(new_core + ending + rest)
                        fh.truncate()
                        lines_patched += 1
                except Exception:
                    pass
            _sync(fh)

        fh.seek(0, 2)
        new_size = fh.tell()
        delta_bytes = new_size - orig_size

        if PROJECTION_DB.is_file():
            con = sqlite3.connect(str(PROJECTION_DB), timeout=5.0)
            try:
                con.execute("BEGIN IMMEDIATE")
                cur = con.execute("""
                    UPDATE thread_turns
                    SET status = 'completed', error_json = NULL
                    WHERE thread_id = ? AND (status = 'failed' OR error_json IS NOT NULL)
                """, (tid,))
                turns_unlocked = cur.rowcount
                if delta_bytes != 0 and len(files) == 1:
                    con.execute(
                        "UPDATE thread_history_projection_state SET next_rollout_byte_offset = ? WHERE thread_id = ?",
                        (new_size, tid),
                    )
                con.execute("DELETE FROM thread_realtime_items WHERE thread_id = ?", (tid,))
                con.commit()
            except Exception:
                if con.in_transaction:
                    con.execute("ROLLBACK")
                raise
            finally:
                con.close()

    _FILE_CACHE.clear()
    _LIST_CACHE.clear()
    set_phase("done")
    return {
        "tid": tid,
        "unlocked": True,
        "backup": str(bdir) if bdir is not None else None,
        "lines_patched": lines_patched,
        "turns_unlocked": turns_unlocked,
    }


def lock_thread(tid: str, message: str, code: str = "misalignmentPolicyViolation", set_phase=lambda _: None, allow_real: bool = False, auto_backup: bool = True) -> dict:
    _check_lock_writable(allow_real=allow_real)
    msg = (message or "").strip()
    if not msg:
        raise ValueError("Укажите причину блокировки чата.")
    camel_code, snake_code = _code_variants(code)

    files = thread_files(tid)
    if not files:
        raise FileNotFoundError(f"Тред {tid} не найден")
    p = files[-1]

    target_tc = None
    last_ordinal = None
    last_turn_id = None
    for idx, (offset, raw) in enumerate(_reverse_lines(p)):
        if idx >= 80:
            break
        try:
            obj = json.loads(raw)
        except Exception:
            continue
        if last_ordinal is None and isinstance(obj.get("ordinal"), int):
            last_ordinal = obj["ordinal"]
        pl = obj.get("payload") or {}
        if last_turn_id is None and isinstance(pl, dict):
            last_turn_id = pl.get("turn_id") or pl.get("root_turn_id")
        if target_tc is None and obj.get("type") == "event_msg" and pl.get("type") == "task_complete":
            target_tc = (offset, obj)
            break

    tail_off = target_tc[0] if target_tc else p.stat().st_size
    lines_patched = 0
    turns_locked = 0

    with _open_locked_rollout(p) as fh:
        orig_size = p.stat().st_size
        bdir = None
        if auto_backup:
            set_phase("backup")
            bdir = _create_lock_backup(tid, "k1tty_lock", p, tail_off, fh=fh)
        set_phase("lock")

        if target_tc is not None:
            offset, obj = target_tc
            pl = obj.get("payload") or {}
            pl["error"] = {"message": msg, "codex_error_info": snake_code}
            pl["last_agent_message"] = None
            obj["payload"] = pl

            fh.seek(offset)
            disk_line = fh.readline()
            ending = b"\r\n" if disk_line.endswith(b"\r\n") else b"\n"
            new_core = json.dumps(obj, ensure_ascii=False, separators=(",", ":")).encode("utf-8")
            pad_needed = len(disk_line) - len(new_core) - len(ending)
            if pad_needed >= 0:
                fh.seek(offset)
                fh.write(new_core + (b" " * pad_needed) + ending)
            else:
                rest = fh.read()
                fh.seek(offset)
                fh.write(new_core + ending + rest)
                fh.truncate()
            lines_patched = 1
        else:
            fh.seek(0, 2)
            if orig_size > 0:
                fh.seek(orig_size - 1)
                needs_nl = fh.read(1) != b"\n"
                fh.seek(0, 2)
                if needs_nl:
                    fh.write(b"\n")
            now_iso = datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%S.%f")[:-3] + "Z"
            now_ts = int(datetime.now(timezone.utc).timestamp())
            tc_turn_id = last_turn_id or tid
            new_ev = {
                "timestamp": now_iso,
                "type": "event_msg",
                "payload": {
                    "type": "task_complete",
                    "root_turn_id": tc_turn_id,
                    "turn_id": tc_turn_id,
                    "last_agent_message": None,
                    "error": {"message": msg, "codex_error_info": snake_code},
                    "started_at": now_ts,
                    "completed_at": now_ts,
                    "duration_ms": 0,
                },
            }
            if last_ordinal is not None:
                new_ev["ordinal"] = last_ordinal + 1
            fh.write(json.dumps(new_ev, ensure_ascii=False, separators=(",", ":")).encode("utf-8") + b"\n")
            lines_patched = 1
        _sync(fh)

        fh.seek(0, 2)
        new_size = fh.tell()
        delta_bytes = new_size - orig_size

        if PROJECTION_DB.is_file():
            err_json = json.dumps({
                "message": msg,
                "codexErrorInfo": camel_code,
                "additionalDetails": None,
                "misalignment": None,
            }, ensure_ascii=False)
            con = sqlite3.connect(str(PROJECTION_DB), timeout=5.0)
            try:
                con.execute("BEGIN IMMEDIATE")
                latest = con.execute(
                    "SELECT rowid, rollout_end_byte_offset FROM thread_turns WHERE thread_id = ? ORDER BY rollout_ordinal DESC LIMIT 1",
                    (tid,),
                ).fetchone()
                if latest:
                    rowid, end_off = latest
                    new_end_off = (end_off + delta_bytes) if (end_off is not None and delta_bytes != 0) else end_off
                    con.execute(
                        "UPDATE thread_turns SET status = 'failed', error_json = ?, rollout_end_byte_offset = ? WHERE rowid = ?",
                        (err_json, new_end_off, rowid),
                    )
                    turns_locked = 1
                if delta_bytes != 0 and len(files) == 1:
                    con.execute(
                        "UPDATE thread_history_projection_state SET next_rollout_byte_offset = ? WHERE thread_id = ?",
                        (new_size, tid),
                    )
                con.commit()
            except Exception:
                if con.in_transaction:
                    con.execute("ROLLBACK")
                raise
            finally:
                con.close()

    _FILE_CACHE.clear()
    _LIST_CACHE.clear()
    set_phase("done")
    return {
        "tid": tid,
        "locked": True,
        "code": camel_code,
        "message": msg,
        "backup": str(bdir) if bdir is not None else None,
        "lines_patched": lines_patched,
        "turns_locked": turns_locked,
    }
