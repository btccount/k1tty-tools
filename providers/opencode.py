from __future__ import annotations

import json
from pathlib import Path

import opencode_history_tool as OC
from process_monitor import format_running, snapshot_processes
from providers import BaseProvider, dir_size_bytes


class OpenCodeProvider(BaseProvider):
    id = "opencode"
    name = "OpenCode"
    short = "OC"

    def __init__(self, db_path: Path | None = None, backup_root: Path | None = None):
        super().__init__()
        self.db_path = Path(db_path) if db_path else OC.DB_PATH
        self.backup_root = Path(backup_root) if backup_root else OC.BACKUP_ROOT

    def _in_backups(self, p: Path) -> bool:
        try:
            rp = p.resolve()
            root = self.backup_root.resolve()
            return rp == root or rp.is_relative_to(root)
        except Exception:
            return False

    def _write_gate(self) -> list[str]:
        ok, msg = OC.db_write_probe(self.db_path)
        if not ok:
            raise RuntimeError(
                f"База данных OpenCode занята другим процессом: {msg}. Закройте OpenCode и повторите."
            )
        warns: list[str] = []
        running = snapshot_processes(force=True).get("opencode", [])
        if running:
            warns.append(
                "OpenCode запущен, но база доступна в WAL-режиме. "
                "Переоткройте диалог в OpenCode после правки."
            )
        return warns

    def state_sync(self, running_map: dict[str, list[str]]) -> dict:
        running = running_map.get("opencode", [])
        avail = self.db_path.is_file()
        ok_w, _ = OC.db_write_probe(self.db_path) if avail else (False, "")
        blocked = bool(avail and not ok_w)
        if not avail:
            note = f"База данных не найдена: {self.db_path}"
        elif blocked:
            note = f"База занята ({format_running(running)}). Закройте OpenCode для правки."
        elif running:
            note = f"Запущено: {format_running(running)}. Переоткройте диалог в OpenCode после правки."
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
            "test": False,
            "appdata": str(self.db_path.parent),
            "backups_root": str(self.backup_root),
            "note": note,
        }

    def list_convs_sync(self, refresh: bool = False) -> list[dict]:
        if not self.db_path.is_file():
            return []
        out: list[dict] = []
        for s in OC.list_sessions(self.db_path):
            dname = Path(s["directory"]).name if s["directory"] else ""
            out.append({
                "provider": self.id,
                "id": s["id"],
                "title": s["title"],
                "subtitle": s["directory"] or s["slug"],
                "badge": f"{s['msg_count']} ш." if s["msg_count"] else (dname[:16] or "OC"),
                "size": s["size"],
                "mtime": s["mtime"],
                "mtime_ts": s["mtime_ts"],
                "writable": True,
            })
        return out

    def probe_conv_lock_sync(self, cid: str | None = None) -> tuple[bool, str]:
        if not self.db_path.is_file():
            return False, ""
        ok, msg = OC.db_write_probe(self.db_path)
        if not ok:
            return True, f"База данных OpenCode занята ({msg}). Закройте OpenCode и повторите."
        return False, ""

    def get_entries_sync(self, cid: str, *, limit: int = 80, before=None, include_tools: bool = False) -> dict:
        all_items = OC.read_session_items(cid, db=self.db_path)
        flat = [
            it for it in all_items
            if include_tools or it["kind"] in ("user", "assistant", "error")
        ]
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
            "total_label": f"записей: {total_items}",
            "cursor": str(start) if start > 0 else None,
            "has_more": start > 0,
            "writable": True,
            "file_locked": locked,
            "lock_reason": lock_reason,
        }

    def edit_preview_sync(self, cid: str, key: str) -> dict:
        info = OC.preview_item_edit(cid, key, db=self.db_path)
        return {
            "text": info["text"],
            "kind": info["kind"],
            "hint": "",
            "revision": key,
            "writable": True,
        }

    def apply_edit_sync(self, cid: str, key: str, text: str, revision: str | None, set_phase, *, auto_backup: bool = True) -> dict:
        if not text or not text.strip():
            raise ValueError("Пустой текст, сохранение отменено.")
        warns = self._write_gate()
        bdir = None
        if auto_backup:
            set_phase("backup")
            bdir = OC.backup_session(cid, tag="edit", db=self.db_path, backup_root=self.backup_root)
        set_phase("edit")
        try:
            rep = OC.apply_item_edit(cid, key, text, db=self.db_path)
        except Exception:
            if bdir is not None:
                try:
                    OC.restore_session_backup(bdir, db=self.db_path)
                except Exception:
                    pass
            raise
        rep["backup"] = str(bdir) if bdir is not None else None
        rep["warns"] = warns
        set_phase("done")
        return rep

    def delete_single_sync(self, cid: str, key: str, revision: str | None, set_phase, *, auto_backup: bool = True) -> dict:
        warns = self._write_gate()
        bdir = None
        if auto_backup:
            set_phase("backup")
            bdir = OC.backup_session(cid, tag="del_single", db=self.db_path, backup_root=self.backup_root)
        set_phase("cut")
        try:
            rep = OC.delete_single_item(cid, key, db=self.db_path)
        except Exception:
            if bdir is not None:
                try:
                    OC.restore_session_backup(bdir, db=self.db_path)
                except Exception:
                    pass
            raise
        rep["backup"] = str(bdir) if bdir is not None else None
        rep["warns"] = warns
        set_phase("done")
        return rep

    def preview_cut_sync(self, cid: str, key: str, mode: str) -> dict:
        rep = OC.preview_cut(cid, key, mode, db=self.db_path)
        return {
            "report": rep,
            "revision": f"{mode}:{key}",
            "writable": True,
            "hint": "",
        }

    def apply_cut_sync(self, cid: str, key: str, mode: str, revision: str | None, set_phase, *, auto_backup: bool = True) -> dict:
        warns = self._write_gate()
        bdir = None
        if auto_backup:
            set_phase("backup")
            bdir = OC.backup_session(cid, tag=f"cut_{mode}", db=self.db_path, backup_root=self.backup_root)
        set_phase("cut")
        try:
            rep = OC.apply_cut(cid, key, mode, db=self.db_path)
        except Exception:
            if bdir is not None:
                try:
                    OC.restore_session_backup(bdir, db=self.db_path)
                except Exception:
                    pass
            raise
        rep["backup"] = str(bdir) if bdir is not None else None
        rep["warns"] = warns
        set_phase("done")
        return rep

    def delete_conv_sync(self, cid: str, set_phase, *, auto_backup: bool = True) -> dict:
        if Path(cid).name != cid or cid in (".", ".."):
            raise ValueError("Недопустимый ID сессии.")
        warns = self._write_gate()
        bdir = None
        if auto_backup:
            set_phase("backup")
            bdir = OC.backup_session(cid, tag="delete_conv", db=self.db_path, backup_root=self.backup_root)
        set_phase("cut")
        try:
            OC.delete_session(cid, db=self.db_path)
        except Exception:
            if bdir is not None:
                try:
                    OC.restore_session_backup(bdir, db=self.db_path)
                except Exception:
                    pass
            raise
        set_phase("done")
        return {"deleted": cid, "provider": self.id, "backup": str(bdir) if bdir is not None else None, "warns": warns}

    def backup_root_path(self) -> Path:
        return self.backup_root

    def list_backups_sync(self, cid: str | None = None) -> dict:
        titles: dict[str, str] = {}
        try:
            for s in OC.list_sessions(self.db_path):
                titles[s["id"]] = s["title"]
        except Exception:
            pass

        groups: list[dict] = []
        total_count = 0
        total_bytes = 0
        if not self.backup_root.exists():
            return {"groups": groups, "root": str(self.backup_root), "total_count": 0, "total_bytes": 0}

        roots = [(cid, self.backup_root / cid)] if cid else [
            (p.name, p) for p in sorted(self.backup_root.iterdir(), reverse=True) if p.is_dir()
        ]
        for xcid, cdir in roots:
            if not cdir.exists() or not cdir.is_dir():
                continue
            rows = []
            meta_title = ""
            for bdir in sorted(cdir.iterdir(), reverse=True):
                if not bdir.is_dir():
                    continue
                meta = {}
                mf = bdir / "backup.meta.json"
                if mf.exists():
                    try:
                        meta = json.loads(mf.read_text(encoding="utf-8"))
                        if not meta_title and meta.get("title"):
                            meta_title = str(meta["title"])
                    except Exception:
                        pass
                sz = dir_size_bytes(bdir)
                total_count += 1
                total_bytes += sz
                has_snap = (bdir / "session_backup.json").is_file()
                rows.append({
                    "provider": self.id,
                    "cid": xcid,
                    "dir": str(bdir),
                    "name": bdir.name,
                    "size": sz,
                    "meta": meta,
                    "restorable": has_snap,
                    "inspectable": has_snap,
                    "reason": "",
                })
            if rows:
                groups.append({
                    "provider": self.id,
                    "cid": xcid,
                    "title": titles.get(xcid) or meta_title or xcid,
                    "backups": rows,
                })
        return {
            "groups": groups,
            "root": str(self.backup_root),
            "total_count": total_count,
            "total_bytes": total_bytes,
        }

    def inspect_backup_sync(self, backup_dir: str, *, limit: int = 80, before=None, include_tools: bool = False) -> dict:
        p = Path(backup_dir)
        if not self._in_backups(p) or not p.is_dir():
            raise PermissionError("Недопустимый путь резервной копии OpenCode.")
        all_items, sid, title = OC.read_backup_items(p)
        flat = [
            it for it in all_items
            if include_tools or it["kind"] in ("user", "assistant", "error")
        ]
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
        return {
            "provider": self.id,
            "cid": sid,
            "backup_dir": str(p),
            "backup_name": p.name,
            "title": title,
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
            raise PermissionError("Недопустимый путь резервной копии OpenCode.")
        self._write_gate()
        set_phase("restore")
        OC.restore_session_backup(p, db=self.db_path)
        set_phase("done")
        return {"restored": str(p), "provider": self.id}
