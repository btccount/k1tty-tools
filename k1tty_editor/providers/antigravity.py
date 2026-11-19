from __future__ import annotations

import json
import os
import shutil
import sqlite3
from datetime import datetime
from pathlib import Path

import ag_history_tool as T
from process_monitor import format_running, snapshot_processes
from providers import BaseProvider, dir_size_bytes

_LEGACY_AG_BACKUPS = Path.home() / "Documents" / "Проект по умолчанию" / "antigravity_history_editor" / "backups"
if not os.environ.get("ANTIGRAVITY_BACKUP_ROOT"):
    if _LEGACY_AG_BACKUPS.exists():
        T.BACKUP_ROOT = _LEGACY_AG_BACKUPS
    else:
        T.BACKUP_ROOT = Path(__file__).resolve().parent.parent / "backups" / "antigravity"

T.antigravity_running = lambda: snapshot_processes(T.DAEMON_DIR).get("antigravity", [])

KIND = {14: "user", 15: "assistant", 132: "tool", 23: "compact", 101: "system", 17: "error"}
KIND_BY_NAME = {
    "USER_INPUT": "user", "PLANNER_RESPONSE": "assistant", "GENERIC": "tool",
    "CHECKPOINT": "compact", "SYSTEM_MESSAGE": "system", "ERROR": "error", "ERROR_MESSAGE": "error",
}


def _kind_of(step_type) -> str:
    if isinstance(step_type, str):
        return KIND_BY_NAME.get(step_type, "other")
    return KIND.get(step_type, "other")


class AntigravityProvider(BaseProvider):
    id = "antigravity"
    name = "Antigravity"
    short = "AG"

    def __init__(
        self,
        *,
        provider_id: str = "antigravity",
        name: str = "Antigravity",
        short: str = "AG",
        store: T.AgStore | None = None,
    ):
        super().__init__()
        self.id = provider_id
        self.name = name
        self.short = short
        self.store = store or T.default_store()
        self._label = {
            "user": "Вы",
            "assistant": self.name,
            "tool": "Инструмент",
            "compact": "Компакция",
            "system": "Система",
            "error": "Ошибка",
            "other": "Прочее",
        }
        self._db_stat_cache: dict[str, tuple[int, int, tuple[int, int] | None]] = {}
        self._turns_cache: dict[str, tuple[tuple, list]] = {}

    def _test_mode(self) -> bool:
        return os.environ.get("ANTIGRAVITY_WEB_TEST") == "1" and "sandbox" in str(self.store.appdata).lower()

    def _invalidate(self, cid: str | None = None):
        if cid:
            self._turns_cache.pop(cid, None)
            self._db_stat_cache.pop(cid, None)
        else:
            self._turns_cache.clear()
            self._db_stat_cache.clear()

    def _in_backups(self, p: Path) -> bool:
        try:
            rp = p.resolve()
            root = self.store.backup_root.resolve()
            return rp == root or rp.is_relative_to(root)
        except Exception:
            return False

    def _any_dialog_locked(self) -> bool:
        if not self.store.conversations_dir.exists():
            return False
        try:
            for p in self.store.conversations_dir.glob("*.db"):
                ok, _ = T.db_write_probe(p)
                if not ok:
                    return True
        except Exception:
            pass
        return False

    def _write_gate(self, cid: str) -> list[str]:
        warns: list[str] = []
        db = self.store.conversations_dir / f"{cid}.db"
        ok, msg = T.db_write_probe(db)
        if not ok:
            raise RuntimeError(
                f"Файлы диалога {self.name} сейчас заняты другим процессом "
                f"(скорее всего {self.name}): {msg}. Закройте {self.name} и повторите."
            )
        running = snapshot_processes(self.store.daemon_dir, force=True).get(self.id, [])
        if running:
            warns.append(
                f"{self.name} запущен, но файлы этого диалога свободны. "
                "Изменения появятся в приложении после его перезапуска."
            )
        if self._test_mode():
            warns.append("Тест-режим: данные пишутся в песочницу.")
        return warns

    def state_sync(self, running_map: dict[str, list[str]]) -> dict:
        running = running_map.get(self.id, [])
        avail = self.store.conversations_dir.exists()
        blocked = bool(not self._test_mode() and running and self._any_dialog_locked())
        if not avail:
            note = f"Каталог не найден: {self.store.appdata}"
        elif self._test_mode():
            note = "Тест-режим: изменения пишутся в песочницу."
        elif blocked:
            note = f"Файлы диалогов заняты ({format_running(running)}). Закройте приложение для правки."
        elif running:
            note = f"Запущено: {format_running(running)}, файлы свободны. Изменения применятся после перезапуска."
        else:
            note = "Файлы свободны."
        return {
            "id": self.id,
            "name": self.name,
            "short": self.short,
            "available": avail,
            "writable": avail and not blocked,
            "blocked": blocked,
            "running": running,
            "test": self._test_mode(),
            "appdata": str(self.store.appdata),
            "backups_root": str(self.store.backup_root),
            "note": note,
        }

    def list_convs_sync(self, refresh: bool = False) -> list[dict]:
        if refresh:
            self._db_stat_cache.clear()
        if not self.store.conversations_dir.exists():
            return []

        titles: dict[str, str] = {}
        steps_summary: dict[str, int] = {}
        try:
            con = T.connect_ro(self.store.summaries_db)
            try:
                for cid_b, title_b, sc in con.execute(
                    "select conversation_id, title, step_count from conversation_summaries"
                ):
                    cid_s = cid_b.decode("utf-8", "replace")
                    titles[cid_s] = (title_b or b"").decode("utf-8", "replace")
                    steps_summary[cid_s] = int(sc)
            finally:
                con.close()
        except Exception:
            pass

        out: list[dict] = []
        for db in self.store.conversations_dir.glob("*.db"):
            try:
                st = db.stat()
            except OSError:
                continue
            cid = db.stem
            cached = self._db_stat_cache.get(cid)
            if cached and cached[0] == st.st_mtime_ns and cached[1] == st.st_size:
                n = cached[2]
            else:
                n = None
                try:
                    con = T.connect_ro(db)
                    try:
                        row = con.execute("select count(*), max(idx) from steps").fetchone()
                        if row:
                            n = (int(row[0]), int(row[1]) if row[1] is not None else 0)
                    finally:
                        con.close()
                except Exception:
                    pass
                self._db_stat_cache[cid] = (st.st_mtime_ns, st.st_size, n)

            step_cnt = n[0] if n else steps_summary.get(cid)
            out.append({
                "provider": self.id,
                "id": cid,
                "title": titles.get(cid) or f"Диалог {cid[:8]}",
                "subtitle": f"шагов: {step_cnt}" if step_cnt is not None else "SQLite + transcript",
                "badge": f"{step_cnt} ш." if step_cnt is not None else self.short,
                "size": st.st_size,
                "mtime": datetime.fromtimestamp(st.st_mtime).strftime("%d.%m.%Y %H:%M"),
                "mtime_ts": st.st_mtime,
                "writable": True,
            })
        out.sort(key=lambda x: x["mtime_ts"], reverse=True)
        return out

    def _build_turns_cached(self, cid: str) -> list[dict]:
        b = T.brain_dir(cid, store=self.store)
        p_full = b / "logs" / "transcript_full.jsonl"
        p_compact = b / "logs" / "transcript.jsonl"
        src = p_full if p_full.exists() else p_compact
        db = self.store.conversations_dir / f"{cid}.db"

        def _sig(p: Path):
            try:
                st = p.stat()
                return (st.st_mtime_ns, st.st_size)
            except OSError:
                return (0, 0)

        sig = (_sig(src), _sig(db), _sig(db.with_name(db.name + "-wal")))
        cached = self._turns_cache.get(cid)
        if cached and cached[0] == sig:
            return cached[1]

        db_types: dict[int, int] = {}
        try:
            con = T.connect_ro(db)
            try:
                db_types = {int(r[0]): int(r[1]) for r in con.execute("select idx, step_type from steps")}
            finally:
                con.close()
        except Exception:
            pass

        turns: list[dict] = []
        cur = None
        for e in T.read_history(cid, store=self.store):
            k = _kind_of(e.get("type"))
            idx = e.get("step_index")
            exists_in_db = idx in db_types
            stype = db_types.get(idx)
            tools = [c.get("name") for c in (e.get("tool_calls") or []) if c.get("name")]
            content_text = e.get("content") or ""

            actions: list[str] = []
            if exists_in_db and (
                stype == 14
                or (stype == 15 and bool(content_text.strip()))
                or (stype in T.SERVICE_STEP_SPECS and bool(content_text.strip()))
            ):
                actions.append("edit")
            if k == "user":
                actions.append("cut_head")
            actions.append("cut_tail")

            label = self._label.get(k, "Прочее")
            if tools and k != "user":
                label = f"{label}: {', '.join(tools[:3])}"

            it = {
                "key": str(idx),
                "step_index": idx,
                "kind": k,
                "subtype": str(e.get("type") or k).lower(),
                "label": label,
                "text": e.get("content") or "",
                "ts": e.get("created_at") or "",
                "tool": ", ".join(tools) if tools else None,
                "status": e.get("status"),
                "in_db": exists_in_db,
                "actions": actions,
            }
            if k == "user":
                cur = {"user": it, "items": [it]}
                turns.append(cur)
            else:
                if cur is None:
                    cur = {"user": None, "items": []}
                    turns.append(cur)
                cur["items"].append(it)

        if len(self._turns_cache) >= 8:
            self._turns_cache.pop(next(iter(self._turns_cache)))
        self._turns_cache[cid] = (sig, turns)
        return turns

    def probe_conv_lock_sync(self, cid: str | None = None) -> tuple[bool, str]:
        if cid:
            db = self.store.conversations_dir / f"{cid}.db"
            if db.is_file():
                ok, msg = T.db_write_probe(db)
                if not ok:
                    return True, f"Файл диалога {self.name} занят ({msg}). Закройте {self.name} и повторите."
        if self.store.summaries_db.is_file():
            ok_s, msg_s = T.db_write_probe(self.store.summaries_db)
            if not ok_s:
                return True, f"База {self.name} занята ({msg_s}). Закройте {self.name} и повторите."
        return False, ""

    def get_entries_sync(self, cid: str, *, limit: int = 80, before=None, include_tools: bool = False) -> dict:
        all_turns = self._build_turns_cached(cid)
        flat: list[dict] = []
        for t in all_turns:
            for it in t["items"]:
                if not include_tools and it["kind"] not in ("user", "assistant"):
                    continue
                flat.append(it)

        total_items = len(flat)
        end = total_items
        if before is not None and str(before).isdigit():
            end = min(total_items, max(0, int(before)))

        msg_limit = min(150, max(1, int(limit)))
        max_items = 350 if include_tools else msg_limit * 2
        start = end
        msgs = 0
        while start > 0 and msgs < msg_limit and (end - start) < max_items:
            start -= 1
            if flat[start]["kind"] in ("user", "assistant"):
                msgs += 1

        window = flat[start:end]
        locked, lock_reason = self.probe_conv_lock_sync(cid)
        return {
            "provider": self.id,
            "cid": cid,
            "turns": [{"items": window}] if window else [],
            "page_msgs": msgs,
            "total_label": f"ходов: {len(all_turns)} · записей: {total_items}",
            "cursor": str(start) if start > 0 else None,
            "has_more": start > 0,
            "writable": True,
            "file_locked": locked,
            "lock_reason": lock_reason,
        }

    def edit_preview_sync(self, cid: str, key: str) -> dict:
        idx = int(key)
        info = T.preview_step_edit(cid, idx, store=self.store, assistant_name=self.name)
        return {
            "text": info["text"],
            "kind": info["kind"],
            "hint": info["hint"],
            "revision": str(idx),
            "writable": True,
            "full_user": info["full_user"],
        }

    def apply_edit_sync(self, cid: str, key: str, text: str, revision: str | None, set_phase, *, auto_backup: bool = True) -> dict:
        idx = int(key)
        if not text or not text.strip():
            raise ValueError("Пустой текст, сохранение не выполнено.")
        warns = self._write_gate(cid)
        con = T.connect_ro(self.store.conversations_dir / f"{cid}.db")
        try:
            row = con.execute("select step_type from steps where idx=?", (idx,)).fetchone()
        finally:
            con.close()
        stype = row[0] if row else None
        full_user = bool(stype == 14)
        payload_text = text.strip() if stype in (14, 15) else text

        bdir = None
        if auto_backup:
            set_phase("backup")
            bdir = T.full_backup(cid, tag="webedit", store=self.store)
        set_phase("edit")
        try:
            rep = T.edit_step_text(cid, idx, payload_text, full_user=full_user, store=self.store)
        except Exception:
            if bdir is not None:
                try:
                    T.restore_backup(bdir, store=self.store)
                except Exception:
                    pass
            raise
        self._invalidate(cid)
        rep["backup"] = str(bdir) if bdir is not None else None
        rep["warns"] = warns
        set_phase("done")
        return rep

    def preview_cut_sync(self, cid: str, key: str, mode: str) -> dict:
        step_i = int(key)
        keep_step = (step_i - 1) if mode == "tail" else step_i
        rep = T.rollback(cid, keep_step, preview_only=True, mode=mode, store=self.store)
        turns = self._build_turns_cached(cid)
        flat = [it for t in turns for it in t["items"]]
        if mode == "tail":
            will = [it for it in flat if (it["step_index"] or 0) > keep_step]
        else:
            will = [it for it in flat if (it["step_index"] or 0) < keep_step]

        wd = rep.get("will_delete") or {}
        steps_count = wd.get("steps_in_db", len(will))
        preview_list = [
            {
                "key": f"#{x['step_index']}",
                "label": x["label"],
                "text": (x["text"] or "").replace("\n", " ")[:140],
            }
            for x in will[:25]
        ]
        norm = {
            "provider": self.id,
            "mode": mode,
            "steps_count": len(will),
            "raw_count": steps_count,
            "raw_label": "шагов в БД",
            "bytes": None,
            "extra_label": f"транскрипт: -{wd.get('transcript_lines', 0)} / -{wd.get('transcript_full_lines', 0)}",
            "steps_preview": preview_list,
            "raw": rep,
        }
        return {
            "report": norm,
            "revision": f"{mode}:{keep_step}",
            "writable": True,
            "hint": "Будут очищены SQLite-база steps, транскрипты, чанки, очередь сообщений, выводы инструментов и FTS5-индекс. Создаётся полная резервная копия.",
        }

    def apply_cut_sync(self, cid: str, key: str, mode: str, revision: str | None, set_phase, *, auto_backup: bool = True) -> dict:
        step_i = int(key)
        keep_step = (step_i - 1) if mode == "tail" else step_i
        warns = self._write_gate(cid)

        bdir = None
        if auto_backup:
            set_phase("backup")
            bdir = T.full_backup(cid, tag="web", store=self.store)
        set_phase("cut")
        try:
            rep = T.rollback(cid, keep_step, preview_only=False, mode=mode, store=self.store)
        except Exception:
            if bdir is not None:
                try:
                    T.restore_backup(bdir, store=self.store)
                except Exception:
                    pass
            raise
        self._invalidate(cid)
        rep["backup"] = str(bdir) if bdir is not None else None

        if bdir is not None:
            set_phase("leak")
            try:
                btr = bdir / "brain" / cid / ".system_generated" / "logs" / "transcript_full.jsonl"
                if not btr.exists():
                    btr = bdir / "brain" / cid / ".system_generated" / "logs" / "transcript.jsonl"
                removed, kept = [], []
                if btr.exists():
                    for line in T.read_jsonl(btr):
                        obj = json.loads(line)
                        si = obj.get("step_index", -10**9)
                        (removed if (si > keep_step if mode == "tail" else si < keep_step) else kept).append(obj)
                rep["leaks"] = T.check_leak(cid, removed, kept, store=self.store)
            except Exception as e:
                rep["leaks"] = [{"error": repr(e)}]
        else:
            rep["leaks"] = []

        rep["warns"] = warns
        set_phase("done")
        return rep

    def delete_conv_sync(self, cid: str, set_phase, *, auto_backup: bool = True) -> dict:
        if Path(cid).name != cid or cid in (".", ".."):
            raise ValueError("Недопустимый ID диалога.")
        db = self.store.conversations_dir / f"{cid}.db"
        if not db.exists():
            raise FileNotFoundError(f"Диалог {cid} не найден.")
        warns = self._write_gate(cid)

        bdir = None
        if auto_backup:
            set_phase("backup")
            bdir = T.full_backup(cid, tag="delete_conv", store=self.store)
        set_phase("cut")
        try:
            for suffix in ("", "-wal", "-shm"):
                p = db.with_name(db.name + suffix)
                if p.exists():
                    p.unlink()
            b_root = self.store.brain_dir_root / cid
            if b_root.exists():
                shutil.rmtree(b_root, ignore_errors=False)
            ann = self.store.annotations_dir / f"{cid}.pbtxt"
            if ann.exists():
                ann.unlink()
            pres = self.store.presence_dir / f"{cid}.lock"
            if pres.exists():
                try:
                    pres.unlink()
                except OSError:
                    pass
            if self.store.summaries_db.exists():
                con = sqlite3.connect(str(self.store.summaries_db), timeout=15)
                try:
                    con.execute("BEGIN IMMEDIATE")
                    con.execute("DELETE FROM conversation_summaries WHERE conversation_id = ?", (cid,))
                    try:
                        con.execute("DELETE FROM conversations_fts WHERE conversation_id = ?", (cid,))
                    except sqlite3.Error:
                        pass
                    try:
                        con.execute("DELETE FROM conversation_parents WHERE conversation_id = ? OR parent_id = ?", (cid, cid))
                    except sqlite3.Error:
                        pass
                    con.commit()
                except Exception:
                    con.rollback()
                    raise
                finally:
                    con.close()
        except Exception as e:
            if bdir is not None:
                try:
                    T.restore_backup(bdir, store=self.store)
                except Exception as re:
                    raise RuntimeError(f"Ошибка удаления: {e}; восстановление не удалось: {re}. Копия: {bdir}") from e
            raise

        self._invalidate(cid)
        set_phase("done")
        return {"deleted": cid, "provider": self.id, "backup": str(bdir) if bdir is not None else None, "warns": warns}

    def backup_root_path(self) -> Path:
        return self.store.backup_root

    def _backup_transcript_file(self, bdir: Path, cid: str) -> Path | None:
        base = bdir / "brain" / cid / ".system_generated" / "logs"
        for fn in ("transcript_full.jsonl", "transcript.jsonl"):
            p = base / fn
            if p.is_file():
                return p
        return None

    def _backup_title(self, bdir: Path, cid: str, live_titles: dict[str, str] | None = None) -> str:
        if live_titles and live_titles.get(cid):
            return live_titles[cid]
        sdb = bdir / "conversation_summaries.db"
        if sdb.is_file():
            try:
                con = T.connect_ro(sdb)
                try:
                    row = con.execute(
                        "select title from conversation_summaries where conversation_id=?",
                        (cid,),
                    ).fetchone()
                    if row and row[0]:
                        t = row[0].decode("utf-8", "replace").strip()
                        if t:
                            return t
                finally:
                    con.close()
            except Exception:
                pass
        return cid

    def list_backups_sync(self, cid: str | None = None) -> dict:
        titles: dict[str, str] = {}
        try:
            con = T.connect_ro(self.store.summaries_db)
            try:
                for c, title in con.execute("select conversation_id, title from conversation_summaries"):
                    titles[c.decode("utf-8", "replace")] = (title or b"").decode("utf-8", "replace")
            finally:
                con.close()
        except Exception:
            pass

        groups: list[dict] = []
        total_count = 0
        total_bytes = 0
        if not self.store.backup_root.exists():
            return {"groups": groups, "root": str(self.store.backup_root), "total_count": 0, "total_bytes": 0}

        roots = [(cid, self.store.backup_root / cid)] if cid else [
            (p.name, p) for p in sorted(self.store.backup_root.iterdir(), reverse=True) if p.is_dir()
        ]
        for xcid, cdir in roots:
            if not cdir.exists() or not cdir.is_dir():
                continue
            rows = []
            group_title = titles.get(xcid, "")
            for bdir in sorted(cdir.iterdir(), reverse=True):
                if not bdir.is_dir():
                    continue
                meta = {}
                mf = bdir / "backup.meta.json"
                if mf.exists():
                    try:
                        meta = json.loads(mf.read_text(encoding="utf-8"))
                    except Exception:
                        pass
                sz = dir_size_bytes(bdir)
                total_count += 1
                total_bytes += sz
                if not group_title:
                    t = self._backup_title(bdir, xcid)
                    if t != xcid:
                        group_title = t
                inspectable = bool(self._backup_transcript_file(bdir, xcid) or (bdir / f"{xcid}.db").is_file())
                rows.append({
                    "provider": self.id,
                    "cid": xcid,
                    "dir": str(bdir),
                    "name": bdir.name,
                    "size": sz,
                    "meta": meta,
                    "restorable": True,
                    "inspectable": inspectable,
                    "reason": "",
                })
            if rows:
                groups.append({
                    "provider": self.id,
                    "cid": xcid,
                    "title": group_title or xcid,
                    "backups": rows,
                })
        return {
            "groups": groups,
            "root": str(self.store.backup_root),
            "total_count": total_count,
            "total_bytes": total_bytes,
        }

    def inspect_backup_sync(self, backup_dir: str, *, limit: int = 80, before=None, include_tools: bool = False) -> dict:
        p = Path(backup_dir)
        if not self._in_backups(p) or not p.is_dir():
            raise PermissionError(f"Недопустимый путь резервной копии {self.name}.")
        cid = p.parent.name
        mf = p / "backup.meta.json"
        if mf.is_file():
            try:
                m = json.loads(mf.read_text(encoding="utf-8"))
                if m.get("conversation_id"):
                    cid = str(m["conversation_id"])
            except Exception:
                pass

        flat: list[dict] = []
        tr = self._backup_transcript_file(p, cid)
        if tr is not None:
            for line in T.read_jsonl(tr):
                try:
                    e = json.loads(line)
                except Exception:
                    continue
                k = _kind_of(e.get("type"))
                if not include_tools and k not in ("user", "assistant"):
                    continue
                idx = e.get("step_index")
                tools = [c.get("name") for c in (e.get("tool_calls") or []) if c.get("name")]
                label = self._label.get(k, "Прочее")
                if tools and k != "user":
                    label = f"{label}: {', '.join(tools[:3])}"
                flat.append({
                    "key": str(idx),
                    "step_index": idx,
                    "kind": k,
                    "subtype": str(e.get("type") or k).lower(),
                    "label": label,
                    "text": e.get("content") or "",
                    "ts": e.get("created_at") or "",
                    "tool": ", ".join(tools) if tools else None,
                    "status": e.get("status"),
                    "in_db": True,
                    "actions": [],
                })
        elif (p / f"{cid}.db").is_file():
            con = T.connect_ro(p / f"{cid}.db")
            try:
                for idx, stype, payload in con.execute("select idx, step_type, step_payload from steps order by idx asc"):
                    k = _kind_of(int(stype))
                    if not include_tools and k not in ("user", "assistant"):
                        continue
                    ps = T.payload_summary(int(stype), bytes(payload or b""))
                    flat.append({
                        "key": str(idx),
                        "step_index": int(idx),
                        "kind": k,
                        "subtype": k,
                        "label": self._label.get(k, "Прочее"),
                        "text": ps.get("text") or ps.get("args") or "",
                        "ts": "",
                        "tool": ps.get("tool") or None,
                        "status": None,
                        "in_db": True,
                        "actions": [],
                    })
            finally:
                con.close()

        total_items = len(flat)
        end = total_items
        if before is not None and str(before).isdigit():
            end = min(total_items, max(0, int(before)))

        msg_limit = min(150, max(1, int(limit)))
        max_items = 350 if include_tools else msg_limit * 2
        start = end
        msgs = 0
        while start > 0 and msgs < msg_limit and (end - start) < max_items:
            start -= 1
            if flat[start]["kind"] in ("user", "assistant"):
                msgs += 1

        window = flat[start:end]
        live_titles: dict[str, str] = {}
        try:
            con = T.connect_ro(self.store.summaries_db)
            try:
                row = con.execute("select title from conversation_summaries where conversation_id=?", (cid,)).fetchone()
                if row and row[0]:
                    live_titles[cid] = row[0].decode("utf-8", "replace")
            finally:
                con.close()
        except Exception:
            pass

        return {
            "provider": self.id,
            "cid": cid,
            "backup_dir": str(p),
            "backup_name": p.name,
            "title": self._backup_title(p, cid, live_titles),
            "readonly": True,
            "turns": [{"items": window}] if window else [],
            "page_msgs": msgs,
            "total_label": f"записей: {total_items}",
            "cursor": str(start) if start > 0 else None,
            "has_more": start > 0,
            "writable": False,
        }

    def restore_backup_sync(self, backup_dir: str, set_phase) -> dict:
        p = Path(backup_dir)
        if not self._in_backups(p) or not p.exists():
            raise PermissionError(f"Недопустимый путь резервной копии {self.name}.")
        set_phase("restore")
        T.restore_backup(p, store=self.store)
        self._invalidate()
        set_phase("done")
        return {"restored": str(p), "provider": self.id}


def build_cli_provider() -> AntigravityProvider:
    cli_appdata = Path(
        os.environ.get("ANTIGRAVITY_CLI_APPDATA", "")
        or (Path.home() / ".gemini" / "antigravity-cli")
    )
    cli_backup_root = Path(
        os.environ.get("ANTIGRAVITY_CLI_BACKUP_ROOT", "")
        or (Path(__file__).resolve().parent.parent / "backups" / "antigravity_cli")
    )
    return AntigravityProvider(
        provider_id="antigravity_cli",
        name="Antigravity CLI",
        short="CLI",
        store=T.make_store(cli_appdata, cli_backup_root),
    )
