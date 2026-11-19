from __future__ import annotations

from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import re
import sqlite3
import time
from urllib.parse import unquote
import uuid

import ag_history_tool as AG
import claude_history_tool as CL
from claude_io import atomic_write_text, win_long_path
import codex_history_tool as CX
import k1tty_pack as KP
import opencode_history_tool as OC

_USER_REQ_RE = re.compile(r"<USER_REQUEST>\s*([\s\S]*?)\s*</USER_REQUEST>")
_ADD_META_RE = re.compile(r"<ADDITIONAL_METADATA>[\s\S]*?</ADDITIONAL_METADATA>\s*")
_SYS_REMINDER_RE = re.compile(r"<system-reminder>[\s\S]*?</system-reminder>\s*")
_BASE62 = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz"


def _now_iso() -> str:
    return datetime.now(timezone.utc).isoformat().replace("+00:00", "Z")


def _parse_ts_sec(ts: str | int | float | None, fallback: float) -> float:
    if not ts:
        return fallback
    if isinstance(ts, (int, float)):
        v = float(ts) / 1000.0 if ts > 10_000_000_000 else float(ts)
        return v if v > 946_684_800 else fallback
    try:
        s = str(ts).strip()
        if s.endswith("Z"):
            s = s[:-1] + "+00:00"
        v = datetime.fromisoformat(s).timestamp()
        return v if v > 946_684_800 else fallback
    except Exception:
        return fallback


def _ts_to_iso(sec: float) -> str:
    return datetime.fromtimestamp(sec, tz=timezone.utc).isoformat().replace("+00:00", "Z")


def _ts_to_ag_dt(sec: float) -> str:
    return datetime.fromtimestamp(sec, tz=timezone.utc).strftime("%Y-%m-%d %H:%M:%S.%f+00:00")


def _pb_timestamp(sec: float) -> bytes:
    whole = max(0, int(sec))
    nanos = max(0, min(999_999_999, int((sec - whole) * 1_000_000_000)))
    out = AG.encode_field(1, 0, AG.encode_varint(whole))
    if nanos:
        out += AG.encode_field(2, 0, AG.encode_varint(nanos))
    return out


def _pb_get_bytes(buf: bytes | None, fnum: int) -> bytes:
    if not buf:
        return b""
    try:
        for fn, wt, val in AG.parse_fields(bytes(buf)):
            if fn == fnum and wt == 2 and isinstance(val, (bytes, bytearray)):
                return bytes(val)
    except Exception:
        pass
    return b""


def _uuid7(ts_ms: int | None = None) -> str:
    ms = int(time.time() * 1000) if ts_ms is None else int(ts_ms)
    rnd = os.urandom(10)
    b0 = (ms >> 40) & 0xFF
    b1 = (ms >> 32) & 0xFF
    b2 = (ms >> 24) & 0xFF
    b3 = (ms >> 16) & 0xFF
    b4 = (ms >> 8) & 0xFF
    b5 = ms & 0xFF
    b6 = 0x70 | (rnd[0] & 0x0F)
    b7 = rnd[1]
    b8 = 0x80 | (rnd[2] & 0x3F)
    raw = bytes([b0, b1, b2, b3, b4, b5, b6, b7, b8]) + rnd[3:10]
    return str(uuid.UUID(bytes=raw))


def _opencode_id(prefix: str, ts_ms: int, counter: int = 1, *, descending: bool = False) -> str:
    val = ((int(ts_ms) * 4096) + (counter & 0xFFF)) & 0xFFFFFFFFFFFF
    if descending:
        val = 0xFFFFFFFFFFFF - val
    rnd = os.urandom(14)
    tail = "".join(_BASE62[b % 62] for b in rnd)
    return f"{prefix}_{val:012x}{tail}"


def _is_ephemeral_workspace(p: str) -> bool:
    low = p.replace("/", "\\").lower()
    if "\\claude-data\\" in low and "\\scratch-workspaces\\" in low:
        return True
    if "\\claude\\scratch-workspaces\\" in low:
        return True
    if "\\documents\\codex\\" in low:
        return True
    return False


def _strip_win_long_prefix(p: str) -> str:
    s = str(p or "").strip()
    if s.startswith("\\\\?\\UNC\\"):
        return "\\\\" + s[8:]
    if s.startswith("\\\\?\\"):
        return s[4:]
    return s


def _clean_ag_user_text(raw: str) -> str:
    if not raw:
        return ""
    m = _USER_REQ_RE.search(raw)
    if m:
        return m.group(1).strip()
    cleaned = _ADD_META_RE.sub("", raw).strip()
    return cleaned or raw


def _clean_claude_user_text(raw: str) -> str:
    if not raw:
        return ""
    cleaned = _SYS_REMINDER_RE.sub("", raw).strip()
    return cleaned if cleaned else raw


def _derive_title(entries: list[dict], preferred: str | None = None) -> str:
    if preferred and preferred.strip():
        return preferred.strip()[:140]
    for e in entries:
        if e.get("kind") == "user":
            txt = " ".join(str(e.get("text") or "").split())
            if txt:
                return txt[:90]
    return "Imported chat"


def _filter_entries(entries: list[dict], include_tools: bool) -> list[dict]:
    if include_tools:
        return [e for e in entries if (e.get("text") or "").strip() or e.get("tool")]
    return [
        e for e in entries
        if e.get("kind") in ("user", "assistant", "compact") and (e.get("text") or "").strip()
    ]


def _monotonic_timestamps(entries: list[dict], now_sec: float) -> list[float]:
    n = len(entries)
    base_start = now_sec - max(2, n) * 2.0
    out: list[float] = []
    prev = 0.0
    for idx, it in enumerate(entries):
        fallback = (prev + 1.0) if prev > 0 else (base_start + idx * 1.5)
        sec = _parse_ts_sec(it.get("ts"), fallback)
        if prev > 0 and sec < prev:
            sec = prev + 0.05
        out.append(sec)
        prev = sec
    return out


def _read_ag_steps_fallback(db_path: Path) -> list[dict]:
    if not db_path.is_file():
        return []
    out: list[dict] = []
    try:
        con = AG.connect_ro(db_path)
        try:
            rows = con.execute(
                "SELECT idx, step_type, metadata, step_payload FROM steps ORDER BY idx"
            ).fetchall()
        finally:
            con.close()
    except Exception:
        return []

    for idx, stype, meta_b, pl_b in rows:
        if not pl_b:
            continue
        sec = 0.0
        if meta_b:
            f1 = _pb_get_bytes(meta_b, 1)
            if f1:
                for fn, wt, val in AG.parse_fields(f1):
                    if fn == 1 and wt == 0:
                        sec = float(val)
                        break
        iso = _ts_to_iso(sec) if sec > 946_684_800 else ""
        if stype == 14:
            f19 = _pb_get_bytes(pl_b, 19)
            txt = ""
            if f19:
                f2 = _pb_get_bytes(f19, 2)
                if f2:
                    txt = f2.decode("utf-8", "replace")
            if txt.strip():
                out.append({
                    "step_index": int(idx),
                    "type": "USER_INPUT",
                    "content": txt,
                    "created_at": iso,
                    "status": "DONE",
                })
        elif stype == 15:
            f20 = _pb_get_bytes(pl_b, 20)
            txt = ""
            think = ""
            if f20:
                f8 = _pb_get_bytes(f20, 8)
                f1 = _pb_get_bytes(f20, 1)
                f3 = _pb_get_bytes(f20, 3)
                raw_t = f8 or f1
                if raw_t:
                    txt = raw_t.decode("utf-8", "replace")
                if f3:
                    think = f3.decode("utf-8", "replace")
            if txt.strip() or think.strip():
                out.append({
                    "step_index": int(idx),
                    "type": "PLANNER_RESPONSE",
                    "content": txt,
                    "thinking": think,
                    "created_at": iso,
                    "status": "DONE",
                })
    return out


def export_provider_chat(prov, cid: str, *, include_tools: bool = True) -> dict:
    pid = prov.id
    entries: list[dict] = []
    title = ""
    directory = ""
    raw_size = 0

    if pid in ("antigravity", "antigravity_cli"):
        store = prov.store
        db = store.conversations_dir / f"{cid}.db"
        if db.is_file():
            raw_size += db.stat().st_size
        b = AG.brain_dir(cid, store=store)
        for name in ("transcript_full.jsonl", "transcript.jsonl"):
            tp = b / "logs" / name
            if tp.is_file():
                raw_size += tp.stat().st_size
                break
        try:
            con = AG.connect_ro(store.summaries_db)
            try:
                row = con.execute(
                    "SELECT title, workspace_uris FROM conversation_summaries WHERE conversation_id=?",
                    (cid,),
                ).fetchone()
                if row:
                    tval = row[0]
                    title = (tval.decode("utf-8", "replace") if isinstance(tval, bytes) else str(tval or "")).strip()
                    wval = row[1]
                    ws = (wval.decode("utf-8", "replace") if isinstance(wval, bytes) else str(wval or "")).strip()
                    if ws:
                        try:
                            uris = json.loads(ws)
                            if isinstance(uris, list) and uris:
                                u0 = unquote(str(uris[0]))
                                if u0.startswith("file:///"):
                                    directory = u0[8:] if (len(u0) >= 10 and u0[9] == ":") else u0[7:]
                        except Exception:
                            pass
            finally:
                con.close()
        except Exception:
            pass

        kind_map = {
            "USER_INPUT": "user",
            "PLANNER_RESPONSE": "assistant",
            "GENERIC": "tool",
            "CHECKPOINT": "compact",
            "SYSTEM_MESSAGE": "system",
            "ERROR": "error",
            "ERROR_MESSAGE": "error",
        }
        hist = AG.read_history(cid, store=store)
        if not hist and db.is_file():
            hist = _read_ag_steps_fallback(db)
        for e in hist:
            et = str(e.get("type") or "")
            k = kind_map.get(et, "other")
            txt = str(e.get("content") or "")
            if k == "user":
                txt = _clean_ag_user_text(txt)
            think = str(e.get("thinking") or "").strip()
            if k == "assistant" and think and include_tools:
                entries.append({
                    "kind": "tool",
                    "subtype": "thinking",
                    "label": "thinking",
                    "text": think,
                    "ts": str(e.get("created_at") or ""),
                    "tool": "thinking",
                    "status": "ok",
                    "code": "",
                })
            tools = [c.get("name") for c in (e.get("tool_calls") or []) if c.get("name")]
            if not txt.strip() and tools:
                txt = "\n".join(
                    f"{c.get('name')}({json.dumps(c.get('args') or c.get('arguments') or {}, ensure_ascii=False)})"
                    for c in (e.get("tool_calls") or [])
                )
            if not txt.strip() and not tools:
                continue
            entries.append({
                "kind": k,
                "subtype": et.lower() or k,
                "label": ", ".join(tools[:3]) if tools else k,
                "text": txt,
                "ts": str(e.get("created_at") or ""),
                "tool": ", ".join(tools) if tools else None,
                "status": e.get("status"),
                "code": "",
            })

    elif pid == "codex":
        files = CX.thread_files(cid)
        if not files:
            raise FileNotFoundError(f"Тред Codex {cid} не найден.")
        raw_size = sum(p.stat().st_size for p in files if p.exists())
        try:
            title = CX.thread_title(cid)
        except Exception:
            title = ""
        try:
            with files[0].open("r", encoding="utf-8", errors="replace") as fh:
                first = json.loads(fh.readline())
                directory = _strip_win_long_prefix(str((first.get("payload") or {}).get("cwd") or ""))
        except Exception:
            pass
        for it in CX.read_history(cid):
            k = it.get("kind", "other")
            txt = str(it.get("text") or "")
            if not txt.strip() and k == "tool" and it.get("label") == "reasoning":
                continue
            entries.append({
                "kind": k,
                "subtype": "thinking" if it.get("label") == "reasoning" else str(it.get("subtype") or k),
                "label": str(it.get("label") or k),
                "text": txt,
                "ts": str(it.get("ts") or ""),
                "tool": it.get("label") if k == "tool" else None,
                "status": it.get("status"),
                "code": str(it.get("code") or ""),
            })

    elif pid == "claude":
        path = CL.session_file(cid)
        raw_size = os.stat(win_long_path(path)).st_size
        try:
            title = CL.session_title(cid)
        except Exception:
            title = ""
        _, cwd = CL._extract_title_from_file(path)
        directory = _strip_win_long_prefix(cwd or "")
        with open(win_long_path(path), "rb") as fh:
            offset = 0
            line_no = 0
            for raw_line in fh:
                line_no += 1
                try:
                    o = json.loads(raw_line.decode("utf-8", "replace"))
                    it = CL._history_item(o, offset, line_no)
                except Exception:
                    it = None
                offset += len(raw_line)
                if not it:
                    continue
                k = it.get("kind", "other")
                txt = str(it.get("text") or "")
                if k == "user":
                    txt = _clean_claude_user_text(txt)
                if not txt.strip() and not it.get("tool"):
                    continue
                entries.append({
                    "kind": k,
                    "subtype": str(it.get("subtype") or k),
                    "label": str(it.get("label") or k),
                    "text": txt,
                    "ts": str(it.get("ts") or ""),
                    "tool": it.get("tool"),
                    "status": it.get("status"),
                    "code": "",
                })

    elif pid == "opencode":
        for s in OC.list_sessions(prov.db_path):
            if s["id"] == cid:
                title = s.get("title") or ""
                directory = _strip_win_long_prefix(s.get("directory") or "")
                raw_size = int(s.get("size") or 0)
                break
        for it in OC.read_session_items(cid, db=prov.db_path):
            txt = str(it.get("text") or "")
            k = str(it.get("kind") or "other")
            sub = str(it.get("subtype") or k)
            if not txt.strip() and sub == "thinking":
                continue
            entries.append({
                "kind": k,
                "subtype": sub,
                "label": str(it.get("label") or ""),
                "text": txt,
                "ts": str(it.get("ts") or ""),
                "tool": it.get("tool"),
                "status": it.get("status"),
                "code": str(it.get("code") or ""),
            })
    else:
        raise ValueError(f"Неизвестный провайдер: {pid}")

    filtered = _filter_entries(entries, include_tools=include_tools)
    final_title = _derive_title(filtered or entries, title)
    ts_list = [e["ts"] for e in filtered if e.get("ts")]
    return {
        "meta": {
            "source_provider": pid,
            "source_id": cid,
            "title": final_title,
            "directory": directory,
            "created_at": ts_list[0] if ts_list else _now_iso(),
            "updated_at": ts_list[-1] if ts_list else _now_iso(),
            "raw_size": raw_size,
        },
        "entries": filtered,
    }


def _ensure_ag_summaries_schema(con: sqlite3.Connection):
    con.execute(
        "CREATE TABLE IF NOT EXISTS conversation_summaries ("
        "conversation_id text PRIMARY KEY, "
        "title text NOT NULL DEFAULT '', "
        "preview text NOT NULL DEFAULT '', "
        "step_count integer NOT NULL DEFAULT 0, "
        "last_modified_time datetime NOT NULL, "
        "workspace_uris text NOT NULL DEFAULT '', "
        "status text NOT NULL DEFAULT '', "
        "source text NOT NULL DEFAULT '', "
        "project_id text NOT NULL DEFAULT '', "
        "agent_name text NOT NULL DEFAULT '', "
        "parent_conversation_id text NOT NULL DEFAULT '', "
        "nesting_depth integer NOT NULL DEFAULT 0, "
        "battle_id text NOT NULL DEFAULT '', "
        "winning_conversation_id text NOT NULL DEFAULT '', "
        "not_fully_idle numeric NOT NULL DEFAULT false, "
        "killed numeric NOT NULL DEFAULT false, "
        "last_user_input_time datetime NOT NULL, "
        "last_user_input_step_index integer NOT NULL DEFAULT -1, "
        "app_data_dir text NOT NULL DEFAULT '', "
        "raw_summary blob, "
        "group_id text NOT NULL DEFAULT '')"
    )
    con.execute(
        "CREATE TABLE IF NOT EXISTS conversation_parents ("
        "conversation_id TEXT PRIMARY KEY, parent_id TEXT NOT NULL) WITHOUT ROWID"
    )
    has_fts = con.execute(
        "SELECT 1 FROM sqlite_master WHERE type='table' AND name='conversations_fts'"
    ).fetchone()
    if not has_fts:
        try:
            con.execute(
                "CREATE VIRTUAL TABLE conversations_fts USING fts5("
                "conversation_id UNINDEXED, chunk_index UNINDEXED, chunk_first_step UNINDEXED, "
                "chunk_last_step UNINDEXED, last_modified_time UNINDEXED, step_count UNINDEXED, "
                "last_step_created_at UNINDEXED, archived UNINDEXED, user_step_byte_offsets UNINDEXED, "
                "agent_step_byte_offsets UNINDEXED, title, workspace, project, user_body, agent_body, "
                "tokenize = 'unicode61 remove_diacritics 2', prefix = '1 2 3')"
            )
        except sqlite3.Error:
            pass


def _create_ag_conv_db(db_path: Path):
    con = sqlite3.connect(str(db_path), timeout=30)
    try:
        con.execute("pragma journal_mode=delete")
        con.execute(
            "CREATE TABLE trajectory_meta ("
            "trajectory_id text PRIMARY KEY, cascade_id text, trajectory_type integer, source integer)"
        )
        con.execute(
            "CREATE TABLE steps ("
            "idx integer PRIMARY KEY, step_type integer NOT NULL DEFAULT 0, status integer NOT NULL DEFAULT 0, "
            "has_subtrajectory numeric NOT NULL DEFAULT false, metadata blob, error_details blob, "
            "permissions blob, task_details blob, render_info blob, step_payload blob, step_format integer NOT NULL DEFAULT 0)"
        )
        con.execute("CREATE INDEX idx_steps_status ON steps(status)")
        con.execute("CREATE INDEX idx_steps_step_type ON steps(step_type)")
        con.execute("CREATE TABLE gen_metadata (idx integer PRIMARY KEY, data blob, size integer NOT NULL DEFAULT 0)")
        con.execute("CREATE TABLE executor_metadata (idx integer PRIMARY KEY, data blob)")
        con.execute("CREATE TABLE parent_references (idx integer PRIMARY KEY, data blob)")
        con.execute("CREATE TABLE trajectory_metadata_blob (id text PRIMARY KEY DEFAULT 'main', data blob)")
        con.execute("CREATE TABLE battle_mode_infos (idx integer PRIMARY KEY, data blob)")
        con.commit()
    finally:
        con.close()


def _encode_nested_pb(path: list[int], leaf: bytes) -> bytes:
    cur = leaf
    for fnum in reversed(path):
        cur = AG.encode_field(fnum, 2, cur)
    return cur


def _ag_template_from_store(store: AG.AgStore) -> dict:
    tpl: dict = {"cap_f15": b"", "inst_uuid": "", "user_cfg_f12": b""}
    if not store.conversations_dir.is_dir():
        return tpl
    dbs = sorted(
        store.conversations_dir.glob("*.db"),
        key=lambda p: p.stat().st_mtime if p.exists() else 0.0,
        reverse=True,
    )
    for db in dbs[:6]:
        try:
            con = AG.connect_ro(db)
            try:
                if not tpl["cap_f15"]:
                    row = con.execute("SELECT data FROM trajectory_metadata_blob WHERE id='main'").fetchone()
                    if row and row[0]:
                        mb = bytes(row[0])
                        f15 = _pb_get_bytes(mb, 15)
                        if f15:
                            tpl["cap_f15"] = f15
                        f3 = _pb_get_bytes(mb, 3)
                        if f3 and not tpl["inst_uuid"]:
                            try:
                                tpl["inst_uuid"] = f3.decode("utf-8")
                            except Exception:
                                pass
                if not tpl["user_cfg_f12"]:
                    srow = con.execute(
                        "SELECT step_payload FROM steps WHERE step_type=14 ORDER BY idx ASC LIMIT 1"
                    ).fetchone()
                    if srow and srow[0]:
                        f19 = _pb_get_bytes(bytes(srow[0]), 19)
                        if f19:
                            f12 = _pb_get_bytes(f19, 12)
                            if f12:
                                tpl["user_cfg_f12"] = f12
            finally:
                con.close()
        except Exception:
            continue
        if tpl["cap_f15"] and tpl["user_cfg_f12"]:
            break
    return tpl


def _write_ag_annotation(store: AG.AgStore, cid: str, title: str, ts_sec: float):
    try:
        store.annotations_dir.mkdir(parents=True, exist_ok=True)
        esc = title.replace("\\", "\\\\").replace('"', '\\"').replace("\r", " ").replace("\n", " ")
        whole = max(0, int(ts_sec))
        nanos = max(0, min(999_999_999, int((ts_sec - whole) * 1_000_000_000)))
        txt = f'title:"{esc}" last_user_view_time:{{seconds:{whole} nanos:{nanos}}}\n'
        p = store.annotations_dir / f"{cid}.pbtxt"
        tmp = p.with_suffix(".pbtxt.tmp")
        tmp.write_text(txt, encoding="utf-8")
        tmp.replace(p)
    except Exception:
        pass


def _upsert_ag_summaries_proto(store: AG.AgStore, app_data_dir: str, cid: str, raw_summary: bytes):
    fname = "jetbox_summaries_proto.pb" if app_data_dir == "antigravity-cli" else "agyhub_summaries_proto.pb"
    pb_path = store.appdata / fname
    if not pb_path.is_file():
        return
    try:
        existing = pb_path.read_bytes()
        cid_b = cid.encode("utf-8")
        kept = bytearray()
        for fn, wt, val in AG.parse_fields(existing):
            if fn == 1 and wt == 2 and isinstance(val, (bytes, bytearray)):
                k = _pb_get_bytes(bytes(val), 1)
                if k == cid_b:
                    continue
            if wt == 0:
                kept += AG.encode_field(fn, 0, AG.encode_varint(val))
            elif wt == 2:
                kept += AG.encode_field(fn, 2, bytes(val))
            elif wt == 1:
                kept += AG.encode_field(fn, 1, bytes(val))
            elif wt == 5:
                kept += AG.encode_field(fn, 5, bytes(val))
        entry = AG.encode_field(1, 2, cid_b) + AG.encode_field(2, 2, raw_summary)
        kept += AG.encode_field(1, 2, entry)
        tmp = pb_path.with_suffix(".pb.tmp")
        tmp.write_bytes(bytes(kept))
        tmp.replace(pb_path)
    except Exception:
        pass


def _merge_ag_entries(raw_entries: list[dict], timestamps: list[float]) -> list[tuple[dict, float, str]]:
    merged: list[tuple[dict, float, str]] = []
    pending_think: list[str] = []
    for it, sec in zip(raw_entries, timestamps):
        kind = str(it.get("kind") or "assistant")
        subtype = str(it.get("subtype") or "")
        text = str(it.get("text") or "")
        if kind == "tool" and subtype == "thinking":
            if text.strip():
                pending_think.append(text.strip())
            continue
        if kind == "assistant" and pending_think:
            merged.append((it, sec, "\n\n".join(pending_think)))
            pending_think.clear()
        else:
            if pending_think:
                merged.append((
                    {"kind": "assistant", "subtype": "planner_response", "text": "", "label": "thinking"},
                    sec,
                    "\n\n".join(pending_think),
                ))
                pending_think.clear()
            merged.append((it, sec, ""))
    if pending_think:
        last_sec = timestamps[-1] if timestamps else time.time()
        merged.append((
            {"kind": "assistant", "subtype": "planner_response", "text": "", "label": "thinking"},
            last_sec,
            "\n\n".join(pending_think),
        ))
    return merged


def import_to_antigravity(
    doc: dict,
    store: AG.AgStore,
    *,
    app_data_dir: str = "antigravity",
    title: str | None = None,
    include_tools: bool = True,
) -> dict:
    raw_entries = _filter_entries(doc.get("entries") or [], include_tools=include_tools)
    if not raw_entries:
        raise ValueError("В файле нет сообщений для импорта.")

    is_cli = app_data_dir == "antigravity-cli"
    meta = doc.get("meta") or {}
    final_title = _derive_title(raw_entries, title or meta.get("title"))
    cid = str(uuid.uuid4())
    traj_id = str(uuid.uuid4())
    now_sec = time.time()
    tpl = _ag_template_from_store(store)
    inst_uuid = tpl["inst_uuid"] or str(uuid.uuid4())

    raw_dir = _strip_win_long_prefix(str(meta.get("directory") or "").strip())
    ws_uri = ""
    if raw_dir and not _is_ephemeral_workspace(raw_dir) and Path(raw_dir).is_dir():
        norm_dir = raw_dir.replace("\\", "/")
        ws_uri = norm_dir if norm_dir.startswith("file://") else ("file:///" + norm_dir.lstrip("/"))
    elif is_cli:
        norm_home = str(Path.home()).replace("\\", "/")
        ws_uri = "file:///" + norm_home.lstrip("/")
        raw_dir = str(Path.home())
    else:
        raw_dir = ""

    project_id = "default-cli-project" if is_cli else "outside-of-project"
    ws_uris = json.dumps([ws_uri], ensure_ascii=False) if ws_uri else ""

    timestamps = _monotonic_timestamps(raw_entries, now_sec)
    merged = _merge_ag_entries(raw_entries, timestamps)

    store.conversations_dir.mkdir(parents=True, exist_ok=True)
    bdir = AG.brain_dir(cid, store=store)
    (bdir / "logs").mkdir(parents=True, exist_ok=True)
    db_path = store.conversations_dir / f"{cid}.db"
    _create_ag_conv_db(db_path)

    step_rows = []
    transcript_lines: list[str] = []
    user_body = bytearray()
    agent_body = bytearray()
    user_offs: list[list[int]] = []
    agent_offs: list[list[int]] = []

    first_sec = merged[0][1] if merged else now_sec
    last_sec = merged[-1][1] if merged else now_sec
    last_user_idx = -1
    last_user_sec = first_sec
    first_user_preview = ""
    turn_group_uuid = str(uuid.uuid4())

    for idx, (it, sec, think_txt) in enumerate(merged):
        iso = _ts_to_iso(sec)
        kind = str(it.get("kind") or "assistant")
        text = str(it.get("text") or "")
        text_b = text.encode("utf-8")
        ts_pb = _pb_timestamp(sec)
        req_hex = uuid.uuid4().hex

        if kind == "user":
            turn_group_uuid = str(uuid.uuid4())

        traj_ref = AG.encode_field(1, 2, traj_id.encode("utf-8"))
        if idx > 0:
            traj_ref += AG.encode_field(2, 0, AG.encode_varint(idx))
        traj_ref += AG.encode_field(4, 2, cid.encode("utf-8"))

        status_done = AG.encode_field(1, 2, AG.encode_field(1, 0, AG.encode_varint(3)) + AG.encode_field(2, 2, ts_pb))

        if kind == "user":
            stype = 14
            t_type = "USER_INPUT"
            t_source = "USER_EXPLICIT"
            t_content = f"<USER_REQUEST>\n{text}\n</USER_REQUEST>"
            meta_pb = (
                AG.encode_field(1, 2, ts_pb)
                + AG.encode_field(3, 0, AG.encode_varint(4))
                + AG.encode_field(12, 2, turn_group_uuid.encode("utf-8"))
                + AG.encode_field(20, 2, traj_ref)
                + AG.encode_field(26, 2, status_done)
                + AG.encode_field(35, 2, req_hex.encode("utf-8"))
            )
            f19 = (
                AG.encode_field(2, 2, text_b)
                + AG.encode_field(3, 2, AG.encode_field(1, 2, text_b))
                + AG.encode_field(4, 2, b"")
            )
            if tpl["user_cfg_f12"]:
                f19 += AG.encode_field(12, 2, tpl["user_cfg_f12"])
            f19 += AG.encode_field(19, 2, str(uuid.uuid4()).encode("utf-8"))
            body_pb = AG.encode_field(19, 2, f19)
            last_user_idx = idx
            last_user_sec = sec
            if not first_user_preview:
                first_user_preview = " ".join(text.split())[:300]
            user_offs.append([idx, len(user_body)])
            user_body += (text + "\n").encode("utf-8")
        elif kind == "assistant":
            stype = 15
            t_type = "PLANNER_RESPONSE"
            t_source = "MODEL"
            t_content = text
            status_pair = (
                AG.encode_field(1, 2, AG.encode_field(1, 0, AG.encode_varint(8)) + AG.encode_field(2, 2, ts_pb))
                + status_done
            )
            meta_pb = (
                AG.encode_field(1, 2, ts_pb)
                + AG.encode_field(3, 0, AG.encode_varint(2))
                + AG.encode_field(6, 2, ts_pb)
                + AG.encode_field(7, 2, ts_pb)
                + AG.encode_field(8, 2, ts_pb)
                + AG.encode_field(12, 2, turn_group_uuid.encode("utf-8"))
                + AG.encode_field(20, 2, traj_ref)
                + AG.encode_field(26, 2, status_pair)
                + AG.encode_field(32, 2, ts_pb)
                + AG.encode_field(35, 2, req_hex.encode("utf-8"))
            )
            bot_id = f"bot-{uuid.uuid4()}".encode("utf-8")
            f20 = b""
            if text_b:
                f20 += AG.encode_field(1, 2, text_b)
            if think_txt:
                f20 += AG.encode_field(3, 2, think_txt.encode("utf-8"))
            f20 += AG.encode_field(6, 2, bot_id)
            if text_b:
                f20 += AG.encode_field(8, 2, text_b)
            f20 += AG.encode_field(12, 0, AG.encode_varint(2))
            body_pb = AG.encode_field(20, 2, f20)
            if text:
                agent_offs.append([idx, len(agent_body)])
                agent_body += (text + "\n").encode("utf-8")
        elif kind == "compact":
            stype = 23
            t_type = "CHECKPOINT"
            t_source = "SYSTEM"
            t_content = text
            meta_pb = (
                AG.encode_field(1, 2, ts_pb)
                + AG.encode_field(3, 0, AG.encode_varint(5))
                + AG.encode_field(8, 2, ts_pb)
                + AG.encode_field(12, 2, turn_group_uuid.encode("utf-8"))
                + AG.encode_field(20, 2, traj_ref)
                + AG.encode_field(26, 2, status_done)
            )
            body_pb = _encode_nested_pb([30, 5], text_b)
        elif kind == "error":
            stype = 17
            t_type = "ERROR_MESSAGE"
            t_source = "SYSTEM"
            t_content = text
            meta_pb = (
                AG.encode_field(1, 2, ts_pb)
                + AG.encode_field(3, 0, AG.encode_varint(5))
                + AG.encode_field(8, 2, ts_pb)
                + AG.encode_field(12, 2, turn_group_uuid.encode("utf-8"))
                + AG.encode_field(20, 2, traj_ref)
                + AG.encode_field(26, 2, status_done)
            )
            body_pb = _encode_nested_pb([24, 3, 2], text_b)
        elif kind == "tool":
            stype = 132
            t_type = "GENERIC"
            t_source = "SYSTEM"
            t_content = text
            meta_pb = (
                AG.encode_field(1, 2, ts_pb)
                + AG.encode_field(3, 0, AG.encode_varint(5))
                + AG.encode_field(8, 2, ts_pb)
                + AG.encode_field(12, 2, turn_group_uuid.encode("utf-8"))
                + AG.encode_field(20, 2, traj_ref)
                + AG.encode_field(26, 2, status_done)
            )
            body_pb = _encode_nested_pb([140, 2, 1], text_b)
        else:
            stype = 101
            t_type = "SYSTEM_MESSAGE"
            t_source = "SYSTEM"
            t_content = text
            meta_pb = (
                AG.encode_field(1, 2, ts_pb)
                + AG.encode_field(3, 0, AG.encode_varint(5))
                + AG.encode_field(8, 2, ts_pb)
                + AG.encode_field(12, 2, turn_group_uuid.encode("utf-8"))
                + AG.encode_field(20, 2, traj_ref)
                + AG.encode_field(26, 2, status_done)
            )
            body_pb = _encode_nested_pb([114, 1], text_b)

        payload_pb = (
            AG.encode_field(1, 0, AG.encode_varint(stype))
            + AG.encode_field(4, 0, AG.encode_varint(3))
            + AG.encode_field(5, 2, meta_pb)
            + body_pb
        )
        step_rows.append((idx, stype, 3, 0, meta_pb, None, None, None, None, payload_pb, 0))

        tr_obj: dict = {
            "step_index": idx,
            "source": t_source,
            "type": t_type,
            "status": "DONE",
            "created_at": iso,
            "content": t_content,
        }
        if think_txt:
            tr_obj["thinking"] = think_txt
        if it.get("tool") and kind == "tool":
            tr_obj["tool_calls"] = [{"name": str(it["tool"])}]
        transcript_lines.append(json.dumps(tr_obj, ensure_ascii=False))

    ws_msg = (AG.encode_field(1, 2, ws_uri.encode("utf-8")) + AG.encode_field(3, 2, b"")) if ws_uri else b""

    summary_meta_pb = b""
    if ws_msg:
        summary_meta_pb += AG.encode_field(1, 2, ws_msg)
    summary_meta_pb += AG.encode_field(2, 2, _pb_timestamp(first_sec))
    summary_meta_pb += AG.encode_field(3, 2, inst_uuid.encode("utf-8"))
    summary_meta_pb += AG.encode_field(6, 2, cid.encode("utf-8"))
    if ws_uri:
        summary_meta_pb += AG.encode_field(7, 2, ws_uri.encode("utf-8"))
    summary_meta_pb += AG.encode_field(18, 2, project_id.encode("utf-8"))

    traj_meta_blob = b""
    if ws_msg:
        traj_meta_blob += AG.encode_field(1, 2, ws_msg)
    traj_meta_blob += AG.encode_field(2, 2, _pb_timestamp(first_sec))
    traj_meta_blob += AG.encode_field(3, 2, inst_uuid.encode("utf-8"))
    traj_meta_blob += AG.encode_field(6, 2, cid.encode("utf-8"))
    if ws_uri:
        traj_meta_blob += AG.encode_field(7, 2, ws_uri.encode("utf-8"))
    if tpl["cap_f15"]:
        traj_meta_blob += AG.encode_field(15, 2, tpl["cap_f15"])
    traj_meta_blob += AG.encode_field(18, 2, project_id.encode("utf-8"))

    con = AG.connect_rw(db_path)
    try:
        con.execute("BEGIN")
        con.execute(
            "INSERT INTO trajectory_meta (trajectory_id, cascade_id, trajectory_type, source) VALUES (?, ?, ?, ?)",
            (traj_id, cid, 4, 17 if is_cli else 1),
        )
        con.execute(
            "INSERT OR REPLACE INTO trajectory_metadata_blob (id, data) VALUES ('main', ?)",
            (traj_meta_blob,),
        )
        con.executemany(
            "INSERT INTO steps (idx, step_type, status, has_subtrajectory, metadata, error_details, "
            "permissions, task_details, render_info, step_payload, step_format) "
            "VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            step_rows,
        )
        con.execute("COMMIT")
    except Exception:
        if con.in_transaction:
            con.execute("ROLLBACK")
        raise
    finally:
        con.close()

    AG.write_transcript_lines(cid, transcript_lines, full=True, store=store)
    AG.write_transcript_lines(cid, transcript_lines, full=False, store=store)
    _write_ag_annotation(store, cid, final_title, last_sec)

    ann_pb = (
        AG.encode_field(1, 2, final_title.encode("utf-8"))
        + AG.encode_field(7, 2, _pb_timestamp(last_sec))
    )
    raw_summary = (
        AG.encode_field(1, 2, final_title.encode("utf-8"))
        + AG.encode_field(2, 0, AG.encode_varint(len(step_rows)))
        + AG.encode_field(3, 2, _pb_timestamp(last_sec))
        + AG.encode_field(4, 2, traj_id.encode("utf-8"))
        + AG.encode_field(5, 0, AG.encode_varint(1))
        + AG.encode_field(7, 2, _pb_timestamp(first_sec))
    )
    if ws_msg:
        raw_summary += AG.encode_field(9, 2, ws_msg)
    raw_summary += AG.encode_field(10, 2, _pb_timestamp(last_user_sec))
    raw_summary += AG.encode_field(15, 2, ann_pb)
    if last_user_idx >= 0:
        raw_summary += AG.encode_field(16, 0, AG.encode_varint(last_user_idx))
    raw_summary += AG.encode_field(17, 2, summary_meta_pb)
    raw_summary += AG.encode_field(22, 0, AG.encode_varint(4))
    raw_summary += AG.encode_field(28, 0, AG.encode_varint(2 if is_cli else 4))

    preview_txt = first_user_preview or final_title

    store.summaries_db.parent.mkdir(parents=True, exist_ok=True)
    scon = sqlite3.connect(str(store.summaries_db), timeout=30)
    try:
        _ensure_ag_summaries_schema(scon)
        scon.execute("BEGIN IMMEDIATE")
        scon.execute(
            "INSERT OR REPLACE INTO conversation_summaries ("
            "conversation_id, title, preview, step_count, last_modified_time, workspace_uris, "
            "status, source, project_id, agent_name, parent_conversation_id, nesting_depth, "
            "battle_id, winning_conversation_id, not_fully_idle, killed, "
            "last_user_input_time, last_user_input_step_index, app_data_dir, raw_summary, group_id"
            ") VALUES (?, ?, ?, ?, ?, ?, ?, '', ?, '', '', 0, '', '', 0, 0, ?, ?, ?, ?, '')",
            (
                cid,
                final_title,
                preview_txt,
                len(step_rows),
                _ts_to_ag_dt(last_sec),
                ws_uris,
                "CASCADE_RUN_STATUS_IDLE",
                project_id,
                _ts_to_ag_dt(last_user_sec),
                last_user_idx,
                app_data_dir,
                raw_summary,
            ),
        )
        has_fts = scon.execute(
            "SELECT 1 FROM sqlite_master WHERE type='table' AND name='conversations_fts'"
        ).fetchone()
        if has_fts:
            scon.execute("DELETE FROM conversations_fts WHERE conversation_id = ?", (cid,))
            scon.execute(
                "INSERT INTO conversations_fts ("
                "conversation_id, chunk_index, chunk_first_step, chunk_last_step, "
                "last_modified_time, step_count, last_step_created_at, archived, "
                "user_step_byte_offsets, agent_step_byte_offsets, title, workspace, project, user_body, agent_body"
                ") VALUES (?, 0, 0, ?, ?, ?, ?, 0, ?, ?, ?, ?, ?, ?, ?)",
                (
                    cid,
                    max(0, len(step_rows) - 1),
                    _ts_to_ag_dt(last_sec),
                    len(step_rows),
                    _ts_to_ag_dt(last_sec),
                    json.dumps(user_offs),
                    json.dumps(agent_offs),
                    final_title,
                    raw_dir,
                    project_id,
                    user_body.decode("utf-8", "replace"),
                    agent_body.decode("utf-8", "replace"),
                ),
            )
        scon.execute("COMMIT")
    except Exception:
        if scon.in_transaction:
            scon.execute("ROLLBACK")
        raise
    finally:
        scon.close()

    _upsert_ag_summaries_proto(store, app_data_dir, cid, raw_summary)

    return {
        "provider": "antigravity_cli" if is_cli else "antigravity",
        "cid": cid,
        "title": final_title,
        "entries": len(step_rows),
    }


def _codex_template_meta(codex_home: Path) -> dict:
    tpl: dict = {
        "account_scoped": False,
        "creator_user_id": None,
        "creator_account_id": None,
        "originator": "Codex Desktop",
        "cli_version": "0.169.0-alpha.11.1",
        "source": "vscode",
        "model_provider": "openai",
        "sandbox_policy": '{"type":"disabled"}',
        "approval_mode": "never",
        "model": "gpt-6.1-sol",
        "reasoning_effort": "high",
        "cwd": str(Path.home()),
        "runtime_workspace_roots": [],
        "base_instructions": None,
        "turn_context": None,
        "rollout_path": None,
    }
    candidate_rollouts: list[str] = []
    stdb = codex_home / "state_5.sqlite"
    if stdb.is_file():
        try:
            con = sqlite3.connect(f"file:{stdb}?mode=ro", uri=True, timeout=5)
            try:
                cols = {c[1] for c in con.execute("PRAGMA table_info(threads)")}
                want = [c for c in (
                    "source", "model_provider", "cwd", "sandbox_policy",
                    "approval_mode", "cli_version", "originator",
                    "creator_user_id", "creator_account_id",
                    "model", "reasoning_effort", "rollout_path",
                ) if c in cols]
                if want:
                    where_opts = []
                    if "thread_source" in cols:
                        if "archived" in cols:
                            where_opts.append("WHERE thread_source = 'user' AND archived = 0")
                        where_opts.append("WHERE thread_source = 'user'")
                    where_opts.append("WHERE source NOT LIKE '{%'")
                    where_opts.append("")
                    rows = []
                    for clause in where_opts:
                        rows = con.execute(
                            f"SELECT {', '.join(want)} FROM threads {clause} ORDER BY updated_at DESC LIMIT 12"
                        ).fetchall()
                        if rows:
                            break
                    if rows:
                        for k, v in zip(want, rows[0]):
                            if v is not None and v != "":
                                tpl[k] = _strip_win_long_prefix(v) if k in ("cwd", "rollout_path") else v
                        if tpl.get("creator_user_id") or tpl.get("creator_account_id"):
                            tpl["account_scoped"] = True
                        if "rollout_path" in want:
                            rp_idx = want.index("rollout_path")
                            for r in rows:
                                if r[rp_idx]:
                                    candidate_rollouts.append(_strip_win_long_prefix(str(r[rp_idx])))
            finally:
                con.close()
        except Exception:
            pass

    for rpath_raw in candidate_rollouts:
        rp = Path(rpath_raw)
        if not rp.is_file():
            continue
        try:
            with rp.open("r", encoding="utf-8", errors="replace") as fh:
                for _ in range(35):
                    ln = fh.readline()
                    if not ln:
                        break
                    o = json.loads(ln)
                    rt = o.get("type")
                    pl = o.get("payload") or {}
                    if rt == "session_meta":
                        if tpl["base_instructions"] is None and isinstance(pl.get("base_instructions"), dict):
                            tpl["base_instructions"] = pl["base_instructions"]
                        if not tpl["runtime_workspace_roots"] and isinstance(pl.get("runtime_workspace_roots"), list):
                            tpl["runtime_workspace_roots"] = [
                                _strip_win_long_prefix(str(x)) for x in pl["runtime_workspace_roots"] if x
                            ]
                    elif rt == "turn_context" and tpl["turn_context"] is None:
                        tpl["turn_context"] = dict(pl)
                    if tpl["base_instructions"] is not None and tpl["turn_context"] is not None:
                        break
        except Exception:
            continue
        if tpl["base_instructions"] is not None and tpl["turn_context"] is not None:
            break
    return tpl


def _ensure_codex_projection_schema(con: sqlite3.Connection):
    con.execute(
        "CREATE TABLE IF NOT EXISTS thread_turns ("
        "thread_id TEXT NOT NULL, turn_id TEXT NOT NULL, rollout_ordinal INTEGER NOT NULL, "
        "status TEXT NOT NULL, error_json TEXT, started_at INTEGER, completed_at INTEGER, "
        "duration_ms INTEGER, first_user_item_id TEXT, final_agent_item_id TEXT, "
        "rollout_byte_offset INTEGER, rollout_end_ordinal INTEGER, rollout_end_byte_offset INTEGER, "
        "root_turn_id TEXT, PRIMARY KEY (thread_id, turn_id))"
    )
    con.execute(
        "CREATE TABLE IF NOT EXISTS thread_items ("
        "thread_id TEXT NOT NULL, turn_id TEXT NOT NULL, item_id TEXT NOT NULL, "
        "rollout_ordinal INTEGER NOT NULL, created_at_ms INTEGER NOT NULL, "
        "item_json TEXT NOT NULL, item_type TEXT NOT NULL DEFAULT '', "
        "updated_at_ordinal INTEGER NOT NULL DEFAULT 0, started_at_ms INTEGER, completed_at_ms INTEGER, "
        "PRIMARY KEY (thread_id, turn_id, item_id))"
    )
    con.execute(
        "CREATE TABLE IF NOT EXISTS thread_history_projection_state ("
        "thread_id TEXT PRIMARY KEY, next_rollout_byte_offset INTEGER NOT NULL, "
        "next_rollout_ordinal INTEGER NOT NULL)"
    )
    con.execute(
        "CREATE TABLE IF NOT EXISTS thread_realtime_items ("
        "thread_id TEXT NOT NULL, item_id TEXT NOT NULL, rollout_ordinal INTEGER NOT NULL, "
        "created_at_ms INTEGER NOT NULL, item_type TEXT NOT NULL, item_json TEXT NOT NULL, "
        "PRIMARY KEY (thread_id, item_id))"
    )


def _update_codex_global_state(codex_home: Path, tid: str, cwd: str, workspace_roots: list[str]):
    gs_path = codex_home / ".codex-global-state.json"
    if not gs_path.is_file():
        return
    try:
        data = json.loads(gs_path.read_text(encoding="utf-8", errors="replace"))
        if not isinstance(data, dict):
            return
        norm_cwd = os.path.normcase(os.path.normpath(_strip_win_long_prefix(cwd)))
        matched_pid = None
        local_projects = data.get("local-projects")
        if isinstance(local_projects, dict):
            for pid, pinfo in local_projects.items():
                if not isinstance(pinfo, dict):
                    continue
                for r in (pinfo.get("rootPaths") or []):
                    if os.path.normcase(os.path.normpath(_strip_win_long_prefix(str(r)))) == norm_cwd:
                        matched_pid = pid
                        break
                if matched_pid:
                    break

        if matched_pid:
            tpa = data.get("thread-project-assignments")
            if not isinstance(tpa, dict):
                tpa = {}
                data["thread-project-assignments"] = tpa
            tpa[tid] = {"projectKind": "local", "projectId": matched_pid}
        else:
            p_ids = data.get("projectless-thread-ids")
            if not isinstance(p_ids, list):
                p_ids = []
                data["projectless-thread-ids"] = p_ids
            if tid not in p_ids:
                p_ids.append(tid)

        ws_hint = workspace_roots[0] if workspace_roots else _strip_win_long_prefix(cwd)
        out_dir = str(Path(_strip_win_long_prefix(cwd)) / "outputs")
        for k, v in list(data.items()):
            if k.startswith("electron-owner-value:thread-workspace-root-hints:") and isinstance(v, dict):
                v[tid] = ws_hint
            elif not matched_pid and k.startswith("electron-owner-value:thread-projectless-output-directories:") and isinstance(v, dict):
                v[tid] = out_dir

        tmp = gs_path.with_suffix(".json.tmp")
        tmp.write_text(json.dumps(data, ensure_ascii=False), encoding="utf-8")
        tmp.replace(gs_path)
    except Exception:
        pass


def _group_codex_turns(raw_entries: list[dict], timestamps: list[float]) -> list[list[tuple[dict, float]]]:
    turns: list[list[tuple[dict, float]]] = []
    for it, sec in zip(raw_entries, timestamps):
        kind = str(it.get("kind") or "assistant")
        if kind == "user" or not turns:
            turns.append([(it, sec)])
        else:
            turns[-1].append((it, sec))
    return turns


def import_to_codex(
    doc: dict,
    *,
    codex_home: Path | None = None,
    title: str | None = None,
    include_tools: bool = True,
) -> dict:
    raw_entries = _filter_entries(doc.get("entries") or [], include_tools=include_tools)
    if not raw_entries:
        raise ValueError("В файле нет сообщений для импорта.")

    is_default_home = codex_home is None
    home = Path(codex_home) if codex_home else CX.CODEX_HOME
    sessions_dir = home / "sessions"
    session_index = home / "session_index.jsonl"
    state_db = home / "state_5.sqlite"
    proj_db = home / "thread_history_1.sqlite"

    meta = doc.get("meta") or {}
    final_title = _derive_title(raw_entries, title or meta.get("title"))
    tpl = _codex_template_meta(home)

    now_sec = time.time()
    now_dt = datetime.fromtimestamp(now_sec, tz=timezone.utc)
    tid = _uuid7(int(now_sec * 1000))

    ws_roots = list(tpl.get("runtime_workspace_roots") or [])
    meta_dir = _strip_win_long_prefix(str(meta.get("directory") or "").strip())
    if meta_dir and not _is_ephemeral_workspace(meta_dir) and Path(meta_dir).is_dir():
        cwd = os.path.normpath(meta_dir)
    elif is_default_home and ws_roots and Path(ws_roots[0]).is_dir():
        ws_dir = Path(ws_roots[0]) / f"{now_dt.year:04d}-{now_dt.month:02d}-{now_dt.day:02d}" / f"imported-{tid[:8]}"
        (ws_dir / "outputs").mkdir(parents=True, exist_ok=True)
        cwd = str(ws_dir)
    elif not is_default_home:
        ws_dir = home / "workspaces" / f"imported-{tid[:8]}"
        (ws_dir / "outputs").mkdir(parents=True, exist_ok=True)
        cwd = str(ws_dir)
    else:
        tpl_cwd = _strip_win_long_prefix(str(tpl.get("cwd") or ""))
        cwd = tpl_cwd if (tpl_cwd and Path(tpl_cwd).is_dir()) else str(Path.home())

    if not ws_roots:
        ws_roots = [str(Path(cwd).parent) if Path(cwd).parent != Path(cwd) else cwd]

    day_dir = sessions_dir / f"{now_dt.year:04d}" / f"{now_dt.month:02d}" / f"{now_dt.day:02d}"
    day_dir.mkdir(parents=True, exist_ok=True)
    ts_slug = now_dt.strftime("%Y-%m-%dT%H-%M-%S")
    rollout_path = day_dir / f"rollout-{ts_slug}-{tid}.jsonl"

    timestamps = _monotonic_timestamps(raw_entries, now_sec)
    turns = _group_codex_turns(raw_entries, timestamps)
    first_sec = timestamps[0]
    last_sec = timestamps[-1]
    first_iso = _ts_to_iso(first_sec)

    raw_chunks: list[bytes] = []
    ordinal = 0
    byte_offset = 0

    def emit(ts_str: str, rec_type: str, payload: dict) -> tuple[int, int, int]:
        nonlocal ordinal, byte_offset
        cur_ord = ordinal
        start_off = byte_offset
        line_b = (json.dumps({
            "timestamp": ts_str,
            "ordinal": cur_ord,
            "type": rec_type,
            "payload": payload,
        }, ensure_ascii=False) + "\n").encode("utf-8")
        raw_chunks.append(line_b)
        ordinal += 1
        byte_offset += len(line_b)
        return cur_ord, start_off, byte_offset

    sm_payload: dict = {
        "account_scoped": bool(tpl.get("account_scoped")),
        "session_id": tid,
        "id": tid,
        "timestamp": first_iso,
        "cwd": cwd,
        "runtime_workspace_roots": ws_roots,
        "originator": tpl["originator"],
        "cli_version": tpl["cli_version"],
        "source": tpl["source"],
        "thread_source": "user",
        "model_provider": tpl["model_provider"],
        "history_mode": "paginated",
        "context_window": {"window_id": _uuid7(int(first_sec * 1000))},
    }
    if tpl.get("creator_user_id"):
        sm_payload["creator_user_id"] = tpl["creator_user_id"]
    if tpl.get("creator_account_id"):
        sm_payload["creator_account_id"] = tpl["creator_account_id"]
    if tpl.get("base_instructions"):
        sm_payload["base_instructions"] = tpl["base_instructions"]
    emit(first_iso, "session_meta", sm_payload)

    tpl_tc = tpl.get("turn_context") if isinstance(tpl.get("turn_context"), dict) else None
    default_model = str(tpl.get("model") or "gpt-6.1-sol")
    default_effort = str(tpl.get("reasoning_effort") or "high")

    proj_turns: list[tuple] = []
    proj_items: list[tuple] = []
    first_user_msg = ""

    for turn_entries in turns:
        t_start_sec = turn_entries[0][1]
        t_end_sec = turn_entries[-1][1]
        t_start_ms = int(t_start_sec * 1000)
        t_end_ms = max(t_start_ms + 1, int(t_end_sec * 1000))
        t_start_iso = _ts_to_iso(t_start_sec)
        turn_id = _uuid7(t_start_ms)

        t_start_ord, t_start_off, _ = emit(t_start_iso, "event_msg", {
            "type": "task_started",
            "turn_attribution": {
                "turn_id": turn_id,
                "turn_trigger": "composer",
                "parent_turn_id": None,
                "initiating_agent_path": None,
                "root_turn_id": turn_id,
            },
            "turn_id": turn_id,
            "root_turn_id": turn_id,
            "started_at": int(t_start_sec),
            "model_context_window": 475000,
            "collaboration_mode_kind": "default",
        })

        if tpl_tc:
            tc_payload = dict(tpl_tc)
            tc_payload["turn_id"] = turn_id
            tc_payload["root_turn_id"] = turn_id
            tc_payload["cwd"] = cwd
        else:
            tc_payload = {
                "turn_id": turn_id,
                "root_turn_id": turn_id,
                "disabled_plugin_ids": [],
                "cwd": cwd,
                "approval_policy": tpl["approval_mode"],
                "approvals_reviewer": "user",
                "permission_profile": {"type": "disabled"},
                "active_permission_profile": {"id": ":danger-full-access"},
                "model": default_model,
                "collaboration_mode": {
                    "mode": "default",
                    "settings": {"model": default_model, "reasoning_effort": default_effort},
                },
                "multi_agent_version": "v1",
                "realtime_active": False,
                "effort": default_effort,
                "summary": "none",
                "sandbox_policy": {"type": "danger-full-access"},
            }
        emit(t_start_iso, "turn_context", tc_payload)

        last_asst_idx = -1
        for i_pos, (it_e, _) in enumerate(turn_entries):
            if str(it_e.get("kind") or "assistant") == "assistant":
                last_asst_idx = i_pos

        first_user_item_id: str | None = None
        final_agent_item_id: str | None = None
        last_agent_text = ""
        turn_error: dict | None = None

        for i_pos, (it, sec) in enumerate(turn_entries):
            ms = int(sec * 1000)
            iso = _ts_to_iso(sec)
            kind = str(it.get("kind") or "assistant")
            subtype = str(it.get("subtype") or "")
            text = str(it.get("text") or "")

            if kind == "user":
                if not first_user_msg:
                    first_user_msg = text[:400]
                user_item_id = _uuid7(ms)
                client_id = str(uuid.uuid4())
                if first_user_item_id is None:
                    first_user_item_id = user_item_id
                emit(iso, "response_item", {
                    "type": "message",
                    "id": f"msg_{_uuid7(ms)}",
                    "role": "user",
                    "content": [{"type": "input_text", "text": text}],
                    "internal_chat_message_metadata_passthrough": {
                        "turn_id": turn_id,
                        "create_time": sec,
                        "content_item_kinds": ["user.text"],
                        "content_item_metadata": [{"provenance": {"type": "user"}}],
                    },
                })
                ev_ord, _, _ = emit(iso, "event_msg", {
                    "type": "item_completed",
                    "thread_id": tid,
                    "turn_id": turn_id,
                    "item": {
                        "type": "UserMessage",
                        "id": user_item_id,
                        "client_id": client_id,
                        "content": [{"type": "text", "text": text, "text_elements": []}],
                    },
                    "started_at_ms": ms,
                    "completed_at_ms": ms,
                })
                proj_items.append((
                    tid, turn_id, user_item_id, ev_ord, ms,
                    json.dumps({
                        "type": "userMessage",
                        "id": user_item_id,
                        "clientId": client_id,
                        "content": [{"type": "text", "text": text, "text_elements": []}],
                    }, ensure_ascii=False),
                    "userMessage", ev_ord, ms, ms,
                ))
            elif kind == "assistant":
                phase = "final_answer" if i_pos == last_asst_idx else "commentary"
                agent_msg_id = f"msg_{uuid.uuid4().hex}{uuid.uuid4().hex[:16]}"
                final_agent_item_id = agent_msg_id
                last_agent_text = text
                ev_ord, _, _ = emit(iso, "event_msg", {
                    "type": "item_completed",
                    "thread_id": tid,
                    "turn_id": turn_id,
                    "item": {
                        "type": "AgentMessage",
                        "id": agent_msg_id,
                        "content": [{"type": "Text", "text": text}],
                        "phase": phase,
                    },
                    "content_token_count": max(1, len(text) // 4),
                    "started_at_ms": ms,
                    "completed_at_ms": ms,
                })
                emit(iso, "response_item", {
                    "type": "message",
                    "status": "completed",
                    "id": agent_msg_id,
                    "role": "assistant",
                    "content": [{"type": "output_text", "text": text, "annotations": [], "logprobs": []}],
                    "phase": phase,
                    "internal_chat_message_metadata_passthrough": {
                        "turn_id": turn_id,
                        "create_time": sec,
                        "content_item_kinds": ["unknown"],
                    },
                })
                proj_items.append((
                    tid, turn_id, agent_msg_id, ev_ord, ms,
                    json.dumps({
                        "type": "agentMessage",
                        "moderation": None,
                        "id": agent_msg_id,
                        "text": text,
                        "phase": phase,
                        "memoryCitation": None,
                        "delivery": None,
                        "questions": None,
                    }, ensure_ascii=False),
                    "agentMessage", ev_ord, ms, ms,
                ))
            elif kind == "tool" and subtype == "thinking":
                rs_id = f"rs_{uuid.uuid4().hex}"
                ev_ord, _, _ = emit(iso, "event_msg", {
                    "type": "item_completed",
                    "thread_id": tid,
                    "turn_id": turn_id,
                    "item": {
                        "type": "Reasoning",
                        "id": rs_id,
                        "summary_text": [text],
                        "raw_content": [],
                    },
                    "started_at_ms": ms,
                    "completed_at_ms": ms,
                })
                proj_items.append((
                    tid, turn_id, rs_id, ev_ord, ms,
                    json.dumps({
                        "type": "reasoning",
                        "id": rs_id,
                        "summary": [text],
                        "content": [],
                    }, ensure_ascii=False),
                    "reasoning", ev_ord, ms, ms,
                ))
            elif kind == "tool":
                tname = str(it.get("tool") or it.get("label") or "tool")
                tool_text = f"[{tname}]\n{text}" if text else f"[{tname}]"
                agent_msg_id = f"msg_{uuid.uuid4().hex}{uuid.uuid4().hex[:16]}"
                ev_ord, _, _ = emit(iso, "event_msg", {
                    "type": "item_completed",
                    "thread_id": tid,
                    "turn_id": turn_id,
                    "item": {
                        "type": "AgentMessage",
                        "id": agent_msg_id,
                        "content": [{"type": "Text", "text": tool_text}],
                        "phase": "commentary",
                    },
                    "content_token_count": max(1, len(tool_text) // 4),
                    "started_at_ms": ms,
                    "completed_at_ms": ms,
                })
                emit(iso, "response_item", {
                    "type": "message",
                    "status": "completed",
                    "id": agent_msg_id,
                    "role": "assistant",
                    "content": [{"type": "output_text", "text": tool_text, "annotations": [], "logprobs": []}],
                    "phase": "commentary",
                })
                proj_items.append((
                    tid, turn_id, agent_msg_id, ev_ord, ms,
                    json.dumps({
                        "type": "agentMessage",
                        "moderation": None,
                        "id": agent_msg_id,
                        "text": tool_text,
                        "phase": "commentary",
                        "memoryCitation": None,
                        "delivery": None,
                        "questions": None,
                    }, ensure_ascii=False),
                    "agentMessage", ev_ord, ms, ms,
                ))
            elif kind == "compact":
                comp_id = f"cmp_{uuid.uuid4().hex[:16]}"
                ev_ord, _, _ = emit(iso, "event_msg", {
                    "type": "item_completed",
                    "thread_id": tid,
                    "turn_id": turn_id,
                    "item": {"type": "ContextCompaction", "id": comp_id},
                    "started_at_ms": ms,
                    "completed_at_ms": ms,
                })
                emit(iso, "response_item", {
                    "type": "message",
                    "id": f"msg_{_uuid7(ms)}",
                    "role": "developer",
                    "content": [{"type": "input_text", "text": text}],
                })
                proj_items.append((
                    tid, turn_id, comp_id, ev_ord, ms,
                    json.dumps({"type": "contextCompaction", "id": comp_id}, ensure_ascii=False),
                    "contextCompaction", ev_ord, ms, ms,
                ))
            elif kind == "error":
                turn_error = {"message": text, "codex_error_info": "other"}
            else:
                emit(iso, "response_item", {
                    "type": "message",
                    "id": f"msg_{_uuid7(ms)}",
                    "role": "developer",
                    "content": [{"type": "input_text", "text": text}],
                })

        tc_ev: dict = {
            "type": "task_complete",
            "root_turn_id": turn_id,
            "turn_id": turn_id,
            "last_agent_message": last_agent_text,
            "started_at": int(t_start_sec),
            "completed_at": int(t_end_sec),
            "duration_ms": max(1, t_end_ms - t_start_ms),
        }
        if turn_error:
            tc_ev["error"] = turn_error
        t_end_ord, _, t_end_off = emit(_ts_to_iso(t_end_sec), "event_msg", tc_ev)
        proj_turns.append((
            tid,
            turn_id,
            t_start_ord,
            "failed" if turn_error else "completed",
            json.dumps(turn_error, ensure_ascii=False) if turn_error else None,
            int(t_start_sec),
            int(t_end_sec),
            max(1, t_end_ms - t_start_ms),
            first_user_item_id,
            final_agent_item_id,
            t_start_off,
            t_end_ord,
            t_end_off,
            turn_id,
        ))

    full_bytes = b"".join(raw_chunks)
    tmp = rollout_path.with_suffix(".jsonl.tmp")
    tmp.write_bytes(full_bytes)
    tmp.replace(rollout_path)

    idx_lines = (
        session_index.read_text(encoding="utf-8", errors="replace").splitlines()
        if session_index.is_file() else []
    )
    idx_lines.append(json.dumps({
        "id": tid,
        "thread_name": final_title,
        "updated_at": _ts_to_iso(max(last_sec, now_sec)),
    }, ensure_ascii=False))
    idx_tmp = session_index.with_suffix(".jsonl.tmp")
    idx_tmp.write_text("\n".join(idx_lines) + "\n", encoding="utf-8")
    idx_tmp.replace(session_index)

    if state_db.is_file():
        con = sqlite3.connect(str(state_db), timeout=15)
        try:
            col_info = con.execute("PRAGMA table_info(threads)").fetchall()
            rpath_str = ("\\\\?\\" + str(rollout_path.resolve())) if os.name == "nt" else str(rollout_path.resolve())
            cwd_db = ("\\\\?\\" + str(Path(cwd).resolve())) if (os.name == "nt" and not cwd.startswith("\\\\")) else cwd
            c_sec = int(first_sec)
            u_sec = int(max(last_sec, now_sec))
            defaults = {
                "id": tid,
                "rollout_path": rpath_str,
                "created_at": c_sec,
                "updated_at": u_sec,
                "source": tpl["source"],
                "model_provider": tpl["model_provider"],
                "cwd": cwd_db,
                "title": final_title,
                "sandbox_policy": tpl["sandbox_policy"],
                "approval_mode": tpl["approval_mode"],
                "tokens_used": 0,
                "has_user_event": 0,
                "archived": 0,
                "cli_version": tpl["cli_version"],
                "first_user_message": first_user_msg or final_title,
                "memory_mode": "enabled",
                "model": default_model,
                "reasoning_effort": default_effort,
                "created_at_ms": c_sec * 1000,
                "updated_at_ms": u_sec * 1000,
                "thread_source": "user",
                "preview": (first_user_msg or final_title)[:200],
                "recency_at": u_sec,
                "recency_at_ms": u_sec * 1000,
                "history_mode": "paginated",
                "name": final_title,
                "is_pinned": 0,
                "originator": tpl["originator"],
                "creator_user_id": tpl.get("creator_user_id"),
                "creator_account_id": tpl.get("creator_account_id"),
            }
            use_cols = []
            use_vals = []
            for _cid_i, cname, ctype, notnull, dflt, _pk in col_info:
                if cname in defaults:
                    use_cols.append(cname)
                    use_vals.append(defaults[cname])
                elif notnull and dflt is None:
                    use_cols.append(cname)
                    use_vals.append(0 if "INT" in (ctype or "").upper() else "")
            if use_cols:
                ph = ", ".join("?" for _ in use_cols)
                con.execute("BEGIN IMMEDIATE")
                con.execute(
                    f"INSERT OR REPLACE INTO threads ({', '.join(use_cols)}) VALUES ({ph})",
                    use_vals,
                )
                con.execute("COMMIT")
        except Exception:
            if con.in_transaction:
                con.execute("ROLLBACK")
            raise
        finally:
            con.close()

    if proj_db.is_file() or not is_default_home:
        pcon = sqlite3.connect(str(proj_db), timeout=15)
        try:
            _ensure_codex_projection_schema(pcon)
            pcon.execute("BEGIN IMMEDIATE")
            pcon.execute("DELETE FROM thread_items WHERE thread_id = ?", (tid,))
            pcon.execute("DELETE FROM thread_turns WHERE thread_id = ?", (tid,))
            pcon.executemany(
                "INSERT OR REPLACE INTO thread_turns ("
                "thread_id, turn_id, rollout_ordinal, status, error_json, started_at, completed_at, "
                "duration_ms, first_user_item_id, final_agent_item_id, rollout_byte_offset, "
                "rollout_end_ordinal, rollout_end_byte_offset, root_turn_id"
                ") VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                proj_turns,
            )
            pcon.executemany(
                "INSERT OR REPLACE INTO thread_items ("
                "thread_id, turn_id, item_id, rollout_ordinal, created_at_ms, item_json, "
                "item_type, updated_at_ordinal, started_at_ms, completed_at_ms"
                ") VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                proj_items,
            )
            pcon.execute(
                "INSERT OR REPLACE INTO thread_history_projection_state "
                "(thread_id, next_rollout_byte_offset, next_rollout_ordinal) VALUES (?, ?, ?)",
                (tid, len(full_bytes), ordinal),
            )
            pcon.execute("COMMIT")
        except Exception:
            if pcon.in_transaction:
                pcon.execute("ROLLBACK")
            raise
        finally:
            pcon.close()

    _update_codex_global_state(home, tid, cwd, ws_roots)

    CX._FILE_CACHE.clear()
    CX._LIST_CACHE.clear()
    return {
        "provider": "codex",
        "cid": tid,
        "title": final_title,
        "entries": len(raw_entries),
    }


def _encode_claude_slug(cwd: str) -> str:
    trimmed = cwd.strip()
    if len(trimmed) > 1:
        trimmed = trimmed.rstrip("\\/")
    if len(trimmed) == 2 and trimmed[1] == ":":
        return f"{trimmed[0]}--"
    if len(trimmed) >= 3 and trimmed[1] == ":":
        drive = trimmed[0]
        rest = re.sub(r"[^A-Za-z0-9]", "-", trimmed[2:].lstrip("\\/"))
        return f"{drive}--{rest}" if rest else f"{drive}--"
    clean = re.sub(r"[^A-Za-z0-9]", "-", trimmed)
    return clean or "imported"


def _claude_template_meta(claude_home: Path) -> dict:
    tpl = {
        "version": "2.1.325",
        "model": "claude-sonnet-5-5",
        "entrypoint": "claude-desktop",
        "cwd": str(Path.home()),
    }
    pdir = claude_home / "projects"
    if not pdir.is_dir():
        return tpl
    candidates = sorted(
        pdir.glob("*/*.jsonl"),
        key=lambda p: p.stat().st_mtime if p.exists() else 0.0,
        reverse=True,
    )
    for fp in candidates[:5]:
        try:
            with open(win_long_path(fp), "r", encoding="utf-8", errors="replace") as fh:
                for _ in range(80):
                    ln = fh.readline()
                    if not ln:
                        break
                    o = json.loads(ln)
                    if o.get("version"):
                        tpl["version"] = str(o["version"])
                    if o.get("entrypoint"):
                        tpl["entrypoint"] = str(o["entrypoint"])
                    if o.get("cwd") and Path(str(o["cwd"])).is_dir():
                        tpl["cwd"] = str(o["cwd"])
                    if o.get("type") == "assistant":
                        m = (o.get("message") or {}).get("model")
                        if m:
                            tpl["model"] = str(m)
                            return tpl
        except Exception:
            continue
    return tpl


def _find_claude_desktop_context() -> tuple[Path | None, Path | None, dict | None]:
    user_home = Path.home()
    roots = [
        user_home / "AppData" / "Local" / "Claude-Data",
        user_home / "AppData" / "Roaming" / "Claude",
        user_home / "Library" / "Application Support" / "Claude",
        user_home / ".config" / "Claude",
    ]
    for root in roots:
        if not root.is_dir():
            continue
        candidates: list[Path] = []
        if (root / "claude-code-sessions").is_dir():
            candidates.append(root)
        else:
            for sub in root.iterdir():
                if sub.is_dir() and (sub / "claude-code-sessions").is_dir():
                    candidates.append(sub)
        for base in candidates:
            ccs = base / "claude-code-sessions"
            local_files = sorted(
                ccs.rglob("local_*.json"),
                key=lambda p: p.stat().st_mtime if p.exists() else 0.0,
                reverse=True,
            )
            if local_files:
                newest = local_files[0]
                tpl_obj = None
                try:
                    tpl_obj = json.loads(newest.read_text(encoding="utf-8", errors="replace"))
                except Exception:
                    pass
                rel = newest.parent.relative_to(ccs)
                scratch_root = base / "scratch-workspaces" / rel
                return newest.parent, (scratch_root if scratch_root.is_dir() else None), tpl_obj
    return None, None, None


def _write_claude_desktop_session(
    sessions_dir: Path,
    tpl_obj: dict | None,
    *,
    cli_sid: str,
    cwd: str,
    title: str,
    first_ms: int,
    last_ms: int,
    now_ms: int,
    completed_turns: int,
    last_asst_uuid: str | None,
    last_asst_text: str,
    model: str,
):
    local_id = f"local_{uuid.uuid4()}"
    desktop_model = (tpl_obj or {}).get("model") or model
    obj: dict = {
        "sessionId": local_id,
        "cliSessionId": cli_sid,
        "cwd": cwd,
        "originCwd": cwd,
        "gitAnchorsLookupOnly": True,
        "gitAnchorsFolderRealpath": cwd,
        "folderPlace": {"cwd": cwd, "realPath": cwd},
        "lastFocusedAt": now_ms,
        "createdAt": first_ms,
        "lastActivityAt": now_ms,
        "lastTurnEdgeAt": last_ms,
        "model": desktop_model,
        "effort": (tpl_obj or {}).get("effort") or "high",
        "sessionSettings": {"ultracode": False},
        "isArchived": False,
        "title": title,
        "titleSource": "auto",
        "permissionMode": (tpl_obj or {}).get("permissionMode") or "auto",
        "chromePermissionMode": "skip_all_permission_checks",
        "completedTurns": max(1, completed_turns),
        "titleTurn": 0,
        "lastAssistantUuid": last_asst_uuid,
        "lastAssistantCliSessionId": cli_sid,
        "interruptedAwaitingPerson": None,
        "lastSpawnRootDetected": False,
        "autoChosenInApp": True,
        "scratchPromptRecents": [],
        "envScopeId": "builtin_local",
        "startedFromEnvironmentId": None,
        "remoteControlAutoEligible": False,
        "steeredByRemoteClient": False,
        "latestUserFrameAt": last_ms,
        "alwaysAllowedReasons": [],
        "sessionPermissionUpdates": [],
        "classifierSummaryEnabled": True,
        "reportFindingsCard": True,
        "turnBoxDeclared": True,
        "setupTools": True,
        "turnBoxMounted": True,
        "autoModeServerFallbackPrompt": True,
        "spawnSeed": {},
    }
    if last_asst_uuid:
        obj["postTurnSummary"] = {
            "status_category": "review_ready",
            "status_detail": " ".join(last_asst_text.split())[:220] or title,
            "needs_action": "",
            "summarizes_uuid": last_asst_uuid,
        }
        obj["postTurnSummaryFor"] = last_asst_uuid
    if isinstance(tpl_obj, dict) and isinstance(tpl_obj.get("toolSurfaceSnapshot"), dict):
        snap = dict(tpl_obj["toolSurfaceSnapshot"])
        snap["recordedAt"] = now_ms
        obj["toolSurfaceSnapshot"] = snap
    sessions_dir.mkdir(parents=True, exist_ok=True)
    atomic_write_text(sessions_dir / f"{local_id}.json", json.dumps(obj, ensure_ascii=False))


def import_to_claude(
    doc: dict,
    *,
    claude_home: Path | None = None,
    desktop_sessions_dir: Path | None = None,
    title: str | None = None,
    include_tools: bool = True,
) -> dict:
    raw_entries = _filter_entries(doc.get("entries") or [], include_tools=include_tools)
    if not raw_entries:
        raise ValueError("В файле нет сообщений для импорта.")

    is_default_home = claude_home is None
    home = Path(claude_home) if claude_home else CL.CLAUDE_HOME
    projects_dir = home / "projects"
    history_jsonl = home / "history.jsonl"

    meta = doc.get("meta") or {}
    final_title = _derive_title(raw_entries, title or meta.get("title"))
    tpl = _claude_template_meta(home)
    version = tpl["version"]
    model = tpl["model"]
    entrypoint = tpl["entrypoint"]

    dt_sessions_dir = desktop_sessions_dir
    dt_scratch_root: Path | None = None
    dt_tpl: dict | None = None
    if is_default_home and dt_sessions_dir is None:
        dt_sessions_dir, dt_scratch_root, dt_tpl = _find_claude_desktop_context()

    now_sec = time.time()
    now_dt = datetime.fromtimestamp(now_sec, tz=timezone.utc)
    meta_dir = _strip_win_long_prefix(str(meta.get("directory") or "").strip())
    if meta_dir and not _is_ephemeral_workspace(meta_dir) and Path(meta_dir).is_dir():
        cwd = os.path.normpath(meta_dir)
    elif is_default_home and dt_scratch_root and dt_scratch_root.is_dir():
        sc_dir = dt_scratch_root / f"scratch-{now_dt.year:04d}-{now_dt.month:02d}-{now_dt.day:02d}-{uuid.uuid4().hex[:6]}"
        sc_dir.mkdir(parents=True, exist_ok=True)
        cwd = str(sc_dir)
    elif not is_default_home:
        sc_dir = home / "workspaces" / f"imported-{uuid.uuid4().hex[:6]}"
        sc_dir.mkdir(parents=True, exist_ok=True)
        cwd = str(sc_dir)
    else:
        cwd = str(Path.home())

    slug = _encode_claude_slug(cwd)
    sid = str(uuid.uuid4())
    proj_dir = projects_dir / slug
    os.makedirs(win_long_path(proj_dir), exist_ok=True)
    target = proj_dir / f"{sid}.jsonl"

    timestamps = _monotonic_timestamps(raw_entries, now_sec)
    first_sec = timestamps[0]
    last_sec = timestamps[-1]
    first_iso = _ts_to_iso(first_sec)

    lines: list[str] = [
        json.dumps({"type": "queue-operation", "operation": "enqueue", "timestamp": first_iso, "sessionId": sid}, ensure_ascii=False),
        json.dumps({"type": "queue-operation", "operation": "dequeue", "timestamp": first_iso, "sessionId": sid}, ensure_ascii=False),
    ]

    prev_uuid: str | None = None
    last_asst_uuid: str | None = None
    last_asst_text = ""
    last_user_text = ""
    prompt_idx = 0
    cur_prompt_id = str(uuid.uuid4())
    client_plat = "desktop_app" if entrypoint == "claude-desktop" else "cli"

    def _asst_usage(txt: str) -> dict:
        return {
            "input_tokens": 0,
            "cache_creation_input_tokens": 0,
            "cache_read_input_tokens": 0,
            "output_tokens": max(1, len(txt) // 4),
            "output_tokens_details": {"thinking_tokens": 0},
            "server_tool_use": {"web_search_requests": 0, "web_fetch_requests": 0},
            "service_tier": "standard",
        }

    for it, sec in zip(raw_entries, timestamps):
        iso = _ts_to_iso(sec)
        cur_uuid = str(uuid.uuid4())
        kind = str(it.get("kind") or "assistant")
        subtype = str(it.get("subtype") or "")
        text = str(it.get("text") or "")

        if kind == "user":
            prompt_idx += 1
            cur_prompt_id = str(uuid.uuid4())
            last_user_text = text
            rec = {
                "parentUuid": prev_uuid,
                "isSidechain": False,
                "promptId": cur_prompt_id,
                "type": "user",
                "message": {"role": "user", "content": text},
                "uuid": cur_uuid,
                "timestamp": iso,
                "permissionMode": "auto",
                "origin": {"kind": "human"},
                "promptSource": "sdk",
                "turnOrigin": "human",
                "turnPosition": {"promptIndex": prompt_idx, "turnIndex": prompt_idx},
                "clientPlatform": client_plat,
                "turnAttributionKey": cur_uuid,
                "userType": "external",
                "entrypoint": entrypoint,
                "cwd": cwd,
                "sessionId": sid,
                "version": version,
                "gitBranch": "",
            }
            lines.append(json.dumps(rec, ensure_ascii=False))
            prev_uuid = cur_uuid
        elif kind == "assistant":
            rec = {
                "parentUuid": prev_uuid,
                "isSidechain": False,
                "message": {
                    "model": model,
                    "id": f"msg_{cur_uuid.replace('-', '')[:24]}",
                    "type": "message",
                    "role": "assistant",
                    "content": [{"type": "text", "text": text}],
                    "container": None,
                    "stop_reason": "end_turn",
                    "stop_sequence": None,
                    "usage": _asst_usage(text),
                },
                "requestId": f"req_{cur_uuid.replace('-', '')[:24]}",
                "requestedModel": model,
                "type": "assistant",
                "uuid": cur_uuid,
                "timestamp": iso,
                "madeBy": "model",
                "userType": "external",
                "entrypoint": entrypoint,
                "cwd": cwd,
                "sessionId": sid,
                "version": version,
                "gitBranch": "",
            }
            lines.append(json.dumps(rec, ensure_ascii=False))
            prev_uuid = cur_uuid
            last_asst_uuid = cur_uuid
            last_asst_text = text
        elif kind == "tool" and subtype == "thinking":
            think_text = f"[Thinking]\n{text}" if text else "[Thinking]"
            rec = {
                "parentUuid": prev_uuid,
                "isSidechain": False,
                "message": {
                    "model": model,
                    "id": f"msg_{cur_uuid.replace('-', '')[:24]}",
                    "type": "message",
                    "role": "assistant",
                    "content": [{"type": "text", "text": think_text}],
                    "container": None,
                    "stop_reason": "end_turn",
                    "stop_sequence": None,
                    "usage": _asst_usage(think_text),
                },
                "requestId": f"req_{cur_uuid.replace('-', '')[:24]}",
                "requestedModel": model,
                "type": "assistant",
                "uuid": cur_uuid,
                "timestamp": iso,
                "madeBy": "model",
                "userType": "external",
                "entrypoint": entrypoint,
                "cwd": cwd,
                "sessionId": sid,
                "version": version,
                "gitBranch": "",
            }
            lines.append(json.dumps(rec, ensure_ascii=False))
            prev_uuid = cur_uuid
            last_asst_uuid = cur_uuid
        elif kind == "tool":
            raw_tname = str(it.get("tool") or it.get("label") or "tool")
            safe_tname = re.sub(r"[^A-Za-z0-9_-]", "_", raw_tname)[:64] or "tool"
            toolu_id = f"toolu_{cur_uuid.replace('-', '')[:24]}"
            rec_use = {
                "parentUuid": prev_uuid,
                "isSidechain": False,
                "message": {
                    "model": model,
                    "id": f"msg_{cur_uuid.replace('-', '')[:24]}",
                    "type": "message",
                    "role": "assistant",
                    "content": [{
                        "type": "tool_use",
                        "id": toolu_id,
                        "name": safe_tname,
                        "input": {"summary": text[:300]},
                    }],
                    "container": None,
                    "stop_reason": "tool_use",
                    "stop_sequence": None,
                    "usage": _asst_usage(text),
                },
                "requestId": f"req_{cur_uuid.replace('-', '')[:24]}",
                "requestedModel": model,
                "type": "assistant",
                "uuid": cur_uuid,
                "timestamp": iso,
                "madeBy": "model",
                "userType": "external",
                "entrypoint": entrypoint,
                "cwd": cwd,
                "sessionId": sid,
                "version": version,
                "gitBranch": "",
            }
            lines.append(json.dumps(rec_use, ensure_ascii=False))
            res_uuid = str(uuid.uuid4())
            rec_res = {
                "parentUuid": cur_uuid,
                "isSidechain": False,
                "promptId": cur_prompt_id,
                "type": "user",
                "message": {
                    "role": "user",
                    "content": [{
                        "tool_use_id": toolu_id,
                        "type": "tool_result",
                        "content": [{"type": "text", "text": text or "ok"}],
                    }],
                },
                "uuid": res_uuid,
                "timestamp": iso,
                "toolUseResult": [{"type": "text", "text": text or "ok"}],
                "sourceToolAssistantUUID": cur_uuid,
                "userType": "external",
                "entrypoint": entrypoint,
                "cwd": cwd,
                "sessionId": sid,
                "version": version,
                "gitBranch": "",
            }
            lines.append(json.dumps(rec_res, ensure_ascii=False))
            prev_uuid = res_uuid
        elif kind == "compact":
            rec = {
                "parentUuid": None,
                "logicalParentUuid": prev_uuid,
                "isSidechain": False,
                "type": "system",
                "subtype": "compact_boundary",
                "content": text or "Conversation compacted",
                "isMeta": False,
                "level": "info",
                "timestamp": iso,
                "uuid": cur_uuid,
                "cwd": cwd,
                "sessionId": sid,
                "version": version,
            }
            lines.append(json.dumps(rec, ensure_ascii=False))
            prev_uuid = cur_uuid
        else:
            rec = {
                "parentUuid": prev_uuid,
                "isSidechain": False,
                "userType": "external",
                "cwd": cwd,
                "sessionId": sid,
                "version": version,
                "type": "system",
                "subtype": subtype or kind,
                "content": text,
                "uuid": cur_uuid,
                "timestamp": iso,
            }
            lines.append(json.dumps(rec, ensure_ascii=False))
            prev_uuid = cur_uuid

    lines.append(json.dumps({"type": "custom-title", "customTitle": final_title, "sessionId": sid}, ensure_ascii=False))
    lines.append(json.dumps({"type": "agent-name", "agentName": final_title, "sessionId": sid}, ensure_ascii=False))
    if prev_uuid:
        lines.append(json.dumps({
            "type": "last-prompt",
            "lastPrompt": (last_user_text or final_title)[:500],
            "leafUuid": prev_uuid,
            "sessionId": sid,
        }, ensure_ascii=False))

    atomic_write_text(target, "\n".join(lines) + "\n")

    custom_dir = proj_dir / sid
    os.makedirs(win_long_path(custom_dir), exist_ok=True)
    atomic_write_text(
        custom_dir / "custom-title.json",
        json.dumps({"customTitle": final_title}, ensure_ascii=False),
    )

    if history_jsonl.parent.exists():
        try:
            existing = (
                history_jsonl.read_text(encoding="utf-8", errors="replace").splitlines()
                if history_jsonl.is_file() else []
            )
            existing.append(json.dumps({
                "display": final_title,
                "pastedContents": {},
                "timestamp": int(now_sec * 1000),
                "project": cwd,
                "sessionId": sid,
            }, ensure_ascii=False))
            atomic_write_text(history_jsonl, "\n".join(existing) + "\n")
        except Exception:
            pass

    if dt_sessions_dir is not None:
        try:
            _write_claude_desktop_session(
                dt_sessions_dir,
                dt_tpl,
                cli_sid=sid,
                cwd=cwd,
                title=final_title,
                first_ms=int(first_sec * 1000),
                last_ms=int(last_sec * 1000),
                now_ms=int(now_sec * 1000),
                completed_turns=prompt_idx,
                last_asst_uuid=last_asst_uuid,
                last_asst_text=last_asst_text,
                model=model,
            )
        except Exception:
            pass

    CL._FILE_CACHE.clear()
    return {
        "provider": "claude",
        "cid": sid,
        "title": final_title,
        "entries": len(raw_entries),
    }


def _ensure_opencode_schema(con: sqlite3.Connection):
    con.execute(
        "CREATE TABLE IF NOT EXISTS project ("
        "id TEXT PRIMARY KEY, worktree TEXT NOT NULL, vcs TEXT, name TEXT, "
        "icon_url TEXT, icon_url_override TEXT, icon_color TEXT, "
        "time_created INTEGER NOT NULL, time_updated INTEGER NOT NULL, "
        "time_initialized INTEGER, time_active INTEGER NOT NULL DEFAULT 0, "
        "time_archived INTEGER, sandboxes TEXT NOT NULL DEFAULT '[]', commands TEXT)"
    )
    con.execute(
        "CREATE TABLE IF NOT EXISTS session_v2 ("
        "id TEXT PRIMARY KEY, project_id TEXT NOT NULL, workspace_id TEXT, parent_id TEXT, "
        "fork_session_id TEXT, fork_boundary TEXT, slug TEXT NOT NULL, directory TEXT NOT NULL, "
        "path TEXT, title TEXT, version TEXT NOT NULL, share_url TEXT, "
        "summary_additions INTEGER, summary_deletions INTEGER, summary_files INTEGER, summary_diffs TEXT, "
        "metadata TEXT, cost REAL NOT NULL DEFAULT 0, tokens_input INTEGER NOT NULL DEFAULT 0, "
        "tokens_output INTEGER NOT NULL DEFAULT 0, tokens_reasoning INTEGER NOT NULL DEFAULT 0, "
        "tokens_cache_read INTEGER NOT NULL DEFAULT 0, tokens_cache_write INTEGER NOT NULL DEFAULT 0, "
        "revert TEXT, permission TEXT, agent TEXT, model TEXT, "
        "time_created INTEGER NOT NULL, time_updated INTEGER NOT NULL, "
        "time_busy INTEGER, time_idle INTEGER, time_viewed INTEGER, idle_outcome TEXT, "
        "time_compacting INTEGER, time_archived INTEGER, time_suspended INTEGER, "
        "resume_attempts INTEGER NOT NULL DEFAULT 0)"
    )
    con.execute(
        "CREATE TABLE IF NOT EXISTS session_message ("
        "id TEXT PRIMARY KEY, session_id TEXT NOT NULL, type TEXT NOT NULL, seq INTEGER NOT NULL, "
        "time_created INTEGER NOT NULL, time_updated INTEGER NOT NULL, data TEXT NOT NULL)"
    )
    con.execute(
        "CREATE TABLE IF NOT EXISTS event_sequence ("
        "aggregate_id TEXT PRIMARY KEY, seq INTEGER NOT NULL, owner_id TEXT)"
    )


def _opencode_template_from_db(con: sqlite3.Connection) -> dict:
    tpl = {
        "version": "2.2.5",
        "agent": "build",
        "model_id": "nemotron-3.5-super-vl-free",
        "provider_id": "opencode",
        "variant": "default",
    }
    try:
        row = con.execute(
            "SELECT version, agent, model FROM session_v2 "
            "WHERE agent IS NOT NULL AND model IS NOT NULL AND slug NOT LIKE 'imported-%' "
            "ORDER BY time_updated DESC LIMIT 1"
        ).fetchone()
        if row:
            if row[0]:
                tpl["version"] = str(row[0])
            if row[1]:
                tpl["agent"] = str(row[1])
            if row[2]:
                m = json.loads(row[2])
                if isinstance(m, dict):
                    if m.get("id"):
                        tpl["model_id"] = str(m["id"])
                    if m.get("providerID"):
                        tpl["provider_id"] = str(m["providerID"])
    except Exception:
        pass
    try:
        mrow = con.execute(
            "SELECT data FROM session_message "
            "WHERE type = 'assistant' AND data LIKE '%\"variant\"%' "
            "ORDER BY time_created DESC LIMIT 1"
        ).fetchone()
        if mrow and mrow[0]:
            d = json.loads(mrow[0])
            m = d.get("model") if isinstance(d, dict) else None
            if isinstance(m, dict):
                if m.get("variant"):
                    tpl["variant"] = str(m["variant"])
                if not tpl["model_id"] and m.get("id"):
                    tpl["model_id"] = str(m["id"])
                if not tpl["provider_id"] and m.get("providerID"):
                    tpl["provider_id"] = str(m["providerID"])
    except Exception:
        pass
    return tpl


def import_to_opencode(
    doc: dict,
    *,
    db_path: Path | None = None,
    title: str | None = None,
    include_tools: bool = True,
) -> dict:
    raw_entries = _filter_entries(doc.get("entries") or [], include_tools=include_tools)
    if not raw_entries:
        raise ValueError("В файле нет сообщений для импорта.")

    target_db = Path(db_path) if db_path else OC.DB_PATH
    target_db.parent.mkdir(parents=True, exist_ok=True)
    meta = doc.get("meta") or {}
    final_title = _derive_title(raw_entries, title or meta.get("title"))
    meta_dir = _strip_win_long_prefix(str(meta.get("directory") or "").strip()).replace("\\", "/")
    if meta_dir and _is_ephemeral_workspace(meta_dir):
        meta_dir = ""

    now_sec = time.time()
    now_ms = int(now_sec * 1000)
    timestamps = _monotonic_timestamps(raw_entries, now_sec)
    first_ms = int(timestamps[0] * 1000)
    last_ms = max(first_ms, int(timestamps[-1] * 1000))
    upd_ms = max(last_ms, now_ms)

    sid = _opencode_id("ses", now_ms, 1, descending=True)
    slug = f"imported-{uuid.uuid4().hex[:8]}"

    con = OC.connect_rw(target_db)
    try:
        _ensure_opencode_schema(con)
        tpl = _opencode_template_from_db(con)
        agent = tpl["agent"]
        model_id = tpl["model_id"]
        provider_id = tpl["provider_id"]
        variant = tpl["variant"]
        oc_ver = tpl["version"]
        asst_model = {"id": model_id, "providerID": provider_id, "variant": variant}
        user_model = {"modelID": model_id, "providerID": provider_id}
        sess_model_json = json.dumps({"id": model_id, "providerID": provider_id}, ensure_ascii=False)
        zero_tokens = {"input": 0, "output": 0, "reasoning": 0, "cache": {"read": 0, "write": 0}}

        con.execute("BEGIN IMMEDIATE")

        prow = None
        if meta_dir:
            prow = con.execute(
                "SELECT id, worktree FROM project WHERE LOWER(worktree) = LOWER(?) LIMIT 1",
                (meta_dir,),
            ).fetchone()
        if not prow:
            prow = con.execute(
                "SELECT id, worktree FROM project WHERE time_archived IS NULL "
                "ORDER BY time_active DESC, time_updated DESC LIMIT 1"
            ).fetchone()
        if not prow:
            prow = con.execute(
                "SELECT id, worktree FROM project ORDER BY time_updated DESC LIMIT 1"
            ).fetchone()

        if prow:
            project_id = prow[0]
            directory = str(prow[1] or meta_dir or str(Path.home()).replace("\\", "/"))
        else:
            directory = meta_dir or str(Path.home()).replace("\\", "/")
            project_id = hashlib.sha1(directory.encode("utf-8")).hexdigest()
            con.execute(
                "INSERT INTO project (id, worktree, time_created, time_updated, time_active, sandboxes) "
                "VALUES (?, ?, ?, ?, ?, '[]')",
                (project_id, directory, now_ms, now_ms, now_ms),
            )

        con.execute(
            "INSERT INTO session_v2 ("
            "id, project_id, slug, directory, path, title, version, "
            "cost, tokens_input, tokens_output, tokens_reasoning, tokens_cache_read, tokens_cache_write, "
            "permission, agent, model, "
            "time_created, time_updated, time_busy, time_idle, time_viewed, idle_outcome, resume_attempts"
            ") VALUES (?, ?, ?, ?, '', ?, ?, 0, 0, 0, 0, 0, 0, '[]', ?, ?, ?, ?, ?, ?, ?, 'succeeded', 0)",
            (
                sid, project_id, slug, directory, final_title, oc_ver,
                agent, sess_model_json,
                first_ms, upd_ms, first_ms, last_ms, last_ms,
            ),
        )

        seq = 1
        msg_counter = 1
        prev_mid: str | None = None
        pending_parts: list[dict] = []
        pending_start_ms: int | None = None
        pending_end_ms: int | None = None

        def flush_assistant():
            nonlocal seq, msg_counter, prev_mid, pending_parts, pending_start_ms, pending_end_ms
            if not pending_parts:
                return
            st_ms = pending_start_ms if pending_start_ms is not None else first_ms
            en_ms = pending_end_ms if pending_end_ms is not None else st_ms
            has_tool = any(p.get("type") == "tool" for p in pending_parts)
            mid = _opencode_id("msg", st_ms, msg_counter, descending=False)
            msg_counter += 1
            data = {
                "time": {"created": st_ms, "streamed": en_ms, "completed": en_ms},
                "agent": agent,
                "model": asst_model,
                "content": pending_parts,
                "finish": "tool-calls" if has_tool else "stop",
                "rawFinish": "tool_calls" if has_tool else "stop",
                "cost": 0,
                "tokens": zero_tokens,
            }
            con.execute(
                "INSERT INTO session_message (id, session_id, type, seq, time_created, time_updated, data) "
                "VALUES (?, ?, 'assistant', ?, ?, ?, ?)",
                (mid, sid, seq, st_ms, en_ms, json.dumps(data, ensure_ascii=False)),
            )
            seq += 1
            prev_mid = mid
            pending_parts = []
            pending_start_ms = None
            pending_end_ms = None

        for it, sec in zip(raw_entries, timestamps):
            ms = int(sec * 1000)
            last_ms = max(last_ms, ms)
            kind = str(it.get("kind") or "assistant")
            subtype = str(it.get("subtype") or "")
            text = str(it.get("text") or "")

            if kind == "assistant":
                if any(p.get("type") == "tool" for p in pending_parts):
                    flush_assistant()
                if pending_start_ms is None:
                    pending_start_ms = ms
                pending_end_ms = ms
                pending_parts.append({"type": "text", "text": text})
                continue
            if kind == "tool" and subtype == "thinking":
                if text.strip():
                    if any(p.get("type") == "tool" for p in pending_parts):
                        flush_assistant()
                    if pending_start_ms is None:
                        pending_start_ms = ms
                    pending_end_ms = ms
                    pending_parts.append({
                        "type": "reasoning",
                        "text": text,
                        "time": {"created": ms, "completed": ms},
                    })
                continue
            if kind == "tool":
                if pending_start_ms is None:
                    pending_start_ms = ms
                pending_end_ms = ms
                tname = str(it.get("tool") or it.get("label") or "tool")
                is_err = str(it.get("status") or "").lower() == "error"
                if is_err:
                    tstate: dict = {
                        "status": "error",
                        "input": {},
                        "error": {"type": "aborted", "message": text or "Tool error"},
                    }
                else:
                    tstate = {
                        "status": "completed",
                        "input": {},
                        "content": [{"type": "text", "text": text}],
                        "metadata": {"status": "completed", "truncated": False},
                    }
                pending_parts.append({
                    "type": "tool",
                    "id": f"call-{uuid.uuid4()}",
                    "name": tname,
                    "executed": False,
                    "state": tstate,
                    "time": {"created": ms, "ran": ms, "completed": ms},
                })
                continue

            flush_assistant()
            mid = _opencode_id("msg", ms, msg_counter, descending=False)
            msg_counter += 1

            if kind == "user":
                mtype = "user"
                data = {
                    "metadata": {
                        "displayText": text,
                        "comments": [],
                        "attachments": [],
                        "agent": agent,
                        "model": user_model,
                    },
                    "time": {"created": ms},
                    "text": text,
                    "files": [],
                    "agents": [],
                }
            elif kind == "compact":
                mtype = "compaction"
                data = {
                    "metadata": {"keepID": prev_mid or mid},
                    "time": {"created": ms},
                    "status": "completed",
                    "reason": "auto",
                    "model": asst_model,
                    "summary": text,
                }
            elif kind == "error":
                mtype = "assistant"
                data = {
                    "time": {"created": ms, "completed": ms},
                    "agent": agent,
                    "model": asst_model,
                    "content": [],
                    "finish": "error",
                    "error": {"type": "aborted", "message": text or "Error"},
                    "cost": 0,
                    "tokens": zero_tokens,
                }
            else:
                mtype = "system"
                data = {
                    "metadata": {"notice": "instructions", "instructionSources": []},
                    "time": {"created": ms},
                    "text": text,
                    "description": str(it.get("label") or "Imported context"),
                }

            con.execute(
                "INSERT INTO session_message (id, session_id, type, seq, time_created, time_updated, data) "
                "VALUES (?, ?, ?, ?, ?, ?, ?)",
                (mid, sid, mtype, seq, ms, ms, json.dumps(data, ensure_ascii=False)),
            )
            seq += 1
            prev_mid = mid

        flush_assistant()

        idle_id = _opencode_id("msg", last_ms, msg_counter, descending=False)
        con.execute(
            "INSERT INTO session_message (id, session_id, type, seq, time_created, time_updated, data) "
            "VALUES (?, ?, 'idle', ?, ?, ?, ?)",
            (idle_id, sid, seq, last_ms, last_ms, json.dumps({"time": {"created": last_ms}, "outcome": "succeeded"})),
        )
        con.execute(
            "INSERT OR REPLACE INTO event_sequence (aggregate_id, seq, owner_id) VALUES (?, ?, NULL)",
            (sid, seq + 1),
        )
        con.execute(
            "UPDATE session_v2 SET time_updated = ?, time_idle = ?, time_viewed = ? WHERE id = ?",
            (max(last_ms, now_ms), last_ms, last_ms, sid),
        )
        con.execute("COMMIT")
    except Exception:
        if con.in_transaction:
            con.execute("ROLLBACK")
        raise
    finally:
        con.close()

    return {
        "provider": "opencode",
        "cid": sid,
        "title": final_title,
        "entries": len(raw_entries),
    }


def import_doc_to_provider(
    doc: dict,
    dst_prov,
    *,
    title: str | None = None,
    include_tools: bool = True,
) -> dict:
    pid = dst_prov.id
    if pid == "antigravity":
        res = import_to_antigravity(
            doc, dst_prov.store, app_data_dir="antigravity", title=title, include_tools=include_tools
        )
        dst_prov._invalidate()
        return res
    if pid == "antigravity_cli":
        res = import_to_antigravity(
            doc, dst_prov.store, app_data_dir="antigravity-cli", title=title, include_tools=include_tools
        )
        dst_prov._invalidate()
        return res
    if pid == "codex":
        return import_to_codex(doc, title=title, include_tools=include_tools)
    if pid == "claude":
        return import_to_claude(doc, title=title, include_tools=include_tools)
    if pid == "opencode":
        return import_to_opencode(doc, db_path=dst_prov.db_path, title=title, include_tools=include_tools)
    raise ValueError(f"Неизвестный целевой провайдер: {pid}")


def convert_chat(
    src_prov,
    cid: str,
    dst_prov,
    *,
    title: str | None = None,
    include_tools: bool = False,
    set_phase=None,
) -> dict:
    if set_phase:
        set_phase("read")
    doc = export_provider_chat(src_prov, cid, include_tools=include_tools)
    if not doc.get("entries"):
        raise ValueError("В выбранном диалоге нет сообщений для конвертации.")
    if set_phase:
        set_phase("write")
    res = import_doc_to_provider(doc, dst_prov, title=title, include_tools=include_tools)
    res["source_provider"] = src_prov.id
    res["source_id"] = cid
    if set_phase:
        set_phase("done")
    return res


def export_to_k1tty_bytes(
    src_prov,
    cid: str,
    *,
    preset: str = "balanced",
    include_tools: bool = True,
) -> tuple[bytes, dict, str]:
    doc = export_provider_chat(src_prov, cid, include_tools=include_tools)
    if not doc.get("entries"):
        raise ValueError("В выбранном диалоге нет сообщений для экспорта.")
    packed, summary = KP.pack_chat(doc, preset=preset, include_tools=include_tools)
    title = doc["meta"].get("title") or cid[:12]
    safe = re.sub(r"[^A-Za-zА-Яа-яЁёІіЇїЄєҐґ0-9._ -]+", "_", title).strip(" ._")[:60] or cid[:12]
    filename = f"{safe}_{src_prov.short.lower()}.k1tty"
    summary["title"] = title
    summary["source_provider"] = src_prov.id
    summary["source_id"] = cid
    return packed, summary, filename


def import_k1tty_bytes(
    raw_bytes: bytes,
    dst_prov,
    *,
    title: str | None = None,
    include_tools: bool = True,
) -> dict:
    doc = KP.unpack_chat(raw_bytes)
    res = import_doc_to_provider(doc, dst_prov, title=title, include_tools=include_tools)
    res["source_provider"] = (doc.get("meta") or {}).get("source_provider") or ""
    res["pack_info"] = doc.get("pack_info") or {}
    return res
