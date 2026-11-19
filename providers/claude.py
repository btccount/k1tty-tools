from __future__ import annotations

import json
import os
import shutil
from pathlib import Path

import claude_history_tool as CL
from claude_io import lock_files, atomic_write_text, win_long_path
from process_monitor import format_running, snapshot_processes
from providers import BaseProvider, dir_size_bytes


LABEL = {
    "user": "Вы", "assistant": "Claude", "tool": "Инструмент",
    "system": "Система", "other": "Прочее",
}


class ClaudeProvider(BaseProvider):
    id = "claude"
    name = "Claude"
    short = "CL"

    def state_sync(self, running_map: dict[str, list[str]]) -> dict:
        running = running_map.get("claude", [])
        avail = CL.PROJECTS_DIR.exists()
        if not avail:
            note = f"Каталог проектов не найден: {CL.PROJECTS_DIR}"
        elif CL._test_mode():
            note = "ТЕСТ-РЕЖИМ: изменения сохраняются в песочницу."
        elif running:
            note = f"Запущено: {format_running(running)}. Закройте активную сессию перед правкой."
        else:
            note = "Готов к работе: правка сообщений, команд, выводов, thinking и safeguards."
        return {
            "id": self.id,
            "name": self.name,
            "short": self.short,
            "available": avail,
            "writable": avail,
            "blocked": False,
            "running": running,
            "test": CL._test_mode(),
            "appdata": str(CL.CLAUDE_HOME),
            "backups_root": str(CL.BACKUPS_DIR),
            "note": note,
        }

    def list_convs_sync(self, refresh: bool = False) -> list[dict]:
        if refresh:
            CL._FILE_CACHE.clear()
        if not CL.PROJECTS_DIR.exists():
            return []
        out: list[dict] = []
        for t in CL.list_threads():
            proj = t.get("project") or ""
            short_proj = Path(proj).name if proj else ""
            out.append({
                "provider": self.id,
                "id": t["id"],
                "title": t["title"],
                "subtitle": proj,
                "badge": short_proj[:16] if short_proj else "CL",
                "size": t["size"],
                "mtime": t["mtime"].strftime("%d.%m.%Y %H:%M"),
                "mtime_ts": t["mtime"].timestamp(),
                "writable": True,
            })
        return out

    def probe_conv_lock_sync(self, cid: str | None = None) -> tuple[bool, str]:
        if cid and os.name != "nt":
            # No mandatory file locks outside Windows: refuse while Claude runs at all.
            running = snapshot_processes().get("claude", [])
            if running:
                return True, (f"Запущено: {format_running(running)}. На macOS/Linux нельзя проверить, "
                              "занят ли конкретный файл, поэтому закройте Claude перед правкой.")
            return False, ""
        if cid:
            try:
                p = CL.session_file(cid)
                with lock_files([p]):
                    pass
            except FileNotFoundError:
                return False, ""
            except OSError:
                return True, "Файл сессии занят Claude. Закройте активную сессию Claude и повторите."
        return False, ""

    def get_entries_sync(self, cid: str, *, limit: int = 80, before=None, include_tools: bool = False) -> dict:
        lim = min(150, max(1, int(limit)))
        items, cursor = CL.read_history_page(cid, limit=lim, before=before, include_tools=include_tools)
        locked, lock_reason = self.probe_conv_lock_sync(cid)
        turns: list[dict] = []
        total_msgs = 0

        for t in CL.turns_from_history(items):
            entries = []
            for it in t["items"]:
                k = it.get("kind", "other")
                actions = ["edit", "del_single"]
                if k == "user":
                    actions.append("cut_head")
                actions.append("cut_tail")

                entry = {
                    "key": it.get("key"),
                    "uuid": it.get("uuid"),
                    "kind": k,
                    "subtype": it.get("subtype") or k,
                    "label": it.get("label") or LABEL.get(k, "Прочее"),
                    "text": it.get("text") or "",
                    "ts": it.get("ts") or "",
                    "tool": it.get("tool"),
                    "status": it.get("status"),
                    "in_db": True,
                    "actions": actions,
                }
                if k in ("user", "assistant"):
                    total_msgs += 1
                entries.append(entry)
            if entries:
                turns.append({"items": entries})

        return {
            "provider": self.id,
            "cid": cid,
            "turns": turns,
            "page_msgs": total_msgs,
            "total_label": f"загружено: {total_msgs}",
            "cursor": cursor,
            "has_more": cursor is not None,
            "writable": True,
            "file_locked": locked,
            "lock_reason": lock_reason,
        }

    def edit_preview_sync(self, cid: str, key: str) -> dict:
        rev = CL.revision(cid)
        r = CL.edit_message(cid, key, "", preview_only=True)
        if CL.revision(cid) != rev:
            raise RuntimeError("Сессия Claude изменилась во время предпросмотра. Обновите тред.")
        return {
            "text": r["old"],
            "kind": r.get("kind") or "Запись",
            "hint": "Изменение сохранится прямо в JSONL-файл сессии Claude Code. Перед записью создаётся автоматическая резервная копия.",
            "revision": rev,
            "writable": True,
        }

    def apply_edit_sync(self, cid: str, key: str, text: str, revision: str | None, set_phase, *, auto_backup: bool = True) -> dict:
        if not text or not text.strip():
            raise ValueError("Пустой текст, сохранение отменено.")
        CL.check_revision(cid, revision or "")
        bdir = None
        if auto_backup:
            set_phase("backup")
            bdir = CL.full_backup(cid, tag="web_edit")
        set_phase("edit")
        try:
            rep = CL.edit_message(cid, key, text, preview_only=False)
        except Exception as e:
            if bdir is not None:
                try:
                    CL.restore_backup(bdir)
                except Exception as re:
                    raise RuntimeError(f"Ошибка: {e}; автовосстановление не удалось: {re}. Копия: {bdir}") from e
            raise
        warns = []
        if snapshot_processes(force=True).get("claude"):
            warns.append("Обнаружен запущенный процесс Claude Code. Перезапустите сессию в терминале.")
        if CL._test_mode():
            warns.append("ТЕСТ-РЕЖИМ: изменения сохранены в песочницу.")
        rep["backup"] = str(bdir) if bdir is not None else None
        rep["warns"] = warns
        set_phase("done")
        return rep

    def preview_cut_sync(self, cid: str, key: str, mode: str) -> dict:
        rev = CL.revision(cid)
        rep = CL.preview_cut_fast(cid, key, mode, delete_selected=(mode == "tail"))
        if CL.revision(cid) != rev:
            raise RuntimeError("Сессия Claude изменилась во время предпросмотра. Обновите тред.")
        wd = rep.get("will_delete") or {}
        norm = {
            "provider": self.id,
            "mode": mode,
            "steps_count": wd.get("steps_preview_count", 0),
            "raw_count": rep.get("rollout_lines", 0),
            "raw_label": "строк JSONL",
            "bytes": rep.get("bytes", 0),
            "extra_label": "DAG-целостность сохраняется",
            "steps_preview": [
                {
                    "key": p.get("key", ""),
                    "label": p.get("label", ""),
                    "text": (p.get("text") or "").replace("\n", " ")[:130],
                }
                for p in (wd.get("steps_preview") or [])
            ],
            "raw": rep,
        }
        return {
            "report": norm,
            "revision": rev,
            "writable": True,
            "hint": "Будут удалены записи из JSONL-файла сессии. При обрезке сверху выбранная запись станет новым корнем (parentUuid = null). Создаётся полная резервная копия.",
        }

    def apply_cut_sync(self, cid: str, key: str, mode: str, revision: str | None, set_phase, *, auto_backup: bool = True) -> dict:
        CL.check_revision(cid, revision or "")
        bdir = None
        if auto_backup:
            set_phase("backup")
            bdir = CL.full_backup(cid, tag="web_cut")
        set_phase("cut")
        try:
            rep = CL.cut(cid, key, preview_only=False, mode=mode, set_phase=set_phase)
        except Exception as e:
            if bdir is not None:
                try:
                    CL.restore_backup(bdir)
                except Exception as re:
                    raise RuntimeError(f"Ошибка: {e}; автовосстановление не удалось: {re}. Копия: {bdir}") from e
            raise
        warns = []
        if snapshot_processes(force=True).get("claude"):
            warns.append("Обнаружен запущенный процесс Claude Code. Перезапустите сессию в терминале.")
        if CL._test_mode():
            warns.append("ТЕСТ-РЕЖИМ: изменения сохранены в песочницу.")
        rep["backup"] = str(bdir) if bdir is not None else None
        rep["warns"] = warns
        set_phase("done")
        return rep

    def delete_single_sync(self, cid: str, key: str, revision: str | None, set_phase, *, auto_backup: bool = True) -> dict:
        if revision:
            CL.check_revision(cid, revision)
        bdir = None
        if auto_backup:
            set_phase("backup")
            bdir = CL.full_backup(cid, tag="del_single")
        set_phase("cut")
        try:
            rep = CL.delete_single(cid, key)
        except Exception as e:
            if bdir is not None:
                try:
                    CL.restore_backup(bdir)
                except Exception as re:
                    raise RuntimeError(f"Ошибка: {e}; автовосстановление не удалось: {re}. Копия: {bdir}") from e
            raise
        warns = []
        if snapshot_processes(force=True).get("claude"):
            warns.append("Обнаружен запущенный процесс Claude Code. Перезапустите сессию в терминале.")
        rep["backup"] = str(bdir) if bdir is not None else None
        rep["warns"] = warns
        set_phase("done")
        return rep

    def delete_conv_sync(self, cid: str, set_phase, *, auto_backup: bool = True) -> dict:
        if Path(cid).name != cid or cid in (".", ".."):
            raise ValueError("Недопустимый ID сессии.")
        path = CL.session_file(cid)

        bdir = None
        if auto_backup:
            set_phase("backup")
            bdir = CL.full_backup(cid, tag="delete_conv")
        set_phase("cut")

        with lock_files([path]):
            pass
        lp_path = win_long_path(path)
        if os.path.exists(lp_path):
            os.unlink(lp_path)

        sub_dir = win_long_path(path.parent / cid)
        if os.path.isdir(sub_dir):
            shutil.rmtree(sub_dir, ignore_errors=True)

        if CL.HISTORY_JSONL.exists():
            try:
                raw_lines = CL.HISTORY_JSONL.read_text(encoding="utf-8", errors="replace").splitlines()
                kept = []
                changed = False
                for ln in raw_lines:
                    if cid in ln:
                        try:
                            if json.loads(ln).get("sessionId") == cid:
                                changed = True
                                continue
                        except Exception:
                            pass
                    kept.append(ln)
                if changed:
                    atomic_write_text(CL.HISTORY_JSONL, "\n".join(kept) + ("\n" if kept else ""))
            except Exception:
                pass

        CL._FILE_CACHE.clear()
        set_phase("done")
        warns = ["Запущен Claude. Перезапустите сессию в приложении или терминале."] if snapshot_processes().get("claude") else []
        return {"deleted": cid, "provider": self.id, "backup": str(bdir) if bdir is not None else None, "warns": warns}

    def backup_root_path(self) -> Path:
        return CL.BACKUPS_DIR

    def _backup_jsonl_file(self, bdir: Path, meta: dict) -> Path | None:
        fn = meta.get("file_name")
        if fn:
            cand = bdir / fn
            if os.path.isfile(win_long_path(cand)):
                return cand
        for p in bdir.glob("*.jsonl"):
            if os.path.isfile(win_long_path(p)):
                return p
        return None

    def _backup_title(self, bdir: Path, xtid: str, src: Path | None) -> str:
        ct = bdir / "custom-title.json"
        if os.path.isfile(win_long_path(ct)):
            try:
                t = json.loads(Path(win_long_path(ct)).read_text(encoding="utf-8")).get("customTitle")
                if t:
                    return str(t)
            except Exception:
                pass
        try:
            t = CL.session_title(xtid)
            if t and t != xtid:
                return t
        except Exception:
            pass
        if src is not None:
            ft, _ = CL._extract_title_from_file(src)
            if ft:
                return ft
        return xtid

    def list_backups_sync(self, cid: str | None = None) -> dict:
        broot = CL.BACKUPS_DIR
        groups: list[dict] = []
        total_count = 0
        total_bytes = 0
        if not broot.exists():
            return {"groups": groups, "root": str(broot), "total_count": 0, "total_bytes": 0}

        roots = [(cid, broot / cid)] if cid else [
            (p.name, p) for p in sorted(broot.iterdir(), reverse=True) if p.is_dir()
        ]
        for xtid, tdir in roots:
            if not tdir.exists() or not tdir.is_dir():
                continue
            rows = []
            group_title = ""
            for bdir in sorted(tdir.iterdir(), reverse=True):
                if not bdir.is_dir():
                    continue
                mf = bdir / "backup.meta.json"
                if not mf.exists():
                    continue
                meta = {}
                try:
                    meta = json.loads(mf.read_text(encoding="utf-8"))
                except Exception:
                    pass
                src = self._backup_jsonl_file(bdir, meta)
                sz = dir_size_bytes(bdir)
                total_count += 1
                total_bytes += sz
                if not group_title:
                    group_title = self._backup_title(bdir, xtid, src)
                rows.append({
                    "provider": self.id,
                    "cid": xtid,
                    "dir": str(bdir),
                    "name": bdir.name,
                    "size": sz,
                    "meta": meta,
                    "restorable": True,
                    "inspectable": src is not None,
                    "reason": "",
                })
            if rows:
                groups.append({
                    "provider": self.id,
                    "cid": xtid,
                    "title": group_title or xtid,
                    "backups": rows,
                })
        return {"groups": groups, "root": str(broot), "total_count": total_count, "total_bytes": total_bytes}

    def inspect_backup_sync(self, backup_dir: str, *, limit: int = 80, before=None, include_tools: bool = False) -> dict:
        bdir = Path(backup_dir)
        if not bdir.resolve().is_relative_to(CL.BACKUPS_DIR.resolve()) or not bdir.is_dir():
            raise PermissionError("Недопустимый путь резервной копии Claude.")
        meta = {}
        mf = bdir / "backup.meta.json"
        if mf.is_file():
            try:
                meta = json.loads(mf.read_text(encoding="utf-8"))
            except Exception:
                pass
        src = self._backup_jsonl_file(bdir, meta)
        if src is None:
            raise FileNotFoundError("Файл JSONL отсутствует в резервной копии Claude.")

        cid = str(meta.get("tid") or bdir.parent.name)
        flat: list[dict] = []
        with open(win_long_path(src), "rb") as fh:
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
                if not include_tools and k not in ("user", "assistant"):
                    continue
                flat.append({
                    "key": it.get("key"),
                    "uuid": it.get("uuid"),
                    "kind": k,
                    "subtype": it.get("subtype") or k,
                    "label": it.get("label") or LABEL.get(k, "Прочее"),
                    "text": it.get("text") or "",
                    "ts": it.get("ts") or "",
                    "tool": it.get("tool"),
                    "status": it.get("status"),
                    "in_db": True,
                    "actions": [],
                })

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
            "cid": cid,
            "backup_dir": str(bdir),
            "backup_name": bdir.name,
            "title": self._backup_title(bdir, cid, src),
            "readonly": True,
            "turns": [{"items": window}] if window else [],
            "page_msgs": msgs,
            "total_label": f"записей: {total_items}",
            "cursor": str(start) if start > 0 else None,
            "has_more": start > 0,
            "writable": False,
        }

    def restore_backup_sync(self, backup_dir: str, set_phase) -> dict:
        bdir = Path(backup_dir)
        if not bdir.resolve().is_relative_to(CL.BACKUPS_DIR.resolve()) or not bdir.exists():
            raise PermissionError("Недопустимый путь резервной копии Claude.")
        set_phase("restore")
        result = CL.restore_backup(bdir)
        set_phase("done")
        return result
