from __future__ import annotations

import hashlib
import json
import os
import shutil
import sqlite3
from datetime import datetime
from pathlib import Path

import codex_history_tool as C
import codex_stream
from codex_io import lock_rollouts
from process_monitor import format_running, snapshot_processes
from providers import BaseProvider, dir_size_bytes

C.codex_running = lambda: snapshot_processes().get("codex", [])

LABEL = {
    "user": "Вы", "assistant": "Codex", "tool": "Инструмент",
    "compact": "Компакция", "system": "Система", "error": "Ошибка", "other": "Прочее",
}


def _test_mode() -> bool:
    return C.CODEX_HOME.resolve().name.lower() in ("sandbox", "codex_sandbox")


def _stream_supported(tid: str) -> bool:
    return len(C.thread_files(tid)) == 1


def _full_revision(tid: str) -> str:
    digest = hashlib.sha256()
    for path in C.thread_files(tid):
        digest.update(str(path.relative_to(C.SESSIONS)).encode("utf-8"))
        with path.open("rb") as f:
            for chunk in iter(lambda: f.read(1024 * 1024), b""):
                digest.update(chunk)
    return digest.hexdigest()


def _view_revision(tid: str) -> str:
    digest = hashlib.sha256()
    for path in C.thread_files(tid):
        st = path.stat()
        digest.update(f"{path}|{st.st_size}|{st.st_mtime_ns}|{st.st_ctime_ns}".encode("utf-8"))
    return digest.hexdigest()


class CodexProvider(BaseProvider):
    id = "codex"
    name = "Codex"
    short = "CX"

    def state_sync(self, running_map: dict[str, list[str]]) -> dict:
        running = running_map.get("codex", [])
        avail = C.SESSIONS.exists()
        writable = bool(avail)
        if not avail:
            note = f"Каталог сессий не найден: {C.SESSIONS}"
        elif _test_mode():
            note = "ТЕСТ-РЕЖИМ: изменения пишутся в песочницу."
        elif running:
            note = f"Запущено: {format_running(running)}. Закройте тред в Codex перед правкой и откройте заново после."
        else:
            note = "Готов к потоковой правке и обрезке однофайловых тредов с проверкой контрольных сумм."
        return {
            "id": self.id,
            "name": self.name,
            "short": self.short,
            "available": avail,
            "writable": writable,
            "blocked": False,
            "running": running,
            "test": _test_mode(),
            "appdata": str(C.CODEX_HOME),
            "backups_root": str(C.CODEX_HOME.parent / "codex_backups"),
            "note": note,
        }

    def list_convs_sync(self, refresh: bool = False) -> list[dict]:
        if refresh:
            C._FILE_CACHE.clear()
            C._LIST_CACHE.clear()
        if not C.SESSIONS.exists():
            return []
        out: list[dict] = []
        for t in C.list_threads():
            single = t["files"] == 1
            out.append({
                "provider": self.id,
                "id": t["id"],
                "title": t["title"],
                "subtitle": f"rollout-файлов: {t['files']}" + ("" if single else " (только чтение)"),
                "badge": f"{t['files']} ф.",
                "size": t["size"],
                "mtime": t["mtime"].strftime("%d.%m.%Y %H:%M"),
                "mtime_ts": t["mtime"].timestamp(),
                "writable": _test_mode() or single,
            })
        return out

    def probe_conv_lock_sync(self, cid: str | None = None) -> tuple[bool, str]:
        if cid:
            files = C.thread_files(cid)
            if files:
                try:
                    with lock_rollouts(files):
                        pass
                except OSError:
                    return True, "Файл диалога занят Codex. Закройте этот тред в Codex или закройте приложение."
        if C.PROJECTION_DB.is_file():
            ok, _msg = C.db_write_probe(C.PROJECTION_DB)
            if not ok:
                return True, "База проекций Codex занята. Закройте Codex и повторите."
        return False, ""

    def get_entries_sync(self, cid: str, *, limit: int = 80, before=None, include_tools: bool = False) -> dict:
        lim = min(150, max(1, int(limit)))
        items, cursor = C.read_history_page(cid, limit=lim, before=before, include_tools=include_tools)
        writable = _test_mode() or _stream_supported(cid)
        locked, lock_reason = self.probe_conv_lock_sync(cid)
        turns: list[dict] = []
        total_msgs = 0

        for t in C.turns_from_history(items):
            entries = []
            for it in t["items"]:
                k = it.get("kind", "other")
                actions: list[str] = []
                if writable:
                    if k in ("user", "assistant", "system", "tool") and it.get("subtype") != "aborted":
                        actions.append("edit")
                    if k == "user":
                        actions.append("cut_head")
                    actions.append("cut_tail")

                label = LABEL.get(k, "Прочее")
                if k in ("tool", "error", "system") and it.get("label"):
                    label = it.get("label")

                entry = {
                    "key": it.get("key"),
                    "kind": k,
                    "subtype": "thinking" if it.get("label") == "reasoning" else (it.get("subtype") or k),
                    "label": label,
                    "text": it.get("text") or "",
                    "ts": it.get("ts") or "",
                    "tool": it.get("label"),
                    "code": it.get("code"),
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
            "writable": writable,
            "file_locked": locked,
            "lock_reason": lock_reason,
            "blocked_info": C.detect_thread_block(cid),
            "block_presets": C.CODEX_BLOCK_PRESETS,
        }

    def edit_preview_sync(self, cid: str, key: str) -> dict:
        streaming = _stream_supported(cid)
        writable = _test_mode() or streaming
        rev_fn = codex_stream.revision if streaming else (_full_revision if writable else _view_revision)
        rev = rev_fn(cid)
        r = C.edit_message(cid, key, "", preview_only=True)
        if rev_fn(cid) != rev:
            raise RuntimeError("История Codex изменилась во время предпросмотра. Обновите тред.")
        kind_label = r.get("label") or (
            "Сообщение пользователя" if r.get("kind") == "user" else
            "Ответ Codex" if r.get("kind") == "assistant" else
            "Служебная запись"
        )
        return {
            "text": r["old"],
            "kind": kind_label,
            "hint": (
                "Текст обновится в rollout-файле с удержанием блокировок, проекция треда будет сброшена. "
                "Перед записью создаётся проверенная резервная копия."
                if writable else
                "Правка недоступна: нужен однофайловый rollout и поддерживаемая платформа."
            ),
            "revision": rev if writable else None,
            "writable": writable,
        }

    def apply_edit_sync(self, cid: str, key: str, text: str, revision: str | None, set_phase, *, auto_backup: bool = True) -> dict:
        if not text or not text.strip():
            raise ValueError("Пустой текст, сохранение не выполнено.")
        if _stream_supported(cid):
            rep = codex_stream.sandbox_cut(
                cid, key, revision, "edit", set_phase, new_text=text, allow_real=not _test_mode(), auto_backup=auto_backup
            )
            rep["warns"] = (
                ["ТЕСТ-РЕЖИМ: изменения пишутся в песочницу."]
                if _test_mode() else
                ["После изменения заново откройте тред в Codex; перестроение проекции может занять время."]
            ) + rep.get("warns", [])
            set_phase("done")
            return rep

        if not revision or _full_revision(cid) != revision:
            raise RuntimeError("История изменилась после предпросмотра.")
        bdir = None
        if auto_backup:
            set_phase("backup")
            bdir = C.full_backup(cid, tag="webedit")
        set_phase("edit")
        try:
            rep = C.edit_message(cid, key, text, preview_only=False)
        except Exception as orig:
            if bdir is not None:
                try:
                    C.restore_backup(bdir)
                except Exception as re:
                    raise RuntimeError(f"Ошибка: {orig}; восстановление не удалось: {re}. Копия: {bdir}") from orig
            raise
        rep["backup"] = str(bdir) if bdir is not None else None
        rep["warns"] = ["ТЕСТ-РЕЖИМ: изменения пишутся в песочницу."] if _test_mode() else []
        set_phase("done")
        return rep

    def preview_cut_sync(self, cid: str, key: str, mode: str) -> dict:
        streaming = _stream_supported(cid)
        writable = _test_mode() or streaming
        rev_fn = codex_stream.revision if streaming else (_full_revision if writable else _view_revision)
        rev = rev_fn(cid)
        rep = C.preview_cut_fast(cid, key, mode, delete_selected=(mode == "tail"))
        if rev_fn(cid) != rev:
            raise RuntimeError("История Codex изменилась во время предпросмотра. Обновите тред.")
        wd = rep.get("will_delete") or {}
        norm = {
            "provider": self.id,
            "mode": mode,
            "steps_count": wd.get("steps_preview_count"),
            "raw_count": rep.get("rollout_lines"),
            "raw_label": "строк rollout",
            "bytes": rep.get("bytes", 0),
            "extra_label": f"файлов целиком: {rep.get('rollout_files', 0)}",
            "count_incomplete": bool(rep.get("count_incomplete")),
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
            "revision": rev if writable else None,
            "writable": writable,
            "hint": (
                "Будут удалены записи в rollout-файле и сброшена проекция десктоп-приложения. "
                "Перед изменением создаётся SHA256-проверенная копия rollout и проекции."
                if writable else
                "Изменение многосегментного треда вне песочницы недоступно (только чтение)."
            ),
        }

    def apply_cut_sync(self, cid: str, key: str, mode: str, revision: str | None, set_phase, *, auto_backup: bool = True) -> dict:
        if _stream_supported(cid):
            rep = codex_stream.sandbox_cut(
                cid, key, revision, mode, set_phase, allow_real=not _test_mode(), auto_backup=auto_backup
            )
            rep["warns"] = (
                ["ТЕСТ-РЕЖИМ: изменения пишутся в песочницу."]
                if _test_mode() else
                ["После изменения заново откройте тред в Codex; перестроение проекции может занять время."]
            ) + rep.get("warns", [])
            set_phase("done")
            return rep

        if not revision or _full_revision(cid) != revision:
            raise RuntimeError("История изменилась после предпросмотра.")
        target_key = C.message_before_key(cid, key) if mode == "tail" else C.resolve_key(cid, key)
        bdir = None
        if auto_backup:
            set_phase("backup")
            bdir = C.full_backup(cid, tag="web")
        set_phase("cut")
        try:
            rep = C.cut(cid, target_key, preview_only=False, mode=mode)
        except Exception as orig:
            if bdir is not None:
                try:
                    C.restore_backup(bdir)
                except Exception as re:
                    raise RuntimeError(f"Ошибка: {orig}; восстановление не удалось: {re}. Копия: {bdir}") from orig
            raise
        rep["backup"] = str(bdir) if bdir is not None else None
        rep["warns"] = ["ТЕСТ-РЕЖИМ: изменения пишутся в песочницу."] if _test_mode() else []
        set_phase("done")
        return rep

    def delete_conv_sync(self, cid: str, set_phase, *, auto_backup: bool = True) -> dict:
        if Path(cid).name != cid or cid in (".", ".."):
            raise ValueError("Недопустимый ID треда.")
        files = C.thread_files(cid)
        if not files:
            raise FileNotFoundError(f"Тред {cid} не найден.")

        saved_index_lines: list[str] = []
        if C.SESSION_INDEX.is_file():
            raw_idx = C.SESSION_INDEX.read_text(encoding="utf-8", errors="replace").splitlines()
            file_stems = {p.stem for p in files}
            kept_idx = []
            for ln in raw_idx:
                matched = False
                try:
                    o = json.loads(ln)
                    oid = o.get("id", "")
                    if oid == cid or any(oid and oid in s for s in file_stems):
                        matched = True
                except Exception:
                    pass
                if matched:
                    saved_index_lines.append(ln)
                else:
                    kept_idx.append(ln)
        else:
            kept_idx = None

        bdir = None
        if auto_backup:
            set_phase("backup")
            ts = datetime.now().strftime("%Y%m%d-%H%M%S-%f")
            bdir = C.CODEX_HOME.parent / "codex_backups" / cid / (ts + "-delete")
            bdir.mkdir(parents=True, exist_ok=False)

            file_hashes: dict[str, str] = {}
            for p in files:
                rel = str(p.relative_to(C.SESSIONS))
                dst = bdir / "sessions" / rel
                dst.parent.mkdir(parents=True, exist_ok=True)
                shutil.copy2(p, dst)
                file_hashes[rel] = codex_stream._sha256_file(dst)

            proj_hash = None
            if C.PROJECTION_DB.is_file():
                C._backup_projection(cid, bdir / "projection.sqlite")
                proj_hash = codex_stream._sha256_file(bdir / "projection.sqlite")

            meta = {
                "tid": cid,
                "created": ts,
                "files": len(files),
                "tag": "delete",
                "sha256": next(iter(file_hashes.values())) if len(file_hashes) == 1 else "",
                "file_hashes": file_hashes,
                "projection_sha256": proj_hash or "",
                "session_index_lines": saved_index_lines,
            }
            (bdir / "backup.meta.json").write_text(json.dumps(meta, ensure_ascii=False, indent=2), encoding="utf-8")

        set_phase("cut")
        with lock_rollouts(files):
            pass
        if C.PROJECTION_DB.is_file():
            con = sqlite3.connect(str(C.PROJECTION_DB), timeout=5)
            try:
                con.execute("BEGIN IMMEDIATE")
                for table in C._PROJECTION_TABLES:
                    con.execute(f'DELETE FROM "{table}" WHERE thread_id=?', (cid,))
                con.commit()
            except Exception:
                con.rollback()
                raise
            finally:
                con.close()

        for p in files:
            if p.exists():
                p.unlink()

        if kept_idx is not None and saved_index_lines:
            tmp = C.SESSION_INDEX.with_suffix(".jsonl.tmp")
            tmp.write_text("\n".join(kept_idx) + ("\n" if kept_idx else ""), encoding="utf-8")
            tmp.replace(C.SESSION_INDEX)

        C._FILE_CACHE.clear()
        C._LIST_CACHE.clear()
        set_phase("done")
        warns = ["Обнаружен запущенный Codex. Перезапустите приложение для обновления списка."] if snapshot_processes().get("codex") else []
        return {"deleted": cid, "provider": self.id, "backup": str(bdir) if bdir is not None else None, "files_removed": len(files), "warns": warns}

    def backup_root_path(self) -> Path:
        return C.CODEX_HOME.parent / "codex_backups"

    def _backup_rollouts(self, bdir: Path) -> list[Path]:
        sroot = bdir / "sessions"
        files = sorted(sroot.rglob("rollout-*.jsonl")) if sroot.exists() else []
        if not files:
            files = sorted(bdir.glob("rollout*.jsonl"))
        return files

    def list_backups_sync(self, cid: str | None = None) -> dict:
        broot = self.backup_root_path()
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
            for bdir in sorted(tdir.iterdir(), reverse=True):
                if not bdir.is_dir():
                    continue
                mf = bdir / "backup.meta.json"
                has_meta = mf.is_file()
                meta = {}
                if has_meta:
                    try:
                        meta = json.loads(mf.read_text(encoding="utf-8"))
                    except Exception:
                        pass
                restorable = bool(
                    has_meta and (
                        meta.get("tag") == "delete"
                        or bool(meta.get("tail_backup"))
                        or meta.get("tag") in ("k1tty_unlock", "k1tty_lock")
                        or (meta.get("sha256") and (meta.get("projection_sha256") or not (bdir / "projection.sqlite").is_file()))
                        or _test_mode()
                    )
                )
                sz = dir_size_bytes(bdir)
                total_count += 1
                total_bytes += sz
                inspectable = bool(self._backup_rollouts(bdir))
                rows.append({
                    "provider": self.id,
                    "cid": xtid,
                    "dir": str(bdir),
                    "name": bdir.name,
                    "size": sz,
                    "meta": meta,
                    "restorable": restorable,
                    "inspectable": inspectable,
                    "reason": "" if restorable else ("Копия без backup.meta.json" if not has_meta else "Старая копия без контрольных сумм требует ручного восстановления"),
                })
            if rows:
                try:
                    title = C.thread_title(xtid)
                except Exception:
                    title = xtid
                groups.append({
                    "provider": self.id,
                    "cid": xtid,
                    "title": title,
                    "backups": rows,
                })
        return {"groups": groups, "root": str(broot), "total_count": total_count, "total_bytes": total_bytes}

    def inspect_backup_sync(self, backup_dir: str, *, limit: int = 80, before=None, include_tools: bool = False) -> dict:
        bdir = Path(backup_dir)
        broot = self.backup_root_path().resolve()
        if not bdir.resolve().is_relative_to(broot) or not bdir.is_dir():
            raise PermissionError("Недопустимый путь резервной копии Codex.")
        files = self._backup_rollouts(bdir)
        if not files:
            raise FileNotFoundError("В этой резервной копии нет полного файла rollout для просмотра.")

        tid = bdir.parent.name
        mf = bdir / "backup.meta.json"
        if mf.is_file():
            try:
                meta = json.loads(mf.read_text(encoding="utf-8"))
                if meta.get("tid"):
                    tid = str(meta["tid"])
            except Exception:
                pass

        lim = min(150, max(1, int(limit)))
        last, end = len(files) - 1, None
        if before:
            fi_s, loc = str(before).split(":", 1)
            if not loc.startswith("b"):
                raise ValueError("Некорректный курсор истории")
            last, end = int(fi_s), int(loc[1:])
            if last < 0 or last >= len(files):
                raise ValueError("Курсор больше не существует")

        items, messages = [], 0
        cursor = None
        for fi in range(last, -1, -1):
            for offset, raw in C._reverse_lines(files[fi], end if fi == last else None):
                if not include_tools:
                    is_msg = b'"response_item"' in raw and b'"message"' in raw
                    is_err = b'"event_msg"' in raw and b'"task_complete"' in raw and b'"error"' in raw
                    if not (is_msg or is_err):
                        continue
                try:
                    item = C._history_item(json.loads(raw), fi, f"b{offset}")
                except (ValueError, UnicodeDecodeError):
                    continue
                if item is None:
                    continue
                if not include_tools and item["kind"] not in ("user", "assistant", "error"):
                    continue
                items.append(item)
                messages += item["kind"] in ("user", "assistant")
                if messages >= lim or len(items) >= 2000:
                    cursor = item["key"] if fi > 0 or offset > 0 else None
                    break
            if cursor is not None or messages >= lim or len(items) >= 2000:
                break
        items.reverse()

        turns: list[dict] = []
        total_msgs = 0
        for t in C.turns_from_history(items):
            entries = []
            for it in t["items"]:
                k = it.get("kind", "other")
                label = LABEL.get(k, "Прочее")
                if k in ("tool", "error", "system") and it.get("label"):
                    label = it.get("label")
                entries.append({
                    "key": it.get("key"),
                    "kind": k,
                    "subtype": "thinking" if it.get("label") == "reasoning" else (it.get("subtype") or k),
                    "label": label,
                    "text": it.get("text") or "",
                    "ts": it.get("ts") or "",
                    "tool": it.get("label"),
                    "code": it.get("code"),
                    "status": it.get("status"),
                    "in_db": True,
                    "actions": [],
                })
                if k in ("user", "assistant"):
                    total_msgs += 1
            if entries:
                turns.append({"items": entries})

        try:
            title = C.thread_title(tid)
        except Exception:
            title = tid

        return {
            "provider": self.id,
            "cid": tid,
            "backup_dir": str(bdir),
            "backup_name": bdir.name,
            "title": title,
            "readonly": True,
            "turns": turns,
            "page_msgs": total_msgs,
            "total_label": f"загружено: {total_msgs}",
            "cursor": cursor,
            "has_more": cursor is not None,
            "writable": False,
        }

    def restore_backup_sync(self, backup_dir: str, set_phase) -> dict:
        bdir = Path(backup_dir)
        broot = (C.CODEX_HOME.parent / "codex_backups").resolve()
        if not bdir.resolve().is_relative_to(broot) or not bdir.exists():
            raise PermissionError("Недопустимый путь резервной копии Codex.")
        meta = json.loads((bdir / "backup.meta.json").read_text(encoding="utf-8"))
        tid = meta["tid"]
        sroot = bdir / "sessions"
        saved = list(sroot.rglob("rollout-*.jsonl")) if sroot.exists() else []
        any_missing = any(not (C.SESSIONS / p.relative_to(sroot)).exists() for p in saved)

        if meta.get("tag") == "delete" or any_missing:
            set_phase("restore")
            file_hashes = meta.get("file_hashes") or {}
            for p in saved:
                rel = str(p.relative_to(sroot))
                exp_hash = file_hashes.get(rel) or (meta.get("sha256") if len(saved) == 1 else "")
                if exp_hash and codex_stream._sha256_file(p) != exp_hash:
                    raise ValueError(f"Контрольная сумма {rel} не совпадает.")
                dst = C.SESSIONS / p.relative_to(sroot)
                if not dst.resolve().is_relative_to(C.SESSIONS.resolve()):
                    raise ValueError("Недопустимый путь назначения.")
                dst.parent.mkdir(parents=True, exist_ok=True)
                shutil.copy2(p, dst)

            snap = bdir / "projection.sqlite"
            if snap.is_file() and C.PROJECTION_DB.is_file():
                con = sqlite3.connect(str(C.PROJECTION_DB), timeout=5)
                try:
                    con.execute("ATTACH DATABASE ? AS saved", (str(snap),))
                    con.execute("BEGIN IMMEDIATE")
                    for table in C._PROJECTION_TABLES:
                        con.execute(f'DELETE FROM main."{table}" WHERE thread_id=?', (tid,))
                        con.execute(f'INSERT INTO main."{table}" SELECT * FROM saved."{table}" WHERE thread_id=?', (tid,))
                    con.commit()
                except Exception:
                    con.rollback()
                    raise
                finally:
                    con.close()

            idx_lines = meta.get("session_index_lines") or []
            if idx_lines:
                existing = C.SESSION_INDEX.read_text(encoding="utf-8", errors="replace").splitlines() if C.SESSION_INDEX.is_file() else []
                existing_set = set(existing)
                for ln in idx_lines:
                    if ln not in existing_set:
                        existing.append(ln)
                tmp = C.SESSION_INDEX.with_suffix(".jsonl.tmp")
                tmp.write_text("\n".join(existing) + "\n", encoding="utf-8")
                tmp.replace(C.SESSION_INDEX)

            C._FILE_CACHE.clear()
            C._LIST_CACHE.clear()
            set_phase("done")
            return {"restored": str(bdir), "provider": self.id}

        if meta.get("tail_backup"):
            set_phase("restore")
            C.restore_tail_backup(bdir, allow_real=not _test_mode())
            set_phase("done")
            return {"restored": str(bdir), "provider": self.id}

        if meta.get("sha256") and (meta.get("projection_sha256") or not (bdir / "projection.sqlite").is_file()):
            result = codex_stream.sandbox_restore(bdir, set_phase, allow_real=not _test_mode())
        elif meta.get("tag") in ("k1tty_unlock", "k1tty_lock"):
            set_phase("restore")
            C.restore_lock_full_backup(bdir, allow_real=not _test_mode())
            result = {"restored": str(bdir)}
        elif _test_mode():
            set_phase("restore")
            C.restore_backup(bdir)
            result = {"restored": str(bdir)}
        else:
            raise ValueError("Старая копия без контрольных сумм требует ручного восстановления.")
        set_phase("done")
        return result

    def unlock_chat_sync(self, cid: str, set_phase, *, auto_backup: bool = True) -> dict:
        rep = C.unlock_thread(cid, set_phase=set_phase, allow_real=not _test_mode(), auto_backup=auto_backup)
        rep["warns"] = (
            ["ТЕСТ-РЕЖИМ: изменения сохраняются в песочницу."]
            if _test_mode() else
            ["Чат успешно разблокирован K1tty Tools. Обновите диалог в приложении Codex."]
        )
        return rep

    def lock_chat_sync(self, cid: str, message: str, code: str, set_phase, *, auto_backup: bool = True) -> dict:
        rep = C.lock_thread(cid, message=message, code=code, set_phase=set_phase, allow_real=not _test_mode(), auto_backup=auto_backup)
        rep["warns"] = (
            ["ТЕСТ-РЕЖИМ: изменения сохраняются в песочницу."]
            if _test_mode() else
            ["Чат заблокирован. Вы можете разбанить его в любой момент."]
        )
        return rep
