from datetime import datetime
import hashlib
import json
import os
import re
import shutil
import sqlite3
from pathlib import Path

import codex_history_tool as C
from codex_io import lock_rollouts


def revision(tid):
    h = hashlib.sha256()
    for path in C.thread_files(tid):
        st = path.stat()
        h.update(f"{path}|{st.st_size}|{st.st_mtime_ns}|{st.st_ctime_ns}".encode("utf-8"))
    return h.hexdigest()


def _sync(fh):
    fh.flush()
    os.fsync(fh.fileno())


def _sha256_file(path):
    digest = hashlib.sha256()
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def _write_json_sync(path, value):
    with path.open("w", encoding="utf-8") as f:
        json.dump(value, f)
        _sync(f)


def _copy_and_validate(src, backup):
    src.seek(0)
    digest = hashlib.sha256()
    first = src.readline()
    if not first:
        raise ValueError("Пустой rollout")
    meta = json.loads(first)
    numbered = "ordinal" in meta
    body = meta.get("payload") or {}
    start = 0
    if numbered:
        start = int(meta["ordinal"])
        if start != 0 and not (body.get("history_base") or body.get("forked_from_id")):
            raise ValueError("Недопустимая нумерация rollout")
    digest.update(first)
    backup.write(first)
    expected = start + 1
    ordinal_re = re.compile(rb'"ordinal"\s*:\s*(\d+)')
    for raw in src:
        if numbered:
            header = raw[:512].split(b'"payload"', 1)[0]
            match = ordinal_re.search(header)
            if not match or int(match.group(1)) != expected:
                raise ValueError(f"Нарушена нумерация rollout около записи {expected}; изменение отменено")
            expected += 1
        digest.update(raw)
        backup.write(raw)
    _sync(backup)
    return digest.hexdigest()


def _head_candidate(src, dst, offset):
    src.seek(0)
    metadata = src.readline()
    meta = json.loads(metadata)
    numbered = "ordinal" in meta
    body = meta.get("payload") or {}
    start = int(meta["ordinal"]) if numbered else 0
    if numbered and start != 0 and not (body.get("history_base") or body.get("forked_from_id")):
        raise ValueError("Неизвестная начальная нумерация rollout")
    dst.write(metadata)
    src.seek(offset)
    previous = None
    new_ordinal = start + 1
    for raw in src:
        if numbered:
            row = json.loads(raw)
            old_ordinal = row.get("ordinal")
            if not isinstance(old_ordinal, int) or (previous is not None and old_ordinal != previous + 1):
                raise ValueError("Нарушена нумерация rollout: обрезка отменена")
            previous = old_ordinal
            row["ordinal"] = new_ordinal
            new_ordinal += 1
            ending = b"\r\n" if raw.endswith(b"\r\n") else (b"\n" if raw.endswith(b"\n") else b"")
            raw = json.dumps(row, ensure_ascii=False).encode("utf-8") + ending
        dst.write(raw)
    _sync(dst)


def _edit_candidate(src, dst, offset, new_text):
    src.seek(offset)
    msg = json.loads(src.readline())
    pl = msg.get("payload") or {}
    info = C.extract_item_edit_info(msg)
    old = info["old"]
    if not new_text or not new_text.strip():
        raise ValueError("Пустой текст сообщения")
    mirror = info["mirror_events"] and bool(old.strip())

    duplicate_messages = 0
    if mirror:
        src.seek(0)
        for raw in src:
            if b'"response_item"' not in raw or b'"message"' not in raw:
                continue
            row = json.loads(raw)
            p = row.get("payload") or {}
            if row.get("type") == "response_item" and p.get("type") == "message":
                text = C._join_content_text(p.get("content"))
                duplicate_messages += text == old

    src.seek(0)
    changed = []
    while True:
        pos = src.tell()
        raw = src.readline()
        if not raw:
            break
        row = None
        if pos == offset:
            row = msg
            C.patch_response_item(pl, new_text)
            row["payload"] = pl
        elif mirror and b'"event_msg"' in raw and any(x in raw for x in (b'"item_completed"', b'"item_started"', b'"task_complete"', b'"agent_message"', b'"user_message"')):
            event_row = json.loads(raw)
            event = event_row.get("payload") or {}
            item = event.get("item") or {}
            item_id = item.get("id") if isinstance(item, dict) else None
            expected = "UserMessage" if pl.get("role") == "user" else "AgentMessage"
            relevant_item = isinstance(item, dict) and item.get("type") == expected
            relevant_event = event.get("type") in (("task_complete", "agent_message") if pl.get("role") == "assistant" else ("user_message",))
            if (relevant_item or relevant_event) and C._deep_replace_text(event_row, old, new_text):
                if duplicate_messages > 1 and (not item_id or item_id != pl.get("id")):
                    raise ValueError("Несколько сообщений имеют одинаковый текст; безопасно определить копию не удалось")
                row = event_row
        if row is not None:
            ending = b"\r\n" if raw.endswith(b"\r\n") else (b"\n" if raw.endswith(b"\n") else b"")
            raw = json.dumps(row, ensure_ascii=False).encode("utf-8") + ending
            changed.append(pos)
        dst.write(raw)
    _sync(dst)
    return {"old": old, "new": new_text, "lines_changed": changed}


def sandbox_cut(tid, key, expected_revision, mode="tail", phase=lambda _: None, new_text=None, allow_real=False, auto_backup=True):
    if not allow_real and C.CODEX_HOME.resolve().name.lower() not in ("sandbox", "codex_sandbox"):
        raise PermissionError("Потоковая запись пока разрешена только в песочнице")
    files = C.thread_files(tid)
    if len(files) != 1:
        raise ValueError("Потоковая обрезка поддерживает только один rollout-файл")
    if Path(tid).name != tid or tid in (".", ".."):
        raise ValueError("Недопустимый идентификатор треда")
    path = files[0]
    with lock_rollouts(files) as locks:
        con = sqlite3.connect(str(C.PROJECTION_DB), timeout=3)
        try:
            con.execute("BEGIN IMMEDIATE")
            if not expected_revision or revision(tid) != expected_revision:
                raise RuntimeError("История изменилась после предпросмотра")
            if mode not in ("head", "tail", "edit"):
                raise ValueError("Недопустимый режим обрезки")
            report = C.preview_cut_fast(tid, key, mode, delete_selected=mode == "tail") if mode != "edit" else {"thread": tid, "mode": "edit"}
            cut_at = path.stat().st_size - report["bytes"] if mode == "tail" else C._byte_key(files, key)[1]
            if mode == "tail" and cut_at <= 0:
                raise ValueError("Удаление всего файла треда не поддерживается")
            src = locks[path]
            bdir = None
            dest = None
            marker = None
            if auto_backup:
                phase("backup")
                ts = datetime.now().strftime("%Y%m%d-%H%M%S-%f")
                bdir = C.CODEX_HOME.parent / "codex_backups" / tid / (ts + "-stream")
                dest = bdir / "sessions" / path.relative_to(C.SESSIONS)
                required = path.stat().st_size * (1 if mode == "tail" else 2)
                required += C.PROJECTION_DB.stat().st_size + 64 * 1024 * 1024
                if shutil.disk_usage(C.CODEX_HOME.parent).free < required:
                    raise OSError("Недостаточно места для резервной копии и временного файла")
                dest.parent.mkdir(parents=True, exist_ok=False)
                with dest.open("xb") as backup:
                    checksum = _copy_and_validate(src, backup)
                if dest.stat().st_size != path.stat().st_size:
                    raise IOError("Резервная копия неполная")
                if _sha256_file(dest) != checksum:
                    raise IOError("Контрольная сумма резервной копии не совпадает")
                C._backup_projection(tid, bdir / "projection.sqlite")
                check = sqlite3.connect(str(bdir / "projection.sqlite"))
                try:
                    if check.execute("PRAGMA quick_check").fetchone()[0] != "ok":
                        raise IOError("Резервная копия проекции повреждена")
                finally:
                    check.close()
                projection_hash = _sha256_file(bdir / "projection.sqlite")
                _write_json_sync(bdir / "backup.meta.json", {"tid": tid, "created": ts, "files": 1, "tag": "stream", "sha256": checksum, "projection_sha256": projection_hash})
                marker = bdir / "transaction.json"
                _write_json_sync(marker, {"status": "prepared", "operation": mode})
                candidate = bdir / "candidate.tmp"
            else:
                candidate = path.with_name(f".{path.name}.{datetime.now().strftime('%H%M%S%f')}.k1tty.tmp")
            modified = False
            try:
                if mode in ("head", "edit"):
                    phase("edit" if mode == "edit" else "cut")
                    with candidate.open("xb") as dst:
                        if mode == "head":
                            _head_candidate(src, dst, cut_at)
                        else:
                            report.update(_edit_candidate(src, dst, cut_at, new_text))
                phase("edit" if mode == "edit" else "cut")
                modified = True
                if mode == "tail":
                    src.truncate(cut_at)
                else:
                    src.seek(0)
                    with candidate.open("rb") as prepared:
                        shutil.copyfileobj(prepared, src, length=1024 * 1024)
                    src.truncate(candidate.stat().st_size)
                _sync(src)
                counts = {}
                for table in C._PROJECTION_TABLES:
                    counts[table] = con.execute(f'DELETE FROM "{table}" WHERE thread_id=?', (tid,)).rowcount
                con.commit()
            except Exception as original:
                con.rollback()
                if modified and dest is not None and dest.exists():
                    try:
                        src.seek(0)
                        with dest.open("rb") as backup:
                            shutil.copyfileobj(backup, src, length=1024 * 1024)
                        src.truncate(dest.stat().st_size)
                        _sync(src)
                    except Exception as restore_error:
                        raise RuntimeError(f"Ошибка: {original}; восстановление не удалось: {restore_error}. Копия: {bdir}") from original
                raise
            finally:
                if candidate.exists():
                    try:
                        candidate.unlink()
                    except OSError:
                        report.setdefault("warns", []).append("Не удалось удалить временный файл.")
            if marker is not None:
                try:
                    _write_json_sync(marker, {"status": "complete", "operation": mode})
                except OSError:
                    report.setdefault("warns", []).append("Операция завершена, но не удалось обновить журнал резервной копии.")
            C._FILE_CACHE.clear()
            report.update(backup=str(bdir) if bdir is not None else None, projection=counts, leaks=None)
            return report
        finally:
            if con.in_transaction:
                con.rollback()
            con.close()


def sandbox_tail(tid, delete_key, expected_revision, phase=lambda _: None):
    return sandbox_cut(tid, delete_key, expected_revision, "tail", phase)


def sandbox_restore(bdir, phase=lambda _: None, allow_real=False):
    if not allow_real and C.CODEX_HOME.resolve().name.lower() not in ("sandbox", "codex_sandbox"):
        raise PermissionError("Потоковое восстановление пока разрешено только в песочнице")
    bdir = Path(bdir)
    meta = json.loads((bdir / "backup.meta.json").read_text(encoding="utf-8"))
    tid = meta["tid"]
    if Path(tid).name != tid or tid in (".", ".."):
        raise ValueError("Недопустимый идентификатор треда")
    sroot = bdir / "sessions"
    saved = list(sroot.rglob("rollout-*.jsonl"))
    snapshot = bdir / "projection.sqlite"
    if len(saved) != 1 or meta.get("files") != 1 or not snapshot.is_file() or not meta.get("sha256") or not meta.get("projection_sha256"):
        raise ValueError("Поддерживаются только полные потоковые копии с контрольной суммой")
    backup_path = saved[0]
    if C._file_meta(backup_path)[0] != tid:
        raise ValueError("Резервная копия относится к другому треду")
    target = C.SESSIONS / backup_path.relative_to(sroot)
    if not target.resolve().is_relative_to(C.SESSIONS.resolve()):
        raise ValueError("Недопустимый путь файла треда")
    if not target.is_file():
        raise ValueError("Текущий rollout не найден; автоматическое восстановление невозможно")
    with lock_rollouts([target, backup_path, snapshot]) as locks:
        src, saved_file = locks[target], locks[backup_path]
        saved_file.seek(0)
        digest = hashlib.sha256()
        while True:
            chunk = saved_file.read(1024 * 1024)
            if not chunk:
                break
            digest.update(chunk)
        if digest.hexdigest() != meta["sha256"]:
            raise ValueError("Контрольная сумма резервной копии не совпадает")
        snapshot_file = locks[snapshot]
        snapshot_file.seek(0)
        digest = hashlib.sha256()
        while True:
            chunk = snapshot_file.read(1024 * 1024)
            if not chunk:
                break
            digest.update(chunk)
        if digest.hexdigest() != meta["projection_sha256"]:
            raise ValueError("Контрольная сумма копии проекции не совпадает")
        con = sqlite3.connect(str(C.PROJECTION_DB), timeout=3)
        try:
            con.execute("ATTACH DATABASE ? AS saved", (str(snapshot),))
            if con.execute("PRAGMA saved.quick_check").fetchone()[0] != "ok":
                raise ValueError("Повреждена копия проекции")
            for table in C._PROJECTION_TABLES:
                current = [r[1] for r in con.execute(f'PRAGMA main.table_info("{table}")')]
                archived = [r[1] for r in con.execute(f'PRAGMA saved.table_info("{table}")')]
                if not current or current != archived or "thread_id" not in current:
                    raise ValueError(f"Не совпадает схема таблицы {table}")
            con.execute("BEGIN IMMEDIATE")
            phase("backup")
            ts = datetime.now().strftime("%Y%m%d-%H%M%S-%f")
            undo = C.CODEX_HOME.parent / "codex_backups" / tid / (ts + "-before-restore")
            undo_file = undo / "sessions" / target.relative_to(C.SESSIONS)
            required = target.stat().st_size + C.PROJECTION_DB.stat().st_size + 64 * 1024 * 1024
            if shutil.disk_usage(C.CODEX_HOME.parent).free < required:
                raise OSError("Недостаточно места для резервной копии текущего состояния")
            undo_file.parent.mkdir(parents=True, exist_ok=False)
            src.seek(0)
            checksum = hashlib.sha256()
            with undo_file.open("xb") as out:
                while True:
                    chunk = src.read(1024 * 1024)
                    if not chunk:
                        break
                    checksum.update(chunk)
                    out.write(chunk)
                _sync(out)
            C._backup_projection(tid, undo / "projection.sqlite")
            undo_projection_hash = _sha256_file(undo / "projection.sqlite")
            _write_json_sync(undo / "backup.meta.json", {"tid": tid, "created": ts, "files": 1, "tag": "before-restore", "sha256": checksum.hexdigest(), "projection_sha256": undo_projection_hash})
            marker = undo / "transaction.json"
            _write_json_sync(marker, {"status": "prepared", "operation": "restore"})
            modified = False
            try:
                phase("restore")
                modified = True
                src.seek(0)
                saved_file.seek(0)
                shutil.copyfileobj(saved_file, src, length=1024 * 1024)
                src.truncate(backup_path.stat().st_size)
                _sync(src)
                for table in C._PROJECTION_TABLES:
                    con.execute(f'DELETE FROM main."{table}" WHERE thread_id=?', (tid,))
                    con.execute(f'INSERT INTO main."{table}" SELECT * FROM saved."{table}" WHERE thread_id=?', (tid,))
                con.commit()
            except Exception as original:
                con.rollback()
                if modified:
                    try:
                        src.seek(0)
                        with undo_file.open("rb") as previous:
                            shutil.copyfileobj(previous, src, length=1024 * 1024)
                        src.truncate(undo_file.stat().st_size)
                        _sync(src)
                    except Exception as restore_error:
                        raise RuntimeError(f"Ошибка: {original}; возврат не удался: {restore_error}. Копия: {undo}") from original
                raise
            warns = []
            try:
                _write_json_sync(marker, {"status": "complete", "operation": "restore"})
            except OSError:
                warns.append("Не удалось обновить журнал резервной копии.")
            C._FILE_CACHE.clear()
            return {"restored": str(bdir), "backup": str(undo), "warns": warns}
        finally:
            if con.in_transaction:
                con.rollback()
            con.close()
