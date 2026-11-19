from __future__ import annotations

import hashlib
import json
import os
import re
import shutil
from datetime import datetime
from pathlib import Path

from claude_io import lock_files, atomic_write_text, win_long_path

def _home() -> Path:
    return Path(os.environ.get("CLAUDE_HOME") or (Path.home() / ".claude"))

CLAUDE_HOME = _home()
PROJECTS_DIR = CLAUDE_HOME / "projects"
HISTORY_JSONL = CLAUDE_HOME / "history.jsonl"
BACKUPS_DIR = CLAUDE_HOME.parent / "claude_backups"

def set_home(p: Path | str):
    global CLAUDE_HOME, PROJECTS_DIR, HISTORY_JSONL, BACKUPS_DIR
    CLAUDE_HOME = Path(p)
    PROJECTS_DIR = CLAUDE_HOME / "projects"
    HISTORY_JSONL = CLAUDE_HOME / "history.jsonl"
    BACKUPS_DIR = CLAUDE_HOME.parent / "claude_backups"
    _FILE_CACHE.clear()

def _test_mode() -> bool:
    name = CLAUDE_HOME.resolve().name.lower()
    return name in ("sandbox", "claude_sandbox") or "test" in name


def claude_running() -> list[str]:
    running = []
    if os.name != "nt":
        return running
    try:
        import subprocess
        out = subprocess.run(
            ["tasklist", "/FO", "CSV", "/NH"],
            capture_output=True, text=True, errors="replace", timeout=15
        ).stdout
        seen = set()
        for line in out.splitlines():
            low = line.lower()
            if "claude" in low and '"' in line:
                parts = [p.strip('"') for p in line.split('","')]
                if parts and parts[0] not in seen:
                    seen.add(parts[0])
                    pid = parts[1] if len(parts) > 1 else "?"
                    running.append(f"{parts[0]} (PID {pid})")
    except Exception:
        pass
    return running


_FILE_CACHE: dict[str, dict] = {}


def _decode_project_slug(slug: str) -> str:
    if not slug:
        return "(неизвестно)"
    if re.match(r"^[a-zA-Z]--", slug):
        drive = slug[0].upper() + ":"
        rest = slug[3:].replace("-", "\\")
        return f"{drive}\\{rest}"
    return slug.replace("-", "/")


def _extract_title_from_file(path: Path) -> tuple[str, str]:
    title = ""
    cwd = ""
    try:
        with open(win_long_path(path), "r", encoding="utf-8", errors="replace") as f:
            for i, line in enumerate(f):
                if i > 40:
                    break
                try:
                    d = json.loads(line)
                except Exception:
                    continue
                if not cwd and d.get("cwd"):
                    cwd = d.get("cwd")
                if not title and d.get("type") == "user":
                    msg = d.get("message") or {}
                    content = msg.get("content")
                    txt = ""
                    if isinstance(content, str):
                        txt = content
                    elif isinstance(content, list):
                        for b in content:
                            if isinstance(b, dict) and b.get("type") == "text":
                                b_txt = b.get("text", "")
                                if not b_txt.startswith("<system-reminder>"):
                                    txt = b_txt
                                    break
                    if txt:
                        clean = txt.strip()
                        if not clean.startswith(("<local-command-", "<system-reminder>", "<command-name>", "Your response above")):
                            title = clean.split("\n")[0][:100]
    except Exception:
        pass
    return title, cwd


def _iter_project_jsonl():
    root = win_long_path(PROJECTS_DIR)
    if not os.path.isdir(root):
        return
    for dirpath, _dirnames, filenames in os.walk(root):
        for fn in filenames:
            if not fn.endswith(".jsonl"):
                continue
            if fn.startswith(".") or fn.endswith(".tmp") or ".tmp." in fn:
                continue
            full = os.path.join(dirpath, fn)
            clean = full[4:] if full.startswith("\\\\?\\") and not full.startswith("\\\\?\\UNC\\") else full
            yield Path(clean)


def list_threads() -> list[dict]:
    threads = []
    if not PROJECTS_DIR.exists():
        return threads

    history_titles: dict[str, str] = {}
    if HISTORY_JSONL.exists():
        try:
            with open(HISTORY_JSONL, "r", encoding="utf-8", errors="replace") as f:
                for line in f:
                    try:
                        d = json.loads(line)
                        sid = d.get("sessionId")
                        disp = d.get("display")
                        if sid and disp and sid not in history_titles:
                            clean = disp.strip().split("\n")[0][:100]
                            if clean and not clean.startswith("/"):
                                history_titles[sid] = clean
                    except Exception:
                        continue
        except Exception:
            pass

    for p in _iter_project_jsonl():
        st = os.stat(win_long_path(p))
        tid = p.stem
        slug = p.parent.name

        cache_entry = _FILE_CACHE.get(str(p))
        if cache_entry and cache_entry.get("mtime_ns") == st.st_mtime_ns:
            threads.append(cache_entry["data"])
            continue

        title = None
        custom = win_long_path(p.parent / tid / "custom-title.json")
        if os.path.exists(custom):
            try:
                with open(custom, "r", encoding="utf-8") as cf:
                    title = json.load(cf).get("customTitle")
            except Exception:
                pass

        file_title, cwd = _extract_title_from_file(p)
        if not title:
            title = history_titles.get(tid) or file_title or f"Сессия {tid[:8]}"

        project_display = cwd or _decode_project_slug(slug)

        item = {
            "id": tid,
            "title": title,
            "project": project_display,
            "project_slug": slug,
            "file": str(p),
            "size": st.st_size,
            "mtime": datetime.fromtimestamp(st.st_mtime),
        }
        _FILE_CACHE[str(p)] = {"mtime_ns": st.st_mtime_ns, "data": item}
        threads.append(item)

    threads.sort(key=lambda x: x["mtime"], reverse=True)
    return threads


def session_file(tid: str) -> Path:
    if not PROJECTS_DIR.exists():
        raise FileNotFoundError(f"Каталог проектов Claude не найден: {PROJECTS_DIR}")
    want = f"{tid}.jsonl"
    for p in _iter_project_jsonl():
        if p.name == want and os.path.isfile(win_long_path(p)):
            return p
    raise FileNotFoundError(f"Файл сессии {tid} не найден")


def session_title(tid: str) -> str:
    for t in list_threads():
        if t["id"] == tid:
            return t["title"]
    return tid


def _clean_human_text(content) -> tuple[str, str | None]:
    if isinstance(content, str):
        if content.startswith("<system-reminder>"):
            m = re.match(r"^<system-reminder>[\s\S]*?</system-reminder>\s*", content)
            if m:
                reminder = m.group(0).strip()
                human = content[m.end():].strip()
                return human or reminder, reminder if human else None
        return content, None

    if isinstance(content, list):
        human_parts = []
        reminder_parts = []
        for b in content:
            if not isinstance(b, dict):
                continue
            if b.get("type") == "text":
                t = b.get("text", "")
                if t.startswith("<system-reminder>"):
                    reminder_parts.append(t)
                else:
                    human_parts.append(t)
        human = "\n\n".join(human_parts).strip()
        reminder = "\n\n".join(reminder_parts).strip() or None
        return human or (reminder or ""), reminder

    return "", None


def _history_item(o: dict, offset: int, line_no: int) -> dict | None:
    t = o.get("type")
    uuid = o.get("uuid")
    parent_uuid = o.get("parentUuid")
    ts = o.get("timestamp") or ""
    key = f"b{offset}"

    base = {
        "key": key,
        "uuid": uuid,
        "parent_uuid": parent_uuid,
        "offset": offset,
        "line": line_no,
        "ts": ts,
    }

    if t == "user":
        msg = o.get("message") or {}
        content = msg.get("content")

        if isinstance(content, list) and any(isinstance(b, dict) and b.get("type") == "tool_result" for b in content):
            results = []
            tool_id = ""
            is_err = False
            for b in content:
                if isinstance(b, dict) and b.get("type") == "tool_result":
                    tool_id = b.get("tool_use_id") or tool_id
                    if b.get("is_error"):
                        is_err = True
                    c = b.get("content")
                    if isinstance(c, str):
                        results.append(c)
                    elif isinstance(c, list):
                        for sub in c:
                            if isinstance(sub, dict) and sub.get("text"):
                                results.append(sub["text"])
                            elif isinstance(sub, str):
                                results.append(sub)
            text_out = "\n".join(results)
            base.update(
                kind="tool",
                subtype="tool_result",
                label=f"Вывод ({tool_id})" if tool_id else "Результат инструмента",
                tool=tool_id or "tool_result",
                text=text_out,
                status="error" if is_err else "ok",
            )
            return base

        raw_text = content if isinstance(content, str) else ""
        if "stopped by a safety classifier" in raw_text or raw_text.startswith("Your response above was stopped"):
            base.update(
                kind="system",
                subtype="safeguard",
                label="Уведомление безопасности (Safety Classifier)",
                text=raw_text,
            )
            return base

        if raw_text.startswith(("<local-command-caveat>", "<command-name>", "<local-command-stdout>")):
            base.update(
                kind="system",
                subtype="command",
                label="Локальная команда",
                text=raw_text,
            )
            return base

        human_text, reminder = _clean_human_text(content)
        if not human_text and reminder:
            base.update(
                kind="system",
                subtype="reminder",
                label="Системный контекст",
                text=reminder,
            )
            return base

        base.update(
            kind="user",
            subtype="prompt",
            label="Вы",
            text=human_text,
            reminder=reminder,
        )
        return base

    elif t == "assistant":
        msg = o.get("message") or {}
        content = msg.get("content")
        model = msg.get("model", "Claude")

        if isinstance(content, list):
            for b in content:
                if not isinstance(b, dict):
                    continue
                b_type = b.get("type")
                if b_type == "text":
                    base.update(
                        kind="assistant",
                        subtype="reply",
                        label="Claude",
                        model=model,
                        text=b.get("text", ""),
                    )
                    return base
                elif b_type == "thinking":
                    base.update(
                        kind="tool",
                        subtype="thinking",
                        label="Размышления (thinking)",
                        tool="thinking",
                        text=b.get("thinking", ""),
                    )
                    return base
                elif b_type == "tool_use":
                    name = b.get("name", "Tool")
                    inp = b.get("input")
                    if isinstance(inp, dict) and "command" in inp and isinstance(inp["command"], str):
                        cmd_text = inp["command"]
                    elif isinstance(inp, (dict, list)):
                        cmd_text = json.dumps(inp, ensure_ascii=False, indent=2)
                    else:
                        cmd_text = str(inp or "")
                    base.update(
                        kind="tool",
                        subtype="tool_use",
                        label=f"Команда: {name}",
                        tool=name,
                        tool_id=b.get("id"),
                        text=cmd_text,
                    )
                    return base
        elif isinstance(content, str):
            base.update(
                kind="assistant",
                subtype="reply",
                label="Claude",
                model=model,
                text=content,
            )
            return base
        return None

    elif t == "system":
        raw_msg = str(o.get("content") or o.get("message") or "")
        is_safeguard = "safeguard" in raw_msg.lower() or o.get("subtype") == "informational"
        base.update(
            kind="system",
            subtype="safeguard" if is_safeguard else "notice",
            label="Уведомление Claude (Notice)" if is_safeguard else "Система",
            text=raw_msg,
        )
        return base

    return None


def _reverse_lines(path: Path, end: int | None = None, block_size: int = 65536):
    with open(win_long_path(path), "rb") as fh:
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


def read_history_page(tid: str, limit: int = 100, before: str | None = None, include_tools: bool = True):
    path = session_file(tid)
    end = None
    if before:
        if not before.startswith("b"):
            raise ValueError("Некорректный курсор истории")
        end = int(before[1:])

    items = []
    messages = 0
    line_no_approx = 0

    for offset, raw in _reverse_lines(path, end):
        line_no_approx += 1
        if not include_tools and b'"text"' not in raw and b'"user"' not in raw:
            continue
        try:
            o = json.loads(raw.decode("utf-8", "replace"))
        except Exception:
            continue

        item = _history_item(o, offset, line_no_approx)
        if item is None:
            continue
        if not include_tools and item["kind"] not in ("user", "assistant"):
            continue

        items.append(item)
        if item["kind"] in ("user", "assistant"):
            messages += 1

        if messages >= limit or len(items) >= 2000:
            cursor = f"b{offset}" if offset > 0 else None
            return list(reversed(items)), cursor

    return list(reversed(items)), None


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


def revision(tid: str) -> str:
    path = session_file(tid)
    h = hashlib.sha256()
    with open(win_long_path(path), "rb") as f:
        for chunk in iter(lambda: f.read(1024 * 1024), b""):
            h.update(chunk)
    return h.hexdigest()


def check_revision(tid: str, expected: str):
    if not expected or revision(tid) != expected:
        raise RuntimeError("Сессия изменилась после предпросмотра. Обновите страницу и повторите действие.")


def _read_at_offset(path: Path, offset: int) -> dict:
    if offset < 0:
        raise ValueError("Недопустимый ключ сообщения")
    with open(win_long_path(path), "rb") as f:
        if offset > 0:
            f.seek(offset - 1)
            if f.read(1) != b"\n":
                raise ValueError(f"Ключ b{offset} устарел (не граница строки)")
        else:
            f.seek(0)
        raw = f.readline()
        if not raw:
            raise ValueError(f"Запись по смещению b{offset} не найдена")
        return json.loads(raw.decode("utf-8", "replace"))


def _find_line_by_key(path: Path, key: str) -> tuple[int, int, dict]:
    target_offset = int(key[1:]) if key.startswith("b") else None
    target_uuid = key if not key.startswith("b") else None

    with open(win_long_path(path), "rb") as f:
        if target_offset is not None:
            if target_offset < 0:
                raise ValueError("Недопустимый ключ сообщения")
            rem, line_idx, last_byte = target_offset, 0, b"\n"
            while rem > 0:
                chunk = f.read(min(rem, 1024 * 1024))
                if not chunk:
                    raise ValueError(f"Запись с ключом {key} не найдена в сессии")
                line_idx += chunk.count(b"\n")
                last_byte = chunk[-1:]
                rem -= len(chunk)
            if target_offset > 0 and last_byte != b"\n":
                raise ValueError(f"Ключ {key} устарел (не граница строки)")
            raw = f.readline()
            if not raw:
                raise ValueError(f"Запись с ключом {key} не найдена в сессии")
            return line_idx, target_offset, json.loads(raw.decode("utf-8", "replace"))

        needle = target_uuid.encode("utf-8") if target_uuid else b""
        line_idx = 0
        while True:
            cur_offset = f.tell()
            raw = f.readline()
            if not raw:
                break
            if needle in raw:
                try:
                    o = json.loads(raw.decode("utf-8", "replace"))
                    if o.get("uuid") == target_uuid:
                        return line_idx, cur_offset, o
                except Exception:
                    pass
            line_idx += 1

    raise ValueError(f"Запись с ключом {key} не найдена в сессии")


def _get_item_text_and_kind(obj: dict) -> tuple[str, str]:
    t = obj.get("type")
    msg = obj.get("message") or {}
    content = msg.get("content") or obj.get("content")

    if t == "system":
        if isinstance(content, str):
            return content, "Системное уведомление"
        return json.dumps(content, ensure_ascii=False, indent=2), "Системное уведомление"

    if t == "user":
        if isinstance(content, list):
            for b in content:
                if isinstance(b, dict) and b.get("type") == "tool_result":
                    c = b.get("content")
                    if isinstance(c, str):
                        return c, "Вывод инструмента (tool_result)"
                    elif isinstance(c, list):
                        parts = [sub.get("text", str(sub)) if isinstance(sub, dict) else str(sub) for sub in c]
                        return "\n".join(parts), "Вывод инструмента (tool_result)"
            human, reminder = _clean_human_text(content)
            return human or (reminder or ""), "Сообщение пользователя"
        elif isinstance(content, str):
            if "stopped by a safety classifier" in content or content.startswith("Your response above"):
                return content, "Уведомление безопасности (Safeguard)"
            if content.startswith(("<local-command-", "<system-reminder>")):
                return content, "Системная команда / контекст"
            return content, "Сообщение пользователя"

    if t == "assistant":
        if isinstance(content, list):
            for b in content:
                if not isinstance(b, dict):
                    continue
                b_type = b.get("type")
                if b_type == "text":
                    return b.get("text", ""), "Ответ Claude"
                elif b_type == "thinking":
                    return b.get("thinking", ""), "Размышления модели (thinking)"
                elif b_type == "tool_use":
                    inp = b.get("input")
                    name = b.get("name", "Tool")
                    if isinstance(inp, dict) and "command" in inp and isinstance(inp["command"], str):
                        return inp["command"], f"Команда инструмента ({name})"
                    elif isinstance(inp, (dict, list)):
                        return json.dumps(inp, ensure_ascii=False, indent=2), f"Параметры инструмента ({name})"
                    return str(inp or ""), f"Команда ({name})"
        elif isinstance(content, str):
            return content, "Ответ Claude"

    return str(content or ""), "Запись"


def _set_item_text(obj: dict, new_text: str):
    t = obj.get("type")
    msg = obj.get("message")

    if t == "system":
        obj["content"] = new_text
        return

    if t == "user" and isinstance(msg, dict):
        content = msg.get("content")
        if isinstance(content, list):
            updated = False
            for b in content:
                if isinstance(b, dict):
                    if b.get("type") == "tool_result":
                        b["content"] = new_text
                        updated = True
                        break
                    elif b.get("type") == "text":
                        if not b.get("text", "").startswith("<system-reminder>"):
                            b["text"] = new_text
                            updated = True
                            break
            if not updated and content:
                if isinstance(content[0], dict) and "text" in content[0]:
                    content[0]["text"] = new_text
                else:
                    content.append({"type": "text", "text": new_text})
        elif isinstance(content, str):
            msg["content"] = new_text
        else:
            msg["content"] = new_text
        return

    if t == "assistant" and isinstance(msg, dict):
        content = msg.get("content")
        if isinstance(content, list):
            for b in content:
                if not isinstance(b, dict):
                    continue
                b_type = b.get("type")
                if b_type == "text":
                    b["text"] = new_text
                    return
                elif b_type == "thinking":
                    b["thinking"] = new_text
                    return
                elif b_type == "tool_use":
                    inp = b.get("input")
                    if isinstance(inp, dict) and "command" in inp and not new_text.strip().startswith("{"):
                        inp["command"] = new_text
                        b["input"] = inp
                    else:
                        try:
                            b["input"] = json.loads(new_text)
                        except Exception:
                            if isinstance(inp, dict) and len(inp) == 1:
                                k = list(inp.keys())[0]
                                inp[k] = new_text
                                b["input"] = inp
                            else:
                                b["input"] = {"command": new_text}
                    return
        elif isinstance(content, str):
            msg["content"] = new_text
            return
        msg["content"] = new_text


def edit_message(tid: str, key: str, new_text: str, preview_only: bool = True) -> dict:
    path = session_file(tid)
    if preview_only and isinstance(key, str) and key.startswith("b"):
        obj = _read_at_offset(path, int(key[1:]))
        old_text, kind_label = _get_item_text_and_kind(obj)
        return {"tid": tid, "key": key, "old": old_text, "kind": kind_label}
    line_idx, offset, obj = _find_line_by_key(path, key)
    old_text, kind_label = _get_item_text_and_kind(obj)

    if preview_only:
        return {"tid": tid, "key": key, "old": old_text, "kind": kind_label}

    if not new_text or not new_text.strip():
        raise ValueError("Новый текст не может быть пустым")

    with open(win_long_path(path), "r", encoding="utf-8", errors="replace") as f:
        lines = f.read().splitlines()

    target_obj = json.loads(lines[line_idx])
    _set_item_text(target_obj, new_text)
    lines[line_idx] = json.dumps(target_obj, ensure_ascii=False)

    with lock_files([path]):
        pass
    atomic_write_text(path, "\n".join(lines) + "\n")

    _FILE_CACHE.clear()
    return {"tid": tid, "key": key, "old": old_text, "updated": True}


def delete_single(tid: str, key: str) -> dict:
    path = session_file(tid)
    target_line_idx, target_offset, target_obj = _find_line_by_key(path, key)
    target_uuid = target_obj.get("uuid")
    parent_uuid = target_obj.get("parentUuid")

    with open(win_long_path(path), "r", encoding="utf-8", errors="replace") as f:
        lines = f.read().splitlines()

    old_text, _ = _get_item_text_and_kind(target_obj)
    removed_texts = [old_text] if old_text else []

    new_lines = []
    for idx, l in enumerate(lines):
        if idx == target_line_idx:
            continue
        if target_uuid and target_uuid in l:
            try:
                row = json.loads(l)
                if row.get("parentUuid") == target_uuid:
                    row["parentUuid"] = parent_uuid
                    l = json.dumps(row, ensure_ascii=False)
            except Exception:
                pass
        new_lines.append(l)

    with lock_files([path]):
        pass
    atomic_write_text(path, "\n".join(new_lines) + "\n")
    _FILE_CACHE.clear()

    leaks = check_leak(tid, removed_texts)
    return {
        "thread": tid,
        "deleted_key": key,
        "deleted_uuid": target_uuid,
        "leaks": leaks,
    }


def preview_cut_fast(tid: str, key: str, mode: str, *, delete_selected: bool = False) -> dict:
    if mode not in ("head", "tail") or not key:
        raise ValueError("Недопустимый ключ или режим обрезки")

    path = session_file(tid)
    target_line_idx, target_offset, target_obj = _find_line_by_key(path, key)

    with open(win_long_path(path), "r", encoding="utf-8", errors="replace") as f:
        all_lines = f.read().splitlines()

    if mode == "tail":
        cut_lines = all_lines[target_line_idx:]
    else:
        cut_lines = all_lines[:target_line_idx]

    preview_msgs = []
    removed_msgs_count = 0
    removed_bytes = 0

    for idx, line in enumerate(cut_lines):
        line_bytes = len(line.encode("utf-8")) + 1
        removed_bytes += line_bytes
        try:
            o = json.loads(line)
        except Exception:
            continue
        item = _history_item(o, 0, idx)
        if item:
            removed_msgs_count += 1
            if len(preview_msgs) < 12:
                preview_msgs.append({
                    "key": item["key"],
                    "label": item["label"],
                    "text": (item["text"] or "").replace("\n", " ")[:120],
                })

    return {
        "thread": tid,
        "mode": mode,
        "rollout_lines": len(cut_lines),
        "rollout_files": 0,
        "bytes": removed_bytes,
        "will_delete": {
            "steps_preview_count": removed_msgs_count,
            "steps_preview": preview_msgs,
        }
    }


def cut(tid: str, key: str, preview_only: bool = False, mode: str = "tail", set_phase=None) -> dict:
    path = session_file(tid)
    target_line_idx, target_offset, target_obj = _find_line_by_key(path, key)

    with open(win_long_path(path), "r", encoding="utf-8", errors="replace") as f:
        lines = f.read().splitlines()

    if mode == "tail":
        new_lines = lines[:target_line_idx]
        cut_lines = lines[target_line_idx:]
    else:
        preamble = []
        for i in range(min(target_line_idx, 5)):
            try:
                row = json.loads(lines[i])
                if not row.get("uuid") and not row.get("parentUuid") and row.get("type") in ("mode", "permission-mode", "atis-latch"):
                    preamble.append(lines[i])
            except Exception:
                pass

        target_dict = json.loads(lines[target_line_idx])
        if "parentUuid" in target_dict:
            target_dict["parentUuid"] = None
        lines[target_line_idx] = json.dumps(target_dict, ensure_ascii=False)

        new_lines = preamble + lines[target_line_idx:]
        cut_lines = lines[len(preamble):target_line_idx]

    removed_texts = []
    for line in cut_lines:
        try:
            o = json.loads(line)
            item = _history_item(o, 0, 0)
            if item and item.get("text"):
                removed_texts.append(item["text"])
        except Exception:
            pass

    if preview_only:
        return {
            "thread": tid, "mode": mode,
            "rollout_lines": len(cut_lines),
            "bytes": sum(len(l.encode("utf-8")) + 1 for l in cut_lines),
        }

    with lock_files([path]):
        pass
    atomic_write_text(path, "\n".join(new_lines) + "\n")

    _FILE_CACHE.clear()

    if set_phase:
        set_phase("leak")
    leaks = check_leak(tid, removed_texts)

    return {
        "thread": tid,
        "mode": mode,
        "rollout_lines": len(cut_lines),
        "bytes": sum(len(l.encode("utf-8")) + 1 for l in cut_lines),
        "leaks": leaks,
    }


def full_backup(tid: str, tag: str = "web") -> Path:
    path = session_file(tid)
    ts = datetime.now().strftime("%Y%m%d_%H%M%S_%f")[:19]
    bdir = BACKUPS_DIR / tid / f"{ts}_{tag}"
    os.makedirs(win_long_path(bdir), exist_ok=True)

    dst_jsonl = bdir / path.name
    shutil.copy2(win_long_path(path), win_long_path(dst_jsonl))

    custom = path.parent / tid / "custom-title.json"
    lp_custom = win_long_path(custom)
    if os.path.exists(lp_custom):
        dst_custom = bdir / "custom-title.json"
        shutil.copy2(lp_custom, win_long_path(dst_custom))

    lp_dst = win_long_path(dst_jsonl)
    with open(lp_dst, "rb") as f:
        raw_bytes = f.read()
    sha = hashlib.sha256(raw_bytes).hexdigest()
    lines_count = raw_bytes.count(b"\n") + (1 if raw_bytes and not raw_bytes.endswith(b"\n") else 0)

    meta = {
        "tid": tid,
        "tag": tag,
        "created": ts,
        "file_name": path.name,
        "project_slug": path.parent.name,
        "sha256": sha,
        "lines": lines_count,
        "size": len(raw_bytes),
    }
    with open(win_long_path(bdir / "backup.meta.json"), "w", encoding="utf-8") as mf:
        json.dump(meta, mf, ensure_ascii=False, indent=2)
    return bdir


def restore_backup(bdir: Path | str) -> dict:
    bdir = Path(bdir)
    meta_path = win_long_path(bdir / "backup.meta.json")
    if not os.path.exists(meta_path):
        raise ValueError("Файл backup.meta.json отсутствует в резервной копии")

    with open(meta_path, "r", encoding="utf-8") as mf:
        meta = json.load(mf)
    tid = meta["tid"]
    file_name = meta["file_name"]
    src_jsonl = bdir / file_name
    lp_src = win_long_path(src_jsonl)
    if not os.path.exists(lp_src):
        raise ValueError(f"Файл {file_name} отсутствует в резервной копии")

    with open(lp_src, "rb") as f:
        cur_sha = hashlib.sha256(f.read()).hexdigest()
    if cur_sha != meta.get("sha256"):
        raise ValueError("Контрольная сумма файла в резервной копии не совпадает")

    slug = meta.get("project_slug")
    target_dir = PROJECTS_DIR / slug if slug else None
    if target_dir:
        os.makedirs(win_long_path(target_dir), exist_ok=True)
        target_path = target_dir / file_name
    else:
        target_path = session_file(tid)

    lp_target = win_long_path(target_path)
    if os.path.exists(lp_target):
        with lock_files([target_path]):
            pass
    shutil.copy2(lp_src, lp_target)
    custom_src = win_long_path(bdir / "custom-title.json")
    if os.path.exists(custom_src):
        custom_dst_dir = target_path.parent / tid
        os.makedirs(win_long_path(custom_dst_dir), exist_ok=True)
        shutil.copy2(custom_src, win_long_path(custom_dst_dir / "custom-title.json"))

    _FILE_CACHE.clear()
    return {"restored": str(bdir), "target": str(target_path)}


def check_leak(tid: str, removed_texts: list[str]) -> list[tuple[str, str]]:
    leaks = []
    frags = []
    for t in removed_texts:
        for s in (t or "").split("\n"):
            s_clean = re.sub(r"[\r\n\t]+", " ", s).strip()
            if len(s_clean) >= 40:
                frags.append(s_clean[:80])
                if len(frags) >= 15:
                    break
        if len(frags) >= 25:
            break

    if not frags:
        return leaks

    targets = []
    try:
        p = session_file(tid)
        if os.path.exists(win_long_path(p)):
            targets.append(p)
    except Exception:
        pass

    if HISTORY_JSONL.exists():
        targets.append(HISTORY_JSONL)

    for p in targets:
        try:
            with open(win_long_path(p), "r", encoding="utf-8", errors="replace") as f:
                data = f.read()
            for frag in frags:
                if frag in data:
                    leaks.append((str(p.name), frag))
        except Exception:
            pass

    return leaks
