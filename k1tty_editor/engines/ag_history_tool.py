from __future__ import annotations

import ctypes
from dataclasses import dataclass
import json
import os
import re
import shutil
import sqlite3
import struct
import sys
import time
from datetime import datetime, timezone
from pathlib import Path

if os.name == "nt":
    try:
        ctypes.windll.kernel32.SetConsoleOutputCP(65001)
        ctypes.windll.kernel32.SetConsoleCP(65001)
    except Exception:
        pass
for stream in (sys.stdout, sys.stderr):
    if hasattr(stream, "reconfigure"):
        try:
            stream.reconfigure(encoding="utf-8", errors="replace")
        except Exception:
            pass

APPDATA = Path(os.environ.get("ANTIGRAVITY_APPDATA", "") or
                (Path(os.environ.get("USERPROFILE", "")) / ".gemini" / "antigravity"))
CONVERSATIONS_DIR = APPDATA / "conversations"
BRAIN_DIR = APPDATA / "brain"
SUMMARIES_DB = APPDATA / "conversation_summaries.db"
ANNOTATIONS_DIR = APPDATA / "annotations"
DAEMON_DIR = APPDATA / "daemon"
BACKUP_ROOT = Path(os.environ.get("ANTIGRAVITY_BACKUP_ROOT", "") or
                   (Path(__file__).resolve().parent / "backups"))


@dataclass(frozen=True)
class AgStore:
    appdata: Path
    conversations_dir: Path
    brain_dir_root: Path
    summaries_db: Path
    annotations_dir: Path
    daemon_dir: Path
    presence_dir: Path
    backup_root: Path


def make_store(appdata: Path, backup_root: Path) -> AgStore:
    ap = Path(appdata)
    return AgStore(
        appdata=ap,
        conversations_dir=ap / "conversations",
        brain_dir_root=ap / "brain",
        summaries_db=ap / "conversation_summaries.db",
        annotations_dir=ap / "annotations",
        daemon_dir=ap / "daemon",
        presence_dir=ap / "presence",
        backup_root=Path(backup_root),
    )


def default_store() -> AgStore:
    return AgStore(
        appdata=APPDATA,
        conversations_dir=CONVERSATIONS_DIR,
        brain_dir_root=BRAIN_DIR,
        summaries_db=SUMMARIES_DB,
        annotations_dir=ANNOTATIONS_DIR,
        daemon_dir=DAEMON_DIR,
        presence_dir=APPDATA / "presence",
        backup_root=BACKUP_ROOT,
    )


def _resolve_store(store: AgStore | None) -> AgStore:
    return store if store is not None else default_store()

STEP_TYPES = {
    14: "USER_INPUT",
    15: "PLANNER_RESPONSE",
    17: "ERROR",
    23: "CHECKPOINT",
    101: "SYSTEM_MESSAGE",
    132: "GENERIC (tool)",
}


def read_varint(buf: bytes, i: int):
    result = 0
    shift = 0
    while True:
        if i >= len(buf):
            raise IndexError("varint out of range")
        b = buf[i]
        i += 1
        result |= (b & 0x7F) << shift
        if not (b & 0x80):
            return result, i
        shift += 7


def parse_fields(buf: bytes):
    i = 0
    out = []
    try:
        while i < len(buf):
            tag, i = read_varint(buf, i)
            field, wt = tag >> 3, tag & 7
            if wt == 0:
                val, i = read_varint(buf, i)
            elif wt == 1:
                if i + 8 > len(buf):
                    break
                val = buf[i:i + 8]
                i += 8
            elif wt == 2:
                ln, i = read_varint(buf, i)
                if i + ln > len(buf):
                    break
                val = buf[i:i + ln]
                i += ln
            elif wt == 5:
                if i + 4 > len(buf):
                    break
                val = buf[i:i + 4]
                i += 4
            else:
                break
            out.append((field, wt, val))
    except (IndexError, ValueError):
        pass
    return out


def encode_varint(v: int) -> bytes:
    out = b""
    while True:
        b = v & 0x7F
        v >>= 7
        if v:
            out += bytes([b | 0x80])
        else:
            out += bytes([b])
            return out


def encode_field(field: int, wt: int, payload: bytes) -> bytes:
    return encode_varint((field << 3) | wt) + (
        payload if wt in (0, 1, 5) else encode_varint(len(payload)) + payload
    )


def extract_gen_attrs(blob: bytes) -> dict:
    attrs = {}
    for f, wt, v in parse_fields(blob):
        if f == 1 and wt == 2:
            for ff, fw, vv in parse_fields(v):
                if ff == 20 and fw == 2:
                    nv = parse_fields(vv)
                    if len(nv) >= 2 and nv[0][1] == 2 and nv[1][1] == 2:
                        try:
                            key = nv[0][2].decode("utf-8", "replace")
                            val = nv[1][2].decode("utf-8", "replace")
                            attrs[key] = val
                        except Exception:
                            pass
    return attrs


def patch_varint_fields(buf: bytes, updates: dict) -> bytes:
    out = bytearray()
    i = 0
    try:
        while i < len(buf):
            start = i
            tag, i = read_varint(buf, i)
            field, wt = tag >> 3, tag & 7
            if wt == 0:
                _, i = read_varint(buf, i)
                if field in updates:
                    out += encode_field(field, 0, encode_varint(updates[field]))
                else:
                    out += buf[start:i]
            elif wt == 2:
                ln, i = read_varint(buf, i)
                if i + ln > len(buf):
                    out += buf[start:]
                    break
                end = i + ln
                out += buf[start:end]
                i = end
            elif wt == 1:
                if i + 8 > len(buf):
                    out += buf[start:]
                    break
                out += buf[start:i + 8]
                i += 8
            elif wt == 5:
                if i + 4 > len(buf):
                    out += buf[start:]
                    break
                out += buf[start:i + 4]
                i += 4
            else:
                out += buf[start:]
                break
    except (IndexError, ValueError):
        out += buf[i:]
    return bytes(out)


def find_field(fields, field_no):
    for f, wt, v in fields:
        if f == field_no:
            return wt, v
    return None, None


def top_level_pairs(buf: bytes):
    return parse_fields(buf)


def payload_summary(step_type: int, payload: bytes) -> dict:
    info = {"text": "", "tool": "", "args": ""}
    try:
        fg = top_level_pairs(payload)
        if step_type == 14:
            _, f19 = find_field(fg, 19)
            if f19:
                _, f3 = find_field(parse_fields(f19), 3)
                if f3:
                    texts = []
                    for ff, fw, fv in parse_fields(f3):
                        if fw == 2:
                            nv = parse_fields(fv)
                            _, s = find_field(nv, 1)
                            if s and isinstance(s, bytes) and s and s[0:1] != b"\n":
                                texts.append(s.decode("utf-8", "replace"))
                    info["text"] = "\n".join(texts)
        elif step_type == 15:
            _, f20 = find_field(fg, 20)
            if f20:
                _, f1 = find_field(parse_fields(f20), 1)
                if f1 and isinstance(f1, bytes):
                    info["text"] = f1.decode("utf-8", "replace")
        elif step_type == 132:
            _, f5 = find_field(fg, 5)
            if f5:
                _, f4 = find_field(parse_fields(f5), 4)
                if f4:
                    f4f = parse_fields(f4)
                    _, name = find_field(f4f, 2)
                    _, args = find_field(f4f, 3)
                    info["tool"] = name.decode("utf-8", "replace") if name else ""
                    info["args"] = args.decode("utf-8", "replace") if args else ""
    except Exception as e:
        info["parse_error"] = repr(e)
    return info


def connect_ro(db: Path):
    con = None
    try:
        con = sqlite3.connect(f"file:{db}?mode=ro", uri=True, timeout=30)
        con.execute("select 1 from sqlite_master limit 1").fetchone()
    except sqlite3.Error:
        if con is not None:
            con.close()
        con = sqlite3.connect(f"file:{db}?mode=ro&immutable=1", uri=True)
    con.text_factory = bytes
    return con


def connect_rw(db: Path):
    con = sqlite3.connect(str(db), timeout=30)
    con.text_factory = bytes
    con.execute("pragma journal_mode=delete")
    return con


def read_jsonl(path: Path):
    lines = []
    with open(path, encoding="utf-8", errors="replace") as f:
        for line in f:
            s = line.rstrip("\n")
            if s.strip():
                lines.append(s)
    return lines


def _process_image_name(pid: int):
    try:
        import subprocess
        out = subprocess.run(
            ["tasklist", "/FI", f"PID eq {pid}", "/FO", "CSV", "/NH"],
            capture_output=True, text=True, errors="replace", timeout=15,
        ).stdout
        for line in out.splitlines():
            line = line.strip()
            if not line.startswith('"'):
                continue
            parts = [p.strip('"') for p in line.split('","')]
            if len(parts) >= 2 and parts[1].strip() == str(pid):
                return parts[0]
    except Exception:
        return None
    return None


def antigravity_running() -> list:
    running = []
    if DAEMON_DIR.exists():
        for hub in DAEMON_DIR.glob("hub_*.json"):
            try:
                info = json.loads(hub.read_text(encoding="utf-8"))
                pid = int(info.get("pid", 0))
                if pid:
                    image = _process_image_name(pid)
                    if image and ("antigravity" in image.lower() or "language_server" in image.lower()):
                        running.append(f"{hub.name}: {image} (PID {pid})")
            except Exception:
                continue
    try:
        import subprocess
        out = subprocess.run(["tasklist", "/FO", "CSV", "/NH"], capture_output=True,
                             text=True, errors="replace", timeout=20).stdout
        for line in out.splitlines():
            low = line.lower()
            if "antigravity" in low or "language_server" in low:
                parts = [p.strip('"') for p in line.split('","')]
                if parts:
                    running.append(f"процесс: {parts[0]} (PID {parts[1] if len(parts) > 1 else '?'})")
    except Exception:
        pass
    return running


def list_conversations(store: AgStore | None = None):
    s = _resolve_store(store)
    res = []
    if not s.conversations_dir.exists():
        return res
    titles = {}
    steps_count = {}
    try:
        con = connect_ro(s.summaries_db)
        for cid, title, sc in con.execute(
            "select conversation_id, title, step_count from conversation_summaries"
        ):
            titles[cid.decode()] = (title or b"").decode("utf-8", "replace")
            steps_count[cid.decode()] = int(sc)
        con.close()
    except Exception:
        pass
    for db in sorted(s.conversations_dir.glob("*.db"), key=lambda p: p.stat().st_mtime, reverse=True):
        cid = db.stem
        n = None
        try:
            con = connect_ro(db)
            n = con.execute("select count(*), max(idx) from steps").fetchone()
            con.close()
        except Exception:
            pass
        res.append({
            "id": cid,
            "db": db,
            "size": db.stat().st_size,
            "mtime": datetime.fromtimestamp(db.stat().st_mtime),
            "title": titles.get(cid) or "(без названия)",
            "steps_count_summaries": steps_count.get(cid),
            "steps_in_db": (n[0], n[1]) if n else None,
        })
    return res


def brain_dir(cid: str, store: AgStore | None = None) -> Path:
    s = _resolve_store(store)
    return s.brain_dir_root / cid / ".system_generated"


def read_history(cid: str, store: AgStore | None = None) -> list:
    b = brain_dir(cid, store=store)
    p_full = b / "logs" / "transcript_full.jsonl"
    p_compact = b / "logs" / "transcript.jsonl"
    src = p_full if p_full.exists() else p_compact
    if not src.exists():
        return []
    entries = []
    for line in read_jsonl(src):
        try:
            entries.append(json.loads(line))
        except Exception:
            continue
    return entries


def read_transcript_lines(cid: str, full: bool = True, store: AgStore | None = None) -> list[str]:
    b = brain_dir(cid, store=store)
    p = b / "logs" / ("transcript_full.jsonl" if full else "transcript.jsonl")
    if not p.exists():
        return []
    return read_jsonl(p)


def write_transcript_lines(cid: str, lines, full: bool = True, store: AgStore | None = None):
    b = brain_dir(cid, store=store)
    p = b / "logs" / ("transcript_full.jsonl" if full else "transcript.jsonl")
    new_bytes = ("\n".join(lines) + ("\n" if lines else "")).encode("utf-8")
    tmp = p.with_suffix(".jsonl.tmp")
    tmp.write_bytes(new_bytes)
    tmp.replace(p)
    regen_chunks(b / "logs" / "chunks" / p.stem, new_bytes)


CHUNK_CAP = 102400


def regen_chunks(cdir: Path, new_bytes: bytes):
    if cdir.exists():
        for cf in sorted(cdir.glob("*.jsonl")):
            cf.unlink()
    total = (len(new_bytes) + CHUNK_CAP - 1) // CHUNK_CAP
    cdir.mkdir(parents=True, exist_ok=True)
    for ci in range(total):
        part = new_bytes[ci * CHUNK_CAP:(ci + 1) * CHUNK_CAP]
        (cdir / f"{ci:08d}.jsonl").write_bytes(part)


def print_history(entries, n=None, from_end=True, max_chars=250):
    turns = []
    for e in entries:
        if e.get("type") == "USER_INPUT":
            turns.append({"u": e["step_index"], "items": [e]})
        elif turns:
            turns[-1]["items"].append(e)
        else:
            turns.append({"u": None, "items": [e]})
    sel = turns[-n:] if n and from_end else turns
    for tno, t in enumerate(sel, 1):
        first = t["items"][0]
        print(f"\n--- Ход {tno} (шаги {first['step_index']}..{t['items'][-1]['step_index']}) ---")
        for e in t["items"]:
            content = (e.get("content") or "").replace("\n", " ")
            if len(content) > max_chars:
                content = content[:max_chars] + f" ...[+{len(e.get('content') or '')-max_chars}]"
            tc = e.get("tool_calls") or []
            tc_str = "; ".join(f"{c.get('name')}()" for c in tc) if tc else ""
            print(f"  [{e['step_index']:>4}] {e['type']:<16} {content}")
            if tc_str:
                print(f"          tool_calls: {tc_str}")


def steps_ids_le(entries, user_turn_index: int):
    turns = [e for e in entries if e.get("type") == "USER_INPUT"]
    if 1 <= user_turn_index <= len(turns):
        return turns[user_turn_index - 1]["step_index"]
    raise ValueError("нет такого хода")


def verify_not_running():
    r = antigravity_running()
    return (not r), r


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


def full_backup(cid: str, tag: str = "", store: AgStore | None = None) -> Path:
    s = _resolve_store(store)
    ts = datetime.now().strftime("%Y%m%d-%H%M%S-%f")
    bdir = s.backup_root / cid / (ts + ("-" + tag if tag else ""))
    bdir.mkdir(parents=True, exist_ok=True)

    db_path = s.conversations_dir / f"{cid}.db"
    shutil.copy2(db_path, bdir / db_path.name)
    for extra in ("-wal", "-shm"):
        p = Path(str(db_path) + extra)
        if p.exists():
            shutil.copy2(p, bdir / Path(p.name))

    b = brain_dir(cid, store=s)
    if b.exists():
        dst = bdir / "brain" / cid / ".system_generated"
        shutil.copytree(b, dst, dirs_exist_ok=True, ignore=shutil.ignore_patterns("*.tmp"))

    if s.summaries_db.exists():
        shutil.copy2(s.summaries_db, bdir / s.summaries_db.name)
        p = Path(str(s.summaries_db) + "-wal")
        if p.exists():
            shutil.copy2(p, bdir / (s.summaries_db.name + "-wal"))

    an = s.annotations_dir / f"{cid}.pbtxt"
    if an.exists():
        (bdir / "annotations").mkdir(exist_ok=True)
        shutil.copy2(an, bdir / "annotations" / an.name)

    (bdir / "backup.meta.json").write_text(json.dumps({
        "conversation_id": cid,
        "created": datetime.now().isoformat(),
        "original_db": str(db_path),
        "appdata": str(s.appdata),
    }, ensure_ascii=False, indent=1), encoding="utf-8")
    return bdir


def create_backup(cid: str, store: AgStore | None = None) -> Path:
    return full_backup(cid, store=store)


def _restore_db_with_wal(src_db: Path, db_path: Path):
    db_path.parent.mkdir(parents=True, exist_ok=True)
    for extra in ("-wal", "-shm"):
        p = Path(str(db_path) + extra)
        if p.exists():
            p.unlink()
    shutil.copy2(src_db, db_path)
    for extra in ("-wal", "-shm"):
        src = Path(str(src_db) + extra)
        if src.exists():
            shutil.copy2(src, Path(str(db_path) + extra))


def restore_backup(backup_dir: Path, store: AgStore | None = None):
    s = _resolve_store(store)
    meta = json.loads((backup_dir / "backup.meta.json").read_text(encoding="utf-8"))
    cid = meta["conversation_id"]
    db_path = s.conversations_dir / f"{cid}.db"
    src_db = backup_dir / db_path.name
    if not src_db.exists():
        raise FileNotFoundError(src_db)
    _restore_db_with_wal(src_db, db_path)

    bd = brain_dir(cid, store=s)
    if (backup_dir / "brain" / cid / ".system_generated").exists():
        if bd.exists():
            shutil.rmtree(bd)
        bd.parent.mkdir(parents=True, exist_ok=True)
        shutil.copytree(backup_dir / "brain" / cid / ".system_generated", bd)

    src_sum = backup_dir / s.summaries_db.name
    if src_sum.exists():
        _restore_db_with_wal(src_sum, s.summaries_db)

    an = s.annotations_dir / f"{cid}.pbtxt"
    src_an = backup_dir / "annotations" / f"{cid}.pbtxt"
    if src_an.exists() and not an.exists():
        s.annotations_dir.mkdir(parents=True, exist_ok=True)
        shutil.copy2(src_an, an)
    print("Восстановление завершено.")


STEP_REF_RE = re.compile(r"steps/(\d+)/output\.txt")
TASK_REF_RE = re.compile(r"task-(\d+)\.log")


def collect_protected_step_refs(entries_kept, kept_msgs):
    protected = set()
    for e in entries_kept:
        s = e.get("content") or ""
        if "steps/" in s:
            protected.update(int(m.group(1)) for m in STEP_REF_RE.finditer(s))
        if "task-" in s:
            protected.update(int(m.group(1)) for m in TASK_REF_RE.finditer(s))
        tc = e.get("tool_calls")
        if tc:
            s2 = json.dumps(tc)
            if "steps/" in s2:
                protected.update(int(m.group(1)) for m in STEP_REF_RE.finditer(s2))
            if "task-" in s2:
                protected.update(int(m.group(1)) for m in TASK_REF_RE.finditer(s2))
    for m in kept_msgs:
        s = m.get("content") or ""
        if "steps/" in s:
            protected.update(int(x.group(1)) for x in STEP_REF_RE.finditer(s))
        if "task-" in s:
            protected.update(int(x.group(1)) for x in TASK_REF_RE.finditer(s))
    return protected


def rollback(cid: str, keep_step: int, preview_only: bool = False, mode: str = "tail", store: AgStore | None = None):
    s = _resolve_store(store)
    db_path = s.conversations_dir / f"{cid}.db"
    b = brain_dir(cid, store=s)
    report = {"cid": cid, "keep_step": keep_step, "mode": mode, "preview_only": preview_only}

    if mode == "tail":
        is_removed = lambda i: i > keep_step
        _sql_cmp = "> ?"
    else:
        is_removed = lambda i: i < keep_step
        _sql_cmp = "< ?"

    entries = read_history(cid, store=s)
    before = len(entries)
    removed_entries = [e for e in entries if is_removed(e["step_index"])]
    kept_entries = [e for e in entries if not is_removed(e["step_index"])]
    report["transcript_lines_before"] = before
    report["transcript_lines_removed"] = len(removed_entries)
    report["transcript_lines_kept"] = len(kept_entries)

    con = connect_ro(db_path)
    db_total = con.execute("select count(*), max(idx), min(idx) from steps").fetchone()
    con.close()
    report["db_rows_before"] = db_total[0]

    if preview_only:
        if mode == "tail":
            db_rows_del = db_total[0] - (keep_step + 1)
        else:
            db_rows_del = keep_step - (db_total[2] or 0)
        report["will_delete"] = {
            "db_rows": db_rows_del,
            "transcript_lines": len(removed_entries),
            "messages_files": count_message_files(cid, keep_step, mode, store=s),
            "steps_output_dirs": count_step_output_dirs(cid, keep_step, mode, store=s),
            "task_logs": count_task_logs(cid, keep_step, mode, store=s),
        }
        return report

    con = connect_rw(db_path)
    try:
        con.execute("begin")
        con.execute(f"delete from steps where idx {_sql_cmp}", (keep_step,))
        gen_del = 0
        for idx, data in con.execute("select idx, data from gen_metadata").fetchall():
            try:
                attrs = extract_gen_attrs(data)
                lsi = int(attrs.get("last_step_index", "-1"))
                if (lsi > keep_step) if mode == "tail" else (lsi < keep_step):
                    con.execute("delete from gen_metadata where idx=?", (idx,))
                    gen_del += 1
            except Exception:
                continue
        con.execute("commit")
        chk = [r[0].decode() for r in con.execute("pragma integrity_check")]
        report["gen_metadata_removed"] = gen_del
        report["integrity_check"] = chk
        assert chk == ["ok"], f"integrity_check fail: {chk}"
        cnt, maxi = con.execute("select count(*), max(idx) from steps").fetchone()
        report["db_rows_after"] = cnt
        report["db_max_idx_after"] = maxi
    except Exception:
        con.execute("rollback") if con.in_transaction else None
        raise
    finally:
        con.close()

    for name in ("transcript.jsonl", "transcript_full.jsonl"):
        p = b / "logs" / name
        if not p.exists():
            continue
        kept_lines = [l for l in read_jsonl(p) if not is_removed(_safe_step_index(l))]
        write_transcript_lines(cid, kept_lines, full=p.stem == "transcript_full", store=s)

    mdir = b / "messages"
    removed_msgs, kept_msgs = [], []
    if mdir.exists():
        for mf in sorted(mdir.glob("*.json")):
            if mf.name == "read.json":
                continue
            try:
                msg = json.loads(mf.read_text(encoding="utf-8"))
            except Exception:
                continue
            si = _msg_step_index(msg)
            if si is not None and is_removed(si):
                mf.unlink()
                removed_msgs.append(msg)
            else:
                kept_msgs.append(msg)
    if (mdir / "read.json").exists():
        removed_ids = {m["id"] for m in removed_msgs if "id" in m}
        try:
            rd = json.loads((mdir / "read.json").read_text(encoding="utf-8"))
            for rid in removed_ids:
                rd.pop(rid, None)
            (mdir / "read.json").write_text(json.dumps(rd), encoding="utf-8")
        except Exception:
            pass
    report["messages_removed"] = len(removed_msgs)

    protected = collect_protected_step_refs(kept_entries, kept_msgs)
    sdir = b / "steps"
    removed_out = 0
    if sdir.exists():
        for sd in sdir.iterdir():
            if not sd.is_dir() or not sd.name.isdigit():
                continue
            if is_removed(int(sd.name)) and int(sd.name) not in protected:
                shutil.rmtree(sd, ignore_errors=True)
                removed_out += 1
    tdir = b / "tasks"
    removed_logs = 0
    if tdir.exists():
        for tf in tdir.glob("task-*.log"):
            m = re.match(r"task-(\d+)\.log", tf.name)
            if m and is_removed(int(m.group(1))) and int(m.group(1)) not in protected:
                tf.unlink()
                removed_logs += 1
    report["steps_outputs_removed"] = removed_out
    report["task_logs_removed"] = removed_logs

    try:
        update_summaries(cid, keep_step, kept_entries, entries, mode, db_count=cnt, store=s)
        report["summaries_updated"] = True
    except Exception as e:
        report["summaries_updated"] = False
        report["summaries_error"] = repr(e)

    return report


def _safe_step_index(line: str):
    try:
        return json.loads(line).get("step_index", -10**9)
    except Exception:
        return -10**9


def _msg_step_index(msg: dict):
    try:
        return int(msg["sourceMetadata"]["tool"]["stepIndex"])
    except Exception:
        return None


def _counter_removed(mode, keep_step):
    if mode == "tail":
        return lambda i: i > keep_step
    return lambda i: i < keep_step


def count_message_files(cid, keep_step, mode="tail", store: AgStore | None = None):
    n = 0
    is_removed = _counter_removed(mode, keep_step)
    mdir = brain_dir(cid, store=store) / "messages"
    if mdir.exists():
        for mf in mdir.glob("*.json"):
            if mf.name == "read.json":
                continue
            try:
                msg = json.loads(mf.read_text(encoding="utf-8"))
                si = _msg_step_index(msg)
                if si is not None and is_removed(si):
                    n += 1
            except Exception:
                pass
    return n


def count_step_output_dirs(cid, keep_step, mode="tail", store: AgStore | None = None):
    is_removed = _counter_removed(mode, keep_step)
    sdir = brain_dir(cid, store=store) / "steps"
    if sdir.exists():
        return sum(1 for d in sdir.iterdir() if d.is_dir() and d.name.isdigit() and is_removed(int(d.name)))
    return 0


def count_task_logs(cid, keep_step, mode="tail", store: AgStore | None = None):
    is_removed = _counter_removed(mode, keep_step)
    tdir = brain_dir(cid, store=store) / "tasks"
    if tdir.exists():
        return sum(1 for f in tdir.glob("task-*.log")
                if re.match(r"task-(\d+)\.log", f.name) and is_removed(int(re.match(r"task-(\d+)", f.name).group(1))))
    return 0


def _as_int(v):
    try:
        return int(v)
    except Exception:
        try:
            return int(float(v))
        except Exception:
            return -10**9


def update_summaries(cid: str, keep_step: int, kept_entries, all_entries, mode: str = "tail",
                     db_count: int | None = None, store: AgStore | None = None):
    s = _resolve_store(store)
    if not s.summaries_db.exists():
        return
    con = connect_rw(s.summaries_db)
    try:
        con.execute("begin")

        last_user_idx, last_user_time, last_step = 0, None, -1
        first_step = None
        for e in kept_entries:
            si = e["step_index"]
            last_step = max(last_step, si)
            first_step = si if first_step is None else min(first_step, si)
            if e.get("type") == "USER_INPUT":
                last_user_idx = max(last_user_idx, si)
                last_user_time = e.get("created_at")

        if db_count is not None:
            new_step_count = db_count
        elif mode == "tail":
            new_step_count = keep_step + 1 if keep_step >= 0 else 0
        else:
            new_step_count = (last_step - keep_step + 1) if kept_entries else 0
        rows = con.execute(
            "select rowid, raw_summary from conversation_summaries where conversation_id=?", (cid,)
        ).fetchall()
        for rowid, raw in rows:
            if mode == "tail":
                con.execute(
                    "update conversation_summaries set step_count=?, last_user_input_step_index=? where rowid=?",
                    (new_step_count, last_user_idx, rowid),
                )
                new_raw = patch_varint_fields(raw, {2: new_step_count, 16: last_user_idx})
            else:
                con.execute(
                    "update conversation_summaries set step_count=? where rowid=?",
                    (new_step_count, rowid),
                )
                new_raw = patch_varint_fields(raw, {2: new_step_count})
            con.execute("update conversation_summaries set raw_summary=? where rowid=?", (new_raw, rowid))

        sts = con.execute(
            "select rowid, chunk_first_step, chunk_last_step, user_step_byte_offsets, "
            "agent_step_byte_offsets, user_body, agent_body from conversations_fts "
            "where conversation_id=?", (cid,)
        ).fetchall()
        n_del, n_upd = 0, 0
        for rowid, first, last, uoff, aoff, ubody, abody in sts:
            first, last = _as_int(first), _as_int(last)
            if mode == "tail":
                if first > keep_step:
                    con.execute("delete from conversations_fts where rowid=?", (rowid,))
                    n_del += 1
                    continue
                if last <= keep_step:
                    continue
            else:
                if last < keep_step:
                    con.execute("delete from conversations_fts where rowid=?", (rowid,))
                    n_del += 1
                    continue
                if first >= keep_step:
                    continue

            def cut(offsets_json, body):
                try:
                    offs = json.loads(offsets_json)
                except Exception:
                    return offsets_json, body, keep_step if mode == "tail" else first
                if mode == "tail":
                    kept = [o for o in offs if _as_int(o[0]) <= keep_step]
                    if len(kept) == len(offs):
                        return offsets_json, body, keep_step
                    new_end = max((_as_int(o[1]) for o in kept), default=0)
                    if isinstance(body, str):
                        body_cut = body[:new_end]
                    else:
                        body_cut = body[:new_end] if new_end <= len(body) else body
                    kept2 = [o for o in kept if _as_int(o[1]) < new_end]
                    return json.dumps(kept2), body_cut, keep_step
                else:
                    kept = [o for o in offs if _as_int(o[0]) >= keep_step]
                    if len(kept) == len(offs):
                        return offsets_json, body, max(first, keep_step)
                    cut_start = min((_as_int(o[1]) for o in kept), default=0)
                    if isinstance(body, str):
                        body_cut = body[cut_start:]
                    else:
                        body_cut = body[cut_start:]
                    kept2 = [[o[0], _as_int(o[1]) - cut_start] for o in kept if _as_int(o[1]) >= cut_start]
                    return json.dumps(kept2), body_cut, max(first, keep_step)

            uoff2, ubody2, u_first = cut(uoff, ubody)
            aoff2, abody2, a_first = cut(aoff, abody)
            if mode == "tail":
                con.execute(
                    "update conversations_fts set chunk_last_step=?, user_step_byte_offsets=?, "
                    "agent_step_byte_offsets=?, user_body=?, agent_body=? where rowid=?",
                    (keep_step, uoff2, aoff2, ubody2, abody2, rowid),
                )
            else:
                con.execute(
                    "update conversations_fts set chunk_first_step=?, user_step_byte_offsets=?, "
                    "agent_step_byte_offsets=?, user_body=?, agent_body=? where rowid=?",
                    (min(u_first, a_first), uoff2, aoff2, ubody2, abody2, rowid),
                )
            n_upd += 1
        con.execute("commit")
    except Exception:
        if con.in_transaction:
            con.execute("rollback")
        raise
    finally:
        con.close()


def _rebuild_with_nested(buf: bytes, chain: list, new_text: bytes, all_occ: bool = True,
                         required: bool = True) -> bytes:
    repl_flag = {"done": False}

    def rec(msg: bytes, fields_left) -> bytes:
        out = bytearray()
        is_final = len(fields_left) == 1
        for f, wt, v in parse_fields(msg):
            if f == fields_left[0] and wt == 2:
                if is_final:
                    if all_occ or not repl_flag["done"]:
                        out += encode_field(f, 2, new_text)
                        repl_flag["done"] = True
                    else:
                        out += encode_field(f, 2, v if isinstance(v, bytes) else bytes(v))
                else:
                    inner = v if isinstance(v, bytes) else bytes(v)
                    out += encode_field(f, 2, rec(inner, fields_left[1:]))
            else:
                out += encode_field(f, wt, v if not isinstance(v, int) else encode_varint(v))
        return bytes(out)

    result = rec(buf, chain)
    if required and not repl_flag["done"]:
        raise ValueError("в шаге не найдено текстовое поле для замены")
    if [(f, wt) for f, wt, _ in parse_fields(result)] != [(f, wt) for f, wt, _ in parse_fields(buf)]:
        raise ValueError("payload structure changed after edit, rollback")
    return result


def extract_user_request(content: str) -> str:
    m = re.search(r"<USER_REQUEST>(.*?)</USER_REQUEST>", content or "", re.S)
    return m.group(1).strip() if m else (content or "").strip()


def replace_user_request(content: str, new_text: str) -> str:
    if "<USER_REQUEST>" in (content or ""):
        return re.sub(r"<USER_REQUEST>.*?</USER_REQUEST>",
                      lambda _m: "<USER_REQUEST>\n" + new_text + "\n</USER_REQUEST>",
                      content, count=1, flags=re.S)
    return new_text


def _patch_user_payload(payload: bytes, new_text: str, old_request: str):
    newb = new_text.encode("utf-8")
    changed = False

    def patch_part(part: bytes) -> bytes:
        nonlocal changed
        out = bytearray()
        for f, wt, v in parse_fields(part):
            if f == 1 and wt == 2 and isinstance(v, bytes):
                try:
                    txt = v.decode("utf-8")
                except Exception:
                    txt = ""
                if txt.strip() == old_request.strip():
                    out += encode_field(f, 2, newb)
                    changed = True
                else:
                    out += encode_field(f, 2, v)
            else:
                out += encode_field(f, wt, v if not isinstance(v, int) else encode_varint(v))
        return bytes(out)

    out = bytearray()
    for f, wt, v in parse_fields(payload):
        if f == 19 and wt == 2:
            f19 = bytearray()
            for ff, fw, vv in parse_fields(v):
                if ff == 2 and fw == 2 and isinstance(vv, bytes):
                    f19 += encode_field(ff, 2, newb)
                    changed = True
                elif ff == 3 and fw == 2 and isinstance(vv, bytes):
                    f19 += encode_field(ff, 2, patch_part(vv))
                else:
                    f19 += encode_field(ff, fw, vv if not isinstance(vv, int) else encode_varint(vv))
            out += encode_field(f, wt, bytes(f19))
        else:
            out += encode_field(f, wt, v if not isinstance(v, int) else encode_varint(v))
    return bytes(out), changed


SERVICE_STEP_SPECS = {
    101: {"type": "SYSTEM_MESSAGE", "label": "Системное сообщение", "paths": ([114, 1],)},
    23:  {"type": "CHECKPOINT",     "label": "Компакция (сводка)",  "paths": ([30, 5],)},
    17:  {"type": "ERROR_MESSAGE",  "label": "Сообщение об ошибке", "paths": ([24, 3, 2], [24, 3, 1])},
    132: {"type": "GENERIC",        "label": "Вывод инструмента",   "paths": ([140, 2, 1],)},
}


def _extract_pb_str(buf: bytes, chain) -> str | None:
    cur = buf
    for fnum in chain:
        found = None
        for f, wt, v in parse_fields(cur):
            if f == fnum and wt == 2 and isinstance(v, bytes):
                found = v
                break
        if found is None:
            return None
        cur = found
    try:
        return cur.decode("utf-8")
    except UnicodeDecodeError:
        return cur.decode("utf-8", "replace")


def get_service_step_field(stype: int, payload: bytes):
    spec = SERVICE_STEP_SPECS.get(stype)
    if not spec:
        return None, None
    for path in spec["paths"]:
        val = _extract_pb_str(payload, path)
        if val is not None:
            return list(path), val
    return None, None


def preview_step_edit(cid: str, idx: int, store: AgStore | None = None, assistant_name: str = "Antigravity") -> dict:
    s = _resolve_store(store)
    db_path = s.conversations_dir / f"{cid}.db"
    con = connect_ro(db_path)
    try:
        row = con.execute("select step_type, step_payload from steps where idx=?", (idx,)).fetchone()
    finally:
        con.close()
    if not row:
        raise LookupError(f"Шага #{idx} нет в базе данных (существует только в транскрипте), править нечего.")
    stype, payload = row[0], bytes(row[1])
    if stype in (14, 15):
        entries = {e.get("step_index"): e for e in read_history(cid, store=s)}
        cur = (entries.get(idx) or {}).get("content") or ""
        is_user = stype == 14
        hint = (
            "Показан полный текст сообщения включая блоки <ADDITIONAL_METADATA> и <USER_SETTINGS_CHANGE>. "
            "Блок <USER_REQUEST> должен остаться. В базу пишется текст запроса, метаданные в транскрипты."
            if is_user else
            "Текст обновится в SQLite (protobuf), обоих транскриптах, чанках и поисковом индексе FTS5."
        )
        return {
            "text": cur,
            "kind": "Сообщение пользователя" if is_user else f"Ответ {assistant_name}",
            "hint": hint,
            "full_user": is_user,
            "step_type": stype,
        }
    spec = SERVICE_STEP_SPECS.get(stype)
    if not spec:
        raise ValueError(f"Шаг типа {stype} не поддерживает редактирование текста.")
    pb_path, old_pb = get_service_step_field(stype, payload)
    if pb_path is None:
        raise ValueError("В этом служебном шаге нет редактируемого текстового поля в БД (возможно, бинарные данные или вложение).")
    return {
        "text": old_pb,
        "kind": spec["label"],
        "hint": "Текст обновится в SQLite (protobuf) и транскриптах. Перед записью создаётся полная резервная копия.",
        "full_user": False,
        "step_type": stype,
    }


def edit_step_text(cid: str, idx: int, new_text: str, full_user: bool = False, store: AgStore | None = None) -> dict:
    s = _resolve_store(store)
    db_path = s.conversations_dir / f"{cid}.db"
    con = connect_ro(db_path)
    row = con.execute("select step_type, step_payload from steps where idx=?", (idx,)).fetchone()
    con.close()
    if not row:
        raise ValueError(f"step {idx} not found in DB")
    stype, payload = row[0], bytes(row[1])
    if stype not in (14, 15) and stype not in SERVICE_STEP_SPECS:
        raise ValueError(f"шаг {idx} имеет тип {stype}: редактирование текста для этого типа не поддерживается")

    entries = {e.get("step_index"): e for e in read_history(cid, store=s)}
    old_content = (entries.get(idx) or {}).get("content") or ""
    new_content = None
    old_pb = None
    if stype == 14:
        side = "user"
        if full_user:
            if "<USER_REQUEST>" not in new_text:
                raise ValueError("в тексте должен остаться блок <USER_REQUEST>…</USER_REQUEST>")
            new_content = new_text
            new_request = extract_user_request(new_content)
        else:
            new_request = new_text
            new_content = replace_user_request(old_content, new_text)
        new_payload, changed = _patch_user_payload(
            payload, new_request, extract_user_request(old_content))
        if not changed:
            raise ValueError("в этом сообщении не найден текст запроса для замены "
                             "(возможно, только вложение)")
    elif stype == 15:
        side = "agent"
        newb = new_text.encode("utf-8")
        payload = _rebuild_with_nested(payload, [20, 1], newb, all_occ=True, required=True)
        payload = _rebuild_with_nested(payload, [20, 8], newb, all_occ=True, required=False)
        new_payload = payload
        new_request = new_text
    else:
        side = SERVICE_STEP_SPECS[stype]["type"].lower()
        pb_path, old_pb = get_service_step_field(stype, payload)
        if pb_path is None:
            raise ValueError("в этом служебном шаге не найдено текстовое поле для замены")
        new_payload = _rebuild_with_nested(payload, pb_path, new_text.encode("utf-8"), all_occ=False, required=True)
        new_request = new_text
    con = connect_rw(db_path)
    try:
        con.execute("begin")
        con.execute("update steps set step_payload=? where idx=?", (new_payload, idx))
        con.execute("commit")
        chk = [r[0].decode() for r in con.execute("pragma integrity_check")]
        assert chk == ["ok"], f"integrity_check fail: {chk}"
    except Exception:
        if con.in_transaction:
            con.execute("rollback")
        raise
    finally:
        con.close()
    report = {"cid": cid, "step_index": idx, "side": side, "integrity_check": "ok"}
    any_changed_t = False
    want_type = (
        "PLANNER_RESPONSE" if stype == 15 else
        "USER_INPUT" if stype == 14 else
        SERVICE_STEP_SPECS[stype]["type"]
    )
    for full in (True, False):
        lines = read_transcript_lines(cid, full=full, store=s)
        file_changed = False
        for i, line in enumerate(lines):
            try:
                obj = json.loads(line)
            except Exception:
                continue
            if obj.get("step_index") == idx and (obj.get("type") == want_type or stype in SERVICE_STEP_SPECS):
                if stype == 14:
                    obj["content"] = new_content
                elif stype == 15:
                    obj["content"] = new_text
                else:
                    cur_c = obj.get("content") or ""
                    if old_pb and old_pb in cur_c:
                        obj["content"] = cur_c.replace(old_pb, new_text, 1)
                    else:
                        obj["content"] = new_text
                lines[i] = json.dumps(obj, ensure_ascii=False)
                file_changed = True
                break
        if file_changed:
            write_transcript_lines(cid, lines, full=full, store=s)
            any_changed_t = True
    report["transcripts_updated"] = any_changed_t

    if stype == 132:
        out_file = brain_dir(cid, store=s) / "steps" / str(idx) / "output.txt"
        if out_file.is_file():
            try:
                cur_out = out_file.read_text(encoding="utf-8", errors="replace")
                if old_pb and old_pb in cur_out:
                    out_file.write_text(cur_out.replace(old_pb, new_text, 1), encoding="utf-8")
                else:
                    out_file.write_text(new_text, encoding="utf-8")
                report["step_output_updated"] = True
            except Exception:
                pass

    if stype in (14, 15):
        fts_new = (new_request + "\n").encode("utf-8") if stype == 14 else new_text.encode("utf-8")
        fupd = _fts_replace_segment(cid, idx, side, fts_new, store=s)
        report.update(fupd)
    return report


def _fts_orjson(x):
    return x.decode("utf-8", "replace") if isinstance(x, (bytes, bytearray)) else x


def _fts_replace_segment(cid: str, idx: int, side: str, newb: bytes, store: AgStore | None = None) -> dict:
    s = _resolve_store(store)
    if not s.summaries_db.exists():
        return {"fts_updated": False, "fts_reason": "no summaries db"}
    keys = {"u_off": f"{side}_step_byte_offsets", "u_body": f"{side}_body"}
    con = connect_rw(s.summaries_db)
    try:
        has_fts = con.execute(
            "select 1 from sqlite_master where type='table' and name='conversations_fts'"
        ).fetchone()
        if not has_fts:
            return {"fts_updated": False, "fts_reason": "no fts table"}
        con.execute("begin")
        rows = con.execute(
            f"select rowid, chunk_first_step, chunk_last_step, {keys['u_off']}, {keys['u_body']} "
            f"from conversations_fts where conversation_id=?", (cid,)
        ).fetchall()
        done = None
        n_remapped = 0
        for rowid, first, last, offs_raw, body in rows:
            off_txt = _fts_orjson(offs_raw)
            if not off_txt:
                continue
            offs = json.loads(off_txt)
            mine = [o for o in offs if _as_int(o[0]) == idx]
            if not mine:
                continue
            o0 = _as_int(mine[0][1])
            later = sorted(_as_int(o[1]) for o in offs if _as_int(o[1]) > o0)
            b1 = later[0] if later else 10**12
            body_bytes = body if isinstance(body, (bytes, bytearray)) else body.encode("utf-8")
            b1 = min(b1, len(body_bytes))
            old_len = b1 - o0
            new_body = body_bytes[:o0] + newb + body_bytes[b1:]
            delta = len(newb) - old_len
            new_offs = []
            for st, off in offs:
                offv = _as_int(off)
                if st == idx:
                    new_offs.append([st, o0])
                elif offv >= b1:
                    new_offs.append([st, offv + delta])
                    n_remapped += 1
                else:
                    new_offs.append([st, offv])
            if isinstance(body, (bytes, bytearray)):
                nb, no = bytes(new_body), json.dumps(new_offs)
            else:
                nb, no = new_body.decode("utf-8", "replace"), json.dumps(new_offs)
            con.execute(
                f"update conversations_fts set {keys['u_off']}=?, {keys['u_body']}=? where rowid=?",
                (no, nb, rowid),
            )
            done = {"fts_updated": True, "fts_rowid": rowid, "chunk_range": [first, last],
                    "old_seg_len": old_len, "new_seg_len": len(newb), "offsets_remapped": n_remapped,
                    "chunk_first_step": first, "chunk_last_step": last}
            break
        con.execute("commit")
        return done or {"fts_updated": False, "fts_reason": f"step {idx} not found in FTS {side} body"}
    except Exception:
        if con.in_transaction:
            con.execute("rollback")
        raise
    finally:
        con.close()


def check_leak(cid: str, removed_entries, kept_entries=(), allow_paths=(), store: AgStore | None = None) -> list:
    s = _resolve_store(store)
    leaks = []
    kept_norm = "\n".join(
        (e.get("content") or "").replace("\n", " ") for e in kept_entries
    ).lower()
    samples = set()
    for e in removed_entries:
        c = e.get("content") or ""
        if len(c) > 120:
            frag = c[40:120].replace("\n", " ")
            if frag.lower() in kept_norm:
                continue
            samples.add(frag)
    if not samples:
        return leaks
    b = brain_dir(cid, store=s)
    text_targets = [
        b / "logs" / "transcript.jsonl",
        b / "logs" / "transcript_full.jsonl",
    ]
    mdir = b / "messages"
    if mdir.exists():
        text_targets.extend(mdir.glob("*.json"))
    blobs = [(t, s_item) for t in text_targets if t.exists() and str(t) not in allow_paths
             for s_item in samples]
    if blobs:
        data_by_file = {}
        for t in {t for t, _ in blobs}:
            try:
                data_by_file[t] = t.read_bytes()
            except Exception:
                data_by_file[t] = b""
        for t, sample in blobs:
            blob = sample.encode("utf-8", "ignore")
            if blob in data_by_file[t]:
                leaks.append((str(t), sample))
    if s.summaries_db.exists() and str(s.summaries_db) not in allow_paths:
        try:
            con = connect_ro(s.summaries_db)
            for sample in samples:
                tokens = [w for w in re.findall(r"[A-Za-zА-Яа-я0-9]+", sample) if len(w) > 3][:10]
                if len(tokens) < 3:
                    continue
                phrase = '"' + " ".join(tokens) + '"'
                rows = con.execute(
                    "select conversation_id, chunk_index from conversations_fts "
                    "where conversations_fts match ? and conversation_id=?", (phrase, cid)
                ).fetchall()
                if rows:
                    leaks.append((str(s.summaries_db) + " (FTS MATCH)", sample[:60]))
            con.close()
        except Exception as e:
            leaks.append((str(s.summaries_db) + " (проверка FTS не удалась)", repr(e)))
    return leaks


def choose_cid() -> str | None:
    convs = list_conversations()
    cwd_conversations = CONVERSATIONS_DIR
    print()
    print("=== Обнаруженные диалоги (Antigravity) ===")
    print("  папка:", cwd_conversations)
    for i, c in enumerate(convs, 1):
        print(f"{i:>3}. {c['id']}")
        print(f"     название: {c['title']}")
        print(f"     размер: {c['size']:,} Б, изменён: {c['mtime']:%d.%m.%Y %H:%M:%S}, "
              f"шагов в БД: {c['steps_in_db'][0] if c['steps_in_db'] else '?'}")
    if not convs:
        print("Диалоги не найдены.")
        return None
    while True:
        s = input("Выберите номер диалога (или Enter для выхода): ").strip()
        if not s:
            return None
        if s.isdigit() and 1 <= int(s) <= len(convs):
            return convs[int(s) - 1]["id"]
        print("Неверный ввод.")


def choose_rollback_point(cid: str):
    entries = read_history(cid)
    if not entries:
        print("История пуста или transcript не найден.")
        return None
    last = entries[-1]["step_index"]
    nturns = sum(1 for e in entries if e.get("type") == "USER_INPUT")
    print()
    print(f"Всего шагов: {len(entries)} (макс. индекс {last}), пользовательских ходов: {nturns}")
    print("Доступные варианты отката:")
    print("  1. По ходам (turn = пользовательское сообщение со всеми ответами/инструментами)")
    print("  2. Произвольный шаг (index)")
    s = input("Вариант [1/2]: ").strip()
    if s == "2":
        idx = input(f"Индекс шага, до которого сохранить всё (0..{last}): ").strip()
        if not idx.isdigit() or not (0 <= int(idx) <= last):
            print("Неверный индекс.")
            return None
        return int(idx)
    print_history(entries, n=15, from_end=True)
    print("Номер хода - сколько ХОДОВ оставить (самый ранний 1).")
    keep = input(f"Оставить первые N ходов (1..{nturns}, 0=отмена): ").strip()
    if not keep.isdigit():
        return None
    n = int(keep)
    if n == 0:
        return None
    if n > nturns:
        print("N больше числа ходов - откат не требуется.")
        return None
    turn_lines = [e for e in entries if e.get("type") == "USER_INPUT"]
    if n < len(turn_lines):
        return turn_lines[n]["step_index"] - 1
    return last


def show_preview(cid: str, keep_step: int, mode: str = "tail"):
    entries = read_history(cid)
    if mode == "tail":
        kept = [e for e in entries if e["step_index"] <= keep_step]
        removed = [e for e in entries if e["step_index"] > keep_step]
        head = f"=== Будет УДАЛЕНО: {len(removed)} шагов (с индекса {keep_step + 1}) ==="
        tail = f"=== Останется: {len(kept)} шагов (0..{keep_step}) ==="
    else:
        kept = [e for e in entries if e["step_index"] >= keep_step]
        removed = [e for e in entries if e["step_index"] < keep_step]
        head = f"=== Будет УДАЛЕНО: {len(removed)} шагов (до индекса {keep_step}) ==="
        tail = f"=== Останется: {len(kept)} шагов (с {keep_step}) ==="
    print(f"\n{head}")
    for e in removed[:40]:
        c = (e.get("content") or "").replace("\n", " ")[:150]
        tc = "; ".join(x.get("name", "") for x in e.get("tool_calls") or [])
        print(f"  [{e['step_index']:>4}] {e['type']:<16} {c}" + (f"  | tools: {tc}" if tc else ""))
    if len(removed) > 40:
        print(f"  ... и ещё {len(removed) - 40} шагов")
    print(f"\n{tail}")
    for e in kept[-10:]:
        c = (e.get("content") or "").replace("\n", " ")[:150]
        print(f"  [{e['step_index']:>4}] {e['type']:<16} {c}")
    prep = rollback(cid, keep_step, preview_only=True, mode=mode)
    print("\nДополнительно будет удалено:")
    print("  строки транскрипта (compact+full+чанки):", prep["transcript_lines_removed"])
    print("  files сообщений:", prep["will_delete"]["messages_files"],
          "\n  папок steps/<n>:", prep["will_delete"]["steps_output_dirs"],
          "\n  логов задач:", prep["will_delete"]["task_logs"])
    con = connect_ro(CONVERSATIONS_DIR / f"{cid}.db")
    cmp = "<" if mode == "head" else ">"
    print("  строк в таблице steps:", con.execute(f"select count(*) from steps where idx {cmp} ?", (keep_step,)).fetchone()[0])
    con.close()


def apply_rollback(cid: str, keep_step: int, mode: str = "tail"):
    ok, running = verify_not_running()
    if not ok:
        print("\n!!! Antigravity, похоже, запущен:")
        for r in running:
            print("   ", r)
        s = input("Продолжать всё равно НЕ рекомендуется. Введите 'нет' для отмены: ").strip().lower()
        if s not in ("да", "y", "yes", "продолжить"):
            print("Отменено.")
            return
    print("\n>>> Шаг 1: резервная копия...")
    bdir = full_backup(cid, tag="prerollback")
    print("    Бэкап:", bdir)
    print("\n>>> Шаг 2: подтверждение")
    if mode == "head":
        print(f"    Будут удалены ВСЕ шаги индексом < {keep_step} (во всех хранилищах).")
    else:
        print(f"    Будут удалены ВСЕ шаги индексом > {keep_step} (во всех хранилищах).")
    s = input("    Для подтверждения наберите 'УДАЛИТЬ': ").strip()
    if s != "УДАЛИТЬ":
        print("Отменено.")
        return
    print("\n>>> Шаг 3: выполнение отката...")
    try:
        report = rollback(cid, keep_step, preview_only=False, mode=mode)
    except Exception as e:
        print("ОШИБКА во время отката:", repr(e))
        print("Автовосстановление из резервной копии...")
        restore_backup(bdir)
        print("Восстановлено. Ничего не изменилось.")
        raise
    for k, v in report.items():
        print(f"    {k}: {v}")
    print("\n>>> Шаг 4: проверка утечек удалённого текста...")
    btr = bdir / "brain" / cid / ".system_generated" / "logs" / "transcript_full.jsonl"
    if not btr.exists():
        btr = bdir / "brain" / cid / ".system_generated" / "logs" / "transcript.jsonl"
    removed, kept = [], []
    if btr.exists():
        for line in read_jsonl(btr):
            d = json.loads(line)
            goes = d["step_index"] > keep_step if mode == "tail" else d["step_index"] < keep_step
            (removed if goes else kept).append(d)
    leaks = check_leak(cid, removed, kept)
    if leaks:
        print("    !!! ОБНАРУЖЕНЫ УТЕЧКИ:")
        for t, s in leaks[:20]:
            print("      ", t, "->", s[:60])
    else:
        print("    Утечек не обнаружено: удалённый текст отсутствует в транскриптах, чанках и FTS.")
    print("\nГотово. Рекомендуется перезапустить Antigravity и проверить диалог.")


def menu_view_conversation(cid):
    entries = read_history(cid)
    print(f"\n                       (Найдено шагов: {len(entries)})")
    print("  Вывод будет показан по ходам. Как много показать?")
    print("   Enter - последние 15 ходов, 'all' - все")
    s = input("  > ").strip()
    if s.lower() == "all":
        print_history(entries)
    else:
        n = int(s) if s.isdigit() and int(s) > 0 else 15
        print_history(entries, n=n, from_end=True)


def menu_recent(cid):
    entries = read_history(cid)
    print(f"\nВсего шагов: {len(entries)}. Показать последние N ходов.")
    s = input("N (по умолч. 5): ").strip() or "5"
    n = int(s) if s.isdigit() and int(s) > 0 else 5
    print_history(entries, n=n, from_end=True)


def main_menu():
    if not APPDATA.exists():
        print(f"Папка Antigravity не найдена: {APPDATA}")
        return
    print()
    print("=" * 70)
    print(" Antigravity History Editor  --  откат истории диалогов")
    print("=" * 70)
    ok, running = verify_not_running()
    if not ok:
        print(" ВНИМАНИЕ: Antigravity запущен:")
        for r in running:
            print("   ", r)
        print(" Изменения при запущенном приложении небезопасны.")
    else:
        print(" Antigravity не запущен (запись безопасна).")
    print()
    print(" 1. Список диалогов")
    print(" 2. Просмотр истории выбранного диалога")
    print(" 3. Просмотр последних N сообщений")
    print(" 4. Выбор точки отката + превью + применение")
    print(" 5. Создать резервную копию")
    print(" 6. Восстановить из резервной копии")
    print(" 7. Открыть ВЕБ-интерфейс (красивый просмотр/редактор - рекомендуется)")
    print(" 0. Выход")
    choice = input("\nВыбор: ").strip()
    if choice == "1":
        list_conversations()
        choose_cid()
    elif choice in ("2", "3", "4"):
        cid = choose_cid()
        if not cid:
            return
        if choice == "2":
            menu_view_conversation(cid)
        elif choice == "3":
            menu_recent(cid)
        else:
            ks = choose_rollback_point(cid)
            if ks is None:
                return
            show_preview(cid, ks)
            if input("\nПродолжить к применению? [y/N]: ").strip().lower() not in ("y", "yes", "да", "д"):
                return
            apply_rollback(cid, ks)
    elif choice == "5":
        cid = choose_cid()
        if cid:
            bdir = full_backup(cid)
            print("Бэкап создан:", bdir)
    elif choice == "6":
        cid = choose_cid()
        if not cid:
            return
        root = BACKUP_ROOT / cid
        if not root.exists():
            print("Бэкапов для диалога нет:", root)
            return
        bdirs = sorted(root.iterdir())
        for i, bd in enumerate(bdirs, 1):
            print(f"{i}. {bd.name}")
        s = input("Номер бэкапа: ").strip()
        if s.isdigit() and 1 <= int(s) <= len(bdirs):
            ok, running = verify_not_running()
            if not ok:
                print("Antigravity запущен - восстановление отменено.")
                return
            restore_backup(bdirs[int(s) - 1])
    elif choice == "7":
        import subprocess
        web = Path(__file__).resolve().parent / "web_editor.py"
        subprocess.Popen([sys.executable, "-X", "utf8", str(web)])
        print("\nВеб-интерфейс запускается: http://127.0.0.1:8765")
        print("(если браузер не открылся - перейдите по адресу вручную)")


if __name__ == "__main__":
    if len(sys.argv) > 1:
        if sys.argv[1] == "--list":
            for c in list_conversations():
                print(c["id"], c["steps_in_db"], c["title"])
        elif sys.argv[1] == "--preview" and len(sys.argv) > 2:
            ks = int(sys.argv[3]) if len(sys.argv) > 3 else int(input("keep_step: "))
            show_preview(sys.argv[2], ks)
        elif sys.argv[1] == "--rollback" and len(sys.argv) > 3:
            apply_rollback(sys.argv[2], int(sys.argv[3]))
        elif sys.argv[1] == "--keep-from" and len(sys.argv) > 3:
            apply_rollback(sys.argv[2], int(sys.argv[3]), mode="head")
        elif sys.argv[1] == "--preview-head" and len(sys.argv) > 3:
            show_preview(sys.argv[2], int(sys.argv[3]), mode="head")
        elif sys.argv[1] == "--edit-text" and len(sys.argv) > 4:
            cid, idx, src = sys.argv[2], int(sys.argv[3]), Path(sys.argv[4])
            new_text = src.read_text(encoding="utf-8").strip()
            ok, running = verify_not_running()
            if not ok:
                print("Antigravity запущен - правка отменена:")
                for r in running:
                    print("   ", r)
                sys.exit(1)
            bdir = full_backup(cid, tag="preedit")
            print("Бэкап:", bdir)
            print(json.dumps(edit_step_text(cid, idx, new_text), ensure_ascii=False, indent=1))
        elif sys.argv[1] == "--backup" and len(sys.argv) > 2:
            print(full_backup(sys.argv[2]))
        else:
            print(__doc__)
    else:
        while True:
            main_menu()
