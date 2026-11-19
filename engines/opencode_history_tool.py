from __future__ import annotations

import json
import os
import sqlite3
import time
from datetime import datetime, timezone
from pathlib import Path

DATA_DIR = Path(
    os.environ.get("OPENCODE_DATA_DIR", "")
    or (Path.home() / ".local" / "share" / "opencode")
)
DB_PATH = DATA_DIR / "opencode.db"
BACKUP_ROOT = Path(
    os.environ.get("OPENCODE_BACKUP_ROOT", "")
    or (Path(__file__).resolve().parent.parent / "backups" / "opencode")
)

BACKUP_TABLES = (
    ("session_v2", "id"),
    ("session_message", "session_id"),
    ("session_part", "session_id"),
    ("session_inbox", "session_id"),
    ("instruction_state", "session_id"),
    ("instruction_entry", "session_id"),
    ("event_sequence", "aggregate_id"),
    ("blob_ref", "session_id"),
    ("schedule", "session_id"),
)


def connect_ro(db: Path | None = None) -> sqlite3.Connection:
    target = Path(db or DB_PATH)
    con = None
    try:
        con = sqlite3.connect(f"file:{target}?mode=ro", uri=True, timeout=10)
        con.execute("select 1 from sqlite_master limit 1").fetchone()
    except sqlite3.Error:
        if con is not None:
            con.close()
        con = sqlite3.connect(f"file:{target}?mode=ro&immutable=1", uri=True)
    return con


def connect_rw(db: Path | None = None) -> sqlite3.Connection:
    target = Path(db or DB_PATH)
    con = sqlite3.connect(str(target), timeout=15)
    con.execute("PRAGMA foreign_keys = ON")
    con.execute("PRAGMA busy_timeout = 10000")
    return con


def db_write_probe(db: Path | None = None) -> tuple[bool, str]:
    target = Path(db or DB_PATH)
    if not target.is_file():
        return False, f"Файл БД не найден: {target}"
    try:
        con = sqlite3.connect(str(target), timeout=2.0)
        try:
            con.execute("BEGIN IMMEDIATE")
            con.execute("ROLLBACK")
        finally:
            con.close()
        return True, ""
    except Exception as e:
        return False, str(e)


def _ms_to_iso(ms) -> str:
    if not ms:
        return ""
    try:
        return datetime.fromtimestamp(float(ms) / 1000.0, tz=timezone.utc).isoformat()
    except Exception:
        return ""


def _tool_text_and_mode(part: dict) -> tuple[str, str]:
    st = part.get("state") or {}
    if not isinstance(st, dict):
        return "", "none"
    content = st.get("content")
    if isinstance(content, list) and content:
        chunks = []
        for c in content:
            if isinstance(c, dict) and c.get("type") == "text":
                chunks.append(str(c.get("text") or ""))
            elif isinstance(c, str):
                chunks.append(c)
        if chunks:
            return "\n".join(chunks), "state_content"
    if isinstance(st.get("output"), str):
        return st["output"], "state_output"
    err = st.get("error")
    if isinstance(err, dict) and isinstance(err.get("message"), str):
        return err["message"], "state_error_msg"
    if isinstance(err, str):
        return err, "state_error_str"
    if "input" in st:
        try:
            return json.dumps(st["input"], ensure_ascii=False, indent=2), "state_input"
        except Exception:
            return str(st["input"]), "state_input"
    return "", "none"


def _format_tool_display(part: dict) -> str:
    st = part.get("state") or {}
    lines: list[str] = []
    inp = st.get("input") if isinstance(st, dict) else None
    if isinstance(inp, dict) and inp:
        if len(inp) == 1 and "command" in inp:
            lines.append("$ " + str(inp["command"]))
        else:
            try:
                lines.append(json.dumps(inp, ensure_ascii=False, indent=2))
            except Exception:
                lines.append(str(inp))
    body, mode = _tool_text_and_mode(part)
    if body and mode != "state_input":
        if lines:
            lines.append("")
        lines.append(body)
    return "\n".join(lines).strip()


def _first_user_preview(con: sqlite3.Connection, sid: str) -> str:
    row = con.execute(
        "SELECT data FROM session_message WHERE session_id = ? AND type = 'user' ORDER BY seq ASC LIMIT 1",
        (sid,),
    ).fetchone()
    if not row:
        return ""
    try:
        d = json.loads(row[0])
        txt = (d.get("text") or (d.get("metadata") or {}).get("displayText") or "").strip()
        txt = " ".join(txt.split())
        return txt[:72]
    except Exception:
        return ""


def list_sessions(db: Path | None = None) -> list[dict]:
    target = Path(db or DB_PATH)
    if not target.is_file():
        return []
    con = connect_ro(target)
    try:
        stats: dict[str, tuple[int, int]] = {}
        for sid, cnt, total_len in con.execute(
            "SELECT session_id, COUNT(*), COALESCE(SUM(LENGTH(data)), 0) FROM session_message GROUP BY session_id"
        ):
            stats[sid] = (int(cnt), int(total_len))

        out: list[dict] = []
        rows = con.execute(
            "SELECT id, parent_id, slug, directory, title, time_created, time_updated, time_archived "
            "FROM session_v2 ORDER BY time_updated DESC"
        ).fetchall()
        for sid, parent_id, slug, directory, title, t_created, t_updated, t_archived in rows:
            cnt, sz = stats.get(sid, (0, 0))
            t_str = (title or "").strip()
            if not t_str:
                t_str = _first_user_preview(con, sid) or (slug or f"Диалог {sid[:10]}")
            ts_ms = t_updated or t_created or 0
            mtime_ts = float(ts_ms) / 1000.0 if ts_ms else 0.0
            out.append({
                "id": sid,
                "parent_id": parent_id,
                "slug": slug or "",
                "directory": directory or "",
                "title": t_str,
                "msg_count": cnt,
                "size": sz,
                "mtime_ts": mtime_ts,
                "mtime": datetime.fromtimestamp(mtime_ts).strftime("%d.%m.%Y %H:%M") if mtime_ts else "",
                "archived": bool(t_archived),
            })
        return out
    finally:
        con.close()


def _items_from_message_rows(rows: list, *, readonly: bool = False) -> list[dict]:
    items: list[dict] = []
    for mid, mtype, seq, t_created, _t_updated, raw in rows:
        try:
            d = json.loads(raw)
        except Exception:
            continue
        ts = _ms_to_iso(t_created)

        if mtype == "user":
            meta = d.get("metadata") if isinstance(d.get("metadata"), dict) else {}
            txt = d.get("text")
            if txt is None:
                txt = meta.get("displayText") or ""
            items.append({
                "key": f"m:{mid}",
                "msg_id": mid,
                "seq": int(seq),
                "kind": "user",
                "subtype": "user",
                "label": "Вы",
                "text": str(txt),
                "ts": ts,
                "tool": None,
                "status": None,
                "code": "",
                "in_db": True,
                "actions": [] if readonly else ["edit", "del_single", "cut_head", "cut_tail"],
            })
        elif mtype == "assistant":
            content = d.get("content") if isinstance(d.get("content"), list) else []
            for pidx, part in enumerate(content):
                if not isinstance(part, dict):
                    continue
                ptype = part.get("type")
                pts = _ms_to_iso((part.get("time") or {}).get("created") or t_created)
                if ptype == "text":
                    txt = str(part.get("text") or "")
                    if not txt.strip() and len(content) > 1:
                        continue
                    items.append({
                        "key": f"p:{mid}:{pidx}",
                        "msg_id": mid,
                        "part_idx": pidx,
                        "seq": int(seq),
                        "kind": "assistant",
                        "subtype": "text",
                        "label": "OpenCode",
                        "text": txt,
                        "ts": pts,
                        "tool": None,
                        "status": None,
                        "code": "",
                        "in_db": True,
                        "actions": [] if readonly else ["edit", "del_single", "cut_tail"],
                    })
                elif ptype == "reasoning":
                    txt = str(part.get("text") or "")
                    items.append({
                        "key": f"p:{mid}:{pidx}",
                        "msg_id": mid,
                        "part_idx": pidx,
                        "seq": int(seq),
                        "kind": "tool",
                        "subtype": "thinking",
                        "label": "Размышления",
                        "text": txt,
                        "ts": pts,
                        "tool": "reasoning",
                        "status": "ok",
                        "code": "",
                        "in_db": True,
                        "actions": [] if readonly else ["edit", "del_single", "cut_tail"],
                    })
                elif ptype == "tool":
                    tname = str(part.get("name") or "tool")
                    st = part.get("state") if isinstance(part.get("state"), dict) else {}
                    tstatus = "error" if st.get("status") == "error" else "ok"
                    items.append({
                        "key": f"p:{mid}:{pidx}",
                        "msg_id": mid,
                        "part_idx": pidx,
                        "seq": int(seq),
                        "kind": "tool",
                        "subtype": "tool_use",
                        "label": tname,
                        "text": _format_tool_display(part),
                        "ts": pts,
                        "tool": tname,
                        "status": tstatus,
                        "code": "",
                        "in_db": True,
                        "actions": [] if readonly else ["edit", "del_single", "cut_tail"],
                    })
            if d.get("error"):
                err = d["error"]
                if isinstance(err, dict):
                    code = str(err.get("type") or "")
                    msg = str(err.get("message") or json.dumps(err, ensure_ascii=False))
                else:
                    code = ""
                    msg = str(err)
                items.append({
                    "key": f"e:{mid}",
                    "msg_id": mid,
                    "seq": int(seq),
                    "kind": "error",
                    "subtype": "error",
                    "label": "Ошибка OpenCode",
                    "text": msg,
                    "ts": ts,
                    "tool": None,
                    "status": "error",
                    "code": code,
                    "in_db": True,
                    "actions": [] if readonly else ["edit", "del_single", "cut_tail"],
                })
        elif mtype == "compaction":
            items.append({
                "key": f"m:{mid}",
                "msg_id": mid,
                "seq": int(seq),
                "kind": "compact",
                "subtype": "compaction",
                "label": "Компакция",
                "text": str(d.get("summary") or ""),
                "ts": ts,
                "tool": None,
                "status": None,
                "code": "",
                "in_db": True,
                "actions": [] if readonly else ["edit", "del_single", "cut_tail"],
            })
        elif mtype in ("system", "synthetic"):
            lbl = str(d.get("description") or ("Система" if mtype == "system" else "Служебное"))
            items.append({
                "key": f"m:{mid}",
                "msg_id": mid,
                "seq": int(seq),
                "kind": "system",
                "subtype": mtype,
                "label": lbl,
                "text": str(d.get("text") or ""),
                "ts": ts,
                "tool": None,
                "status": None,
                "code": "",
                "in_db": True,
                "actions": [] if readonly else ["edit", "del_single", "cut_tail"],
            })
    return items


def read_session_items(sid: str, db: Path | None = None) -> list[dict]:
    target = Path(db or DB_PATH)
    if not target.is_file():
        return []
    con = connect_ro(target)
    try:
        rows = con.execute(
            "SELECT id, type, seq, time_created, time_updated, data "
            "FROM session_message WHERE session_id = ? ORDER BY seq ASC, time_created ASC, id ASC",
            (sid,),
        ).fetchall()
    finally:
        con.close()
    return _items_from_message_rows(rows, readonly=False)


def read_backup_items(bdir: Path) -> tuple[list[dict], str, str]:
    snap_path = Path(bdir) / "session_backup.json"
    if not snap_path.is_file():
        raise FileNotFoundError("Файл session_backup.json не найден в резервной копии.")
    snap = json.loads(snap_path.read_text(encoding="utf-8"))
    sid = str(snap.get("session_id") or Path(bdir).parent.name)
    title = ""
    tables = snap.get("tables") or {}
    sv2 = tables.get("session_v2") or {}
    sv2_cols = sv2.get("columns") or sv2.get("cols") or []
    sv2_rows = sv2.get("rows") or []
    if sv2_rows and "title" in sv2_cols:
        ti = sv2_cols.index("title")
        title = str(sv2_rows[0][ti] or "").strip()
    if not title:
        mf = Path(bdir) / "backup.meta.json"
        if mf.is_file():
            try:
                title = str(json.loads(mf.read_text(encoding="utf-8")).get("title") or "").strip()
            except Exception:
                pass

    sm = tables.get("session_message") or {}
    cols = sm.get("columns") or sm.get("cols") or []
    raw_rows = sm.get("rows") or []
    want = ("id", "type", "seq", "time_created", "time_updated", "data")
    if all(c in cols for c in want):
        idxs = [cols.index(c) for c in want]
        norm_rows = [tuple(r[i] for i in idxs) for r in raw_rows]
        norm_rows.sort(key=lambda r: (int(r[2] or 0), int(r[3] or 0), str(r[0] or "")))
        items = _items_from_message_rows(norm_rows, readonly=True)
    else:
        items = []
    return items, sid, title or sid


def _parse_key(key: str) -> tuple[str, str, int | None]:
    parts = key.split(":")
    if len(parts) == 2 and parts[0] in ("m", "e"):
        return parts[0], parts[1], None
    if len(parts) == 3 and parts[0] == "p" and parts[2].isdigit():
        return "p", parts[1], int(parts[2])
    raise ValueError(f"Некорректный ключ записи OpenCode: {key}")


def _load_msg_row(con: sqlite3.Connection, sid: str, mid: str):
    row = con.execute(
        "SELECT id, type, seq, data FROM session_message WHERE id = ? AND session_id = ?",
        (mid, sid),
    ).fetchone()
    if not row:
        raise LookupError(f"Сообщение {mid} не найдено в сессии {sid}")
    return row[0], row[1], int(row[2]), json.loads(row[3])


def preview_item_edit(sid: str, key: str, db: Path | None = None) -> dict:
    kind_k, mid, pidx = _parse_key(key)
    con = connect_ro(db)
    try:
        _, mtype, _seq, d = _load_msg_row(con, sid, mid)
    finally:
        con.close()

    if kind_k == "m":
        if mtype == "user":
            meta = d.get("metadata") if isinstance(d.get("metadata"), dict) else {}
            txt = d.get("text") if d.get("text") is not None else (meta.get("displayText") or "")
            return {"text": str(txt), "kind": "Сообщение пользователя", "hint": ""}
        if mtype == "compaction":
            return {"text": str(d.get("summary") or ""), "kind": "Компакция", "hint": ""}
        if mtype in ("system", "synthetic"):
            lbl = str(d.get("description") or ("Системное сообщение" if mtype == "system" else "Служебное сообщение"))
            return {"text": str(d.get("text") or ""), "kind": lbl, "hint": ""}
        raise ValueError(f"Тип сообщения {mtype} не поддерживает редактирование")

    if kind_k == "e":
        err = d.get("error")
        if isinstance(err, dict) and isinstance(err.get("message"), str):
            return {"text": err["message"], "kind": "Ошибка OpenCode", "hint": ""}
        return {"text": str(err or ""), "kind": "Ошибка OpenCode", "hint": ""}

    content = d.get("content") if isinstance(d.get("content"), list) else []
    if pidx is None or pidx >= len(content) or not isinstance(content[pidx], dict):
        raise LookupError(f"Фрагмент #{pidx} не найден в сообщении {mid}")
    part = content[pidx]
    ptype = part.get("type")
    if ptype == "text":
        return {"text": str(part.get("text") or ""), "kind": "Ответ OpenCode", "hint": ""}
    if ptype == "reasoning":
        return {"text": str(part.get("text") or ""), "kind": "Размышления", "hint": ""}
    if ptype == "tool":
        txt, _ = _tool_text_and_mode(part)
        tname = str(part.get("name") or "Инструмент")
        return {"text": txt, "kind": f"Инструмент: {tname}", "hint": ""}
    raise ValueError(f"Фрагмент типа {ptype} не поддерживает редактирование")


def apply_item_edit(sid: str, key: str, new_text: str, db: Path | None = None) -> dict:
    kind_k, mid, pidx = _parse_key(key)
    now_ms = int(time.time() * 1000)
    con = connect_rw(db)
    try:
        con.execute("BEGIN IMMEDIATE")
        _, mtype, seq, d = _load_msg_row(con, sid, mid)

        if kind_k == "m":
            if mtype == "user":
                d["text"] = new_text
                if isinstance(d.get("metadata"), dict) and "displayText" in d["metadata"]:
                    d["metadata"]["displayText"] = new_text
            elif mtype == "compaction":
                d["summary"] = new_text
            elif mtype in ("system", "synthetic"):
                d["text"] = new_text
            else:
                raise ValueError(f"Тип сообщения {mtype} не поддерживает редактирование")
        elif kind_k == "e":
            if isinstance(d.get("error"), dict) and "message" in d["error"]:
                d["error"]["message"] = new_text
            else:
                d["error"] = new_text
        else:
            content = d.get("content") if isinstance(d.get("content"), list) else []
            if pidx is None or pidx >= len(content) or not isinstance(content[pidx], dict):
                raise LookupError(f"Фрагмент #{pidx} не найден в сообщении {mid}")
            part = content[pidx]
            ptype = part.get("type")
            if ptype in ("text", "reasoning"):
                part["text"] = new_text
            elif ptype == "tool":
                _, mode = _tool_text_and_mode(part)
                st = part.setdefault("state", {})
                if mode == "state_content":
                    st["content"] = [{"type": "text", "text": new_text}]
                elif mode == "state_output":
                    st["output"] = new_text
                elif mode == "state_error_msg":
                    st["error"]["message"] = new_text
                elif mode == "state_error_str":
                    st["error"] = new_text
                elif mode == "state_input":
                    try:
                        st["input"] = json.loads(new_text)
                    except Exception:
                        st["input"] = {"value": new_text}
                else:
                    st["output"] = new_text
            else:
                raise ValueError(f"Фрагмент типа {ptype} не поддерживает редактирование")

        raw_new = json.dumps(d, ensure_ascii=False)
        con.execute(
            "UPDATE session_message SET data = ?, time_updated = ? WHERE id = ? AND session_id = ?",
            (raw_new, now_ms, mid, sid),
        )
        con.execute("UPDATE session_v2 SET time_updated = ? WHERE id = ?", (now_ms, sid))
        con.commit()
        chk = [r[0] for r in con.execute("PRAGMA quick_check")]
        assert chk == ["ok"], f"quick_check failed: {chk}"
    except Exception:
        if con.in_transaction:
            con.rollback()
        raise
    finally:
        con.close()
    return {"session_id": sid, "key": key, "seq": seq, "integrity_check": "ok"}


def delete_single_item(sid: str, key: str, db: Path | None = None) -> dict:
    kind_k, mid, pidx = _parse_key(key)
    now_ms = int(time.time() * 1000)
    con = connect_rw(db)
    try:
        con.execute("BEGIN IMMEDIATE")
        _, _mtype, seq, d = _load_msg_row(con, sid, mid)
        deleted_row = False

        if kind_k == "m":
            con.execute("DELETE FROM session_message WHERE id = ? AND session_id = ?", (mid, sid))
            deleted_row = True
        elif kind_k == "e":
            d.pop("error", None)
            content = d.get("content") if isinstance(d.get("content"), list) else []
            if not content:
                con.execute("DELETE FROM session_message WHERE id = ? AND session_id = ?", (mid, sid))
                deleted_row = True
            else:
                con.execute(
                    "UPDATE session_message SET data = ?, time_updated = ? WHERE id = ? AND session_id = ?",
                    (json.dumps(d, ensure_ascii=False), now_ms, mid, sid),
                )
        else:
            content = d.get("content") if isinstance(d.get("content"), list) else []
            if pidx is None or pidx >= len(content):
                raise LookupError(f"Фрагмент #{pidx} не найден в сообщении {mid}")
            content.pop(pidx)
            if not content and not d.get("error"):
                con.execute("DELETE FROM session_message WHERE id = ? AND session_id = ?", (mid, sid))
                deleted_row = True
            else:
                d["content"] = content
                con.execute(
                    "UPDATE session_message SET data = ?, time_updated = ? WHERE id = ? AND session_id = ?",
                    (json.dumps(d, ensure_ascii=False), now_ms, mid, sid),
                )

        con.execute("UPDATE session_v2 SET time_updated = ? WHERE id = ?", (now_ms, sid))
        con.commit()
    except Exception:
        if con.in_transaction:
            con.rollback()
        raise
    finally:
        con.close()
    return {"session_id": sid, "key": key, "seq": seq, "deleted_row": deleted_row}


def preview_cut(sid: str, key: str, mode: str, db: Path | None = None) -> dict:
    items = read_session_items(sid, db=db)
    pos = next((i for i, it in enumerate(items) if it["key"] == key), None)
    if pos is None:
        raise LookupError(f"Запись {key} не найдена")

    if mode == "tail":
        will = items[pos:]
    else:
        will = items[:pos]

    target = items[pos]
    target_seq = target["seq"]
    kind_k, mid, pidx = _parse_key(key)

    con = connect_ro(db)
    try:
        if mode == "tail":
            if kind_k == "p" and (pidx or 0) > 0:
                raw_cnt = con.execute(
                    "SELECT COUNT(*), COALESCE(SUM(LENGTH(data)), 0) FROM session_message WHERE session_id = ? AND seq > ?",
                    (sid, target_seq),
                ).fetchone()
                raw_rows = int(raw_cnt[0]) + 1
                raw_bytes = int(raw_cnt[1])
            else:
                raw_cnt = con.execute(
                    "SELECT COUNT(*), COALESCE(SUM(LENGTH(data)), 0) FROM session_message WHERE session_id = ? AND seq >= ?",
                    (sid, target_seq),
                ).fetchone()
                raw_rows = int(raw_cnt[0])
                raw_bytes = int(raw_cnt[1])
        else:
            raw_cnt = con.execute(
                "SELECT COUNT(*), COALESCE(SUM(LENGTH(data)), 0) FROM session_message WHERE session_id = ? AND seq < ?",
                (sid, target_seq),
            ).fetchone()
            raw_rows = int(raw_cnt[0])
            raw_bytes = int(raw_cnt[1])
    finally:
        con.close()

    preview_list = [
        {
            "key": f"#{x['seq']}",
            "label": x["label"],
            "text": (x["text"] or "").replace("\n", " ")[:140],
        }
        for x in will[:25]
    ]
    return {
        "provider": "opencode",
        "mode": mode,
        "steps_count": len(will),
        "raw_count": raw_rows,
        "raw_label": "сообщений в БД",
        "bytes": raw_bytes,
        "steps_preview": preview_list,
    }


def apply_cut(sid: str, key: str, mode: str, db: Path | None = None) -> dict:
    kind_k, mid, pidx = _parse_key(key)
    now_ms = int(time.time() * 1000)
    con = connect_rw(db)
    try:
        con.execute("BEGIN IMMEDIATE")
        _, _mtype, target_seq, d = _load_msg_row(con, sid, mid)
        removed_rows = 0

        if mode == "tail":
            if kind_k == "p" and (pidx or 0) > 0:
                content = d.get("content") if isinstance(d.get("content"), list) else []
                d["content"] = content[:pidx]
                d.pop("error", None)
                con.execute(
                    "UPDATE session_message SET data = ?, time_updated = ? WHERE id = ? AND session_id = ?",
                    (json.dumps(d, ensure_ascii=False), now_ms, mid, sid),
                )
                cur = con.execute(
                    "DELETE FROM session_message WHERE session_id = ? AND seq > ?",
                    (sid, target_seq),
                )
                removed_rows = cur.rowcount
            else:
                cur = con.execute(
                    "DELETE FROM session_message WHERE session_id = ? AND seq >= ?",
                    (sid, target_seq),
                )
                removed_rows = cur.rowcount
        else:
            cur = con.execute(
                "DELETE FROM session_message WHERE session_id = ? AND seq < ?",
                (sid, target_seq),
            )
            removed_rows = cur.rowcount

        con.execute("UPDATE session_v2 SET time_updated = ?, revert = NULL WHERE id = ?", (now_ms, sid))
        con.commit()
        chk = [r[0] for r in con.execute("PRAGMA quick_check")]
        assert chk == ["ok"], f"quick_check failed: {chk}"
        left = con.execute("SELECT COUNT(*) FROM session_message WHERE session_id = ?", (sid,)).fetchone()[0]
    except Exception:
        if con.in_transaction:
            con.rollback()
        raise
    finally:
        con.close()
    return {
        "session_id": sid,
        "mode": mode,
        "removed_rows": removed_rows,
        "rows_after": int(left),
        "integrity_check": "ok",
    }


def backup_session(sid: str, tag: str = "", db: Path | None = None, backup_root: Path | None = None) -> Path:
    b_root = Path(backup_root or BACKUP_ROOT)
    ts = datetime.now().strftime("%Y%m%d-%H%M%S-%f")
    bdir = b_root / sid / (ts + ("-" + tag if tag else ""))
    bdir.mkdir(parents=True, exist_ok=True)

    con = connect_ro(db)
    try:
        existing_tables = {
            r[0] for r in con.execute("SELECT name FROM sqlite_master WHERE type = 'table'")
        }
        snap: dict[str, dict] = {}
        for tname, id_col in BACKUP_TABLES:
            if tname not in existing_tables:
                continue
            cols = [r[1] for r in con.execute(f'PRAGMA table_info("{tname}")')]
            rows = con.execute(f'SELECT * FROM "{tname}" WHERE "{id_col}" = ?', (sid,)).fetchall()
            snap[tname] = {"columns": cols, "rows": [list(r) for r in rows]}
    finally:
        con.close()

    if not (snap.get("session_v2") or {}).get("rows"):
        raise FileNotFoundError(f"Сессия OpenCode {sid} не найдена")

    title = ""
    try:
        s_cols = snap["session_v2"]["columns"]
        s_row = snap["session_v2"]["rows"][0]
        title = str(dict(zip(s_cols, s_row)).get("title") or "")
    except Exception:
        pass

    (bdir / "session_backup.json").write_text(
        json.dumps({"session_id": sid, "tables": snap}, ensure_ascii=False),
        encoding="utf-8",
    )
    (bdir / "backup.meta.json").write_text(
        json.dumps({
            "session_id": sid,
            "title": title,
            "created": datetime.now().isoformat(),
            "tag": tag,
            "db": str(Path(db or DB_PATH)),
        }, ensure_ascii=False, indent=1),
        encoding="utf-8",
    )
    return bdir


def restore_session_backup(backup_dir: Path, db: Path | None = None) -> dict:
    snap_path = Path(backup_dir) / "session_backup.json"
    if not snap_path.is_file():
        raise FileNotFoundError(snap_path)
    payload = json.loads(snap_path.read_text(encoding="utf-8"))
    sid = payload["session_id"]
    tables = payload.get("tables") or {}

    con = connect_rw(db)
    try:
        existing_tables = {
            r[0] for r in con.execute("SELECT name FROM sqlite_master WHERE type = 'table'")
        }
        con.execute("BEGIN IMMEDIATE")

        cur_seq_row = con.execute(
            "SELECT seq FROM event_sequence WHERE aggregate_id = ?", (sid,)
        ).fetchone() if "event_sequence" in existing_tables else None
        cur_seq = int(cur_seq_row[0]) if cur_seq_row else 0

        if "event_sequence" in existing_tables:
            con.execute("DELETE FROM event_sequence WHERE aggregate_id = ?", (sid,))
        con.execute("DELETE FROM session_v2 WHERE id = ?", (sid,))

        restored_counts: dict[str, int] = {}
        for tname, _id_col in BACKUP_TABLES:
            if tname not in existing_tables or tname not in tables:
                continue
            tdata = tables[tname]
            cols = tdata.get("columns") or []
            rows = tdata.get("rows") or []
            if not cols or not rows:
                continue
            valid_cols = {r[1] for r in con.execute(f'PRAGMA table_info("{tname}")')}
            keep_idx = [i for i, c in enumerate(cols) if c in valid_cols]
            use_cols = [cols[i] for i in keep_idx]
            placeholders = ", ".join("?" for _ in use_cols)
            col_sql = ", ".join(f'"{c}"' for c in use_cols)
            sql = f'INSERT OR REPLACE INTO "{tname}" ({col_sql}) VALUES ({placeholders})'
            for r in rows:
                vals = [r[i] for i in keep_idx]
                if tname == "event_sequence" and "seq" in use_cols:
                    si = use_cols.index("seq")
                    vals[si] = max(int(vals[si] or 0), cur_seq)
                con.execute(sql, vals)
            restored_counts[tname] = len(rows)

        con.commit()
        chk = [r[0] for r in con.execute("PRAGMA quick_check")]
        assert chk == ["ok"], f"quick_check failed: {chk}"
    except Exception:
        if con.in_transaction:
            con.rollback()
        raise
    finally:
        con.close()
    return {"session_id": sid, "restored_tables": restored_counts, "integrity_check": "ok"}


def delete_session(sid: str, db: Path | None = None) -> dict:
    con = connect_rw(db)
    try:
        con.execute("BEGIN IMMEDIATE")
        try:
            con.execute("DELETE FROM event_sequence WHERE aggregate_id = ?", (sid,))
        except sqlite3.Error:
            pass
        cur = con.execute("DELETE FROM session_v2 WHERE id = ?", (sid,))
        if cur.rowcount == 0:
            raise FileNotFoundError(f"Сессия OpenCode {sid} не найдена")
        con.commit()
    except Exception:
        if con.in_transaction:
            con.rollback()
        raise
    finally:
        con.close()
    return {"deleted": sid}
