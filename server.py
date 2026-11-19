from __future__ import annotations

import asyncio
import json
import os
import sys
import time
import traceback
from urllib.parse import quote
import uuid
import webbrowser
from pathlib import Path

if sys.version_info < (3, 14):
    sys.exit(
        f"K1tty Tools requires Python 3.14 or newer (found {sys.version.split()[0]}).\n"
        "Install it from https://www.python.org/downloads/ or run with uv: "
        "uv run --python 3.14 --with-requirements requirements.txt python -X utf8 server.py"
    )

try:
    from aiohttp import WSMsgType, web
except ImportError:
    sys.exit("Missing dependencies. Install them with: pip install -r requirements.txt")

HERE = Path(__file__).resolve().parent
if str(HERE) not in sys.path:
    sys.path.insert(0, str(HERE))
if str(HERE / "engines") not in sys.path:
    sys.path.insert(0, str(HERE / "engines"))

import ag_history_tool as AG_MOD
import chat_converter as CC
import k1tty_pack as KP
import legacy_layout
from process_monitor import snapshot_processes
from providers import BaseProvider, build_providers

STATIC_DIR = HERE / "static"
SETTINGS_FILE = HERE / "k1tty_settings.json"


def _load_settings(path: Path) -> dict:
    defaults = {"auto_backup": True}
    try:
        if path.is_file():
            raw = json.loads(path.read_text(encoding="utf-8"))
            if isinstance(raw, dict):
                if "auto_backup" in raw:
                    defaults["auto_backup"] = bool(raw["auto_backup"])
    except Exception:
        pass
    return defaults


def _save_settings(path: Path, data: dict):
    tmp = path.with_suffix(".json.tmp")
    tmp.write_text(json.dumps(data, ensure_ascii=False, indent=2), encoding="utf-8")
    tmp.replace(path)


class AppRunningError(RuntimeError):
    pass


def _is_expected_error(e: Exception) -> bool:
    if isinstance(e, (AppRunningError, PermissionError, ValueError, FileNotFoundError, KP.K1ttyPackError)):
        return True
    if isinstance(e, OSError) and getattr(e, "errno", None) in (13, 32):
        return True
    return False


def _format_client_error(e: Exception) -> str:
    if isinstance(e, OSError) and getattr(e, "errno", None) in (13, 32):
        return "Файл занят запущенным приложением. Закройте приложение и повторите."
    if _is_expected_error(e):
        return str(e)
    return f"{e.__class__.__name__}: {e}"


class K1ttyServer:
    def __init__(self, providers: dict[str, BaseProvider] | None = None, *, check_running_guard: bool = True, settings_path: Path | None = None):
        self.providers = providers or build_providers()
        self.check_running_guard = check_running_guard
        self.settings_path = settings_path or SETTINGS_FILE
        self.settings = _load_settings(self.settings_path)
        self.clients: dict[str, web.WebSocketResponse] = {}
        self.client_views: dict[str, tuple[str, str]] = {}
        self._last_conv_locks: dict[tuple[str, str], tuple[bool, str]] = {}
        self.jobs: dict[str, dict] = {}
        self.tasks: set[asyncio.Task] = set()
        self.downloads: dict[str, tuple[bytes, str, float]] = {}
        self.boot_id: str = uuid.uuid4().hex[:12]
        self._last_procs_sig: dict[str, tuple[str, ...]] | None = None
        self._watcher_task: asyncio.Task | None = None

    def auto_backup_enabled(self) -> bool:
        return bool(self.settings.get("auto_backup", True))

    def _assert_writable(self, prov: BaseProvider, cid: str | None = None):
        if not self.check_running_guard:
            return
        locked, reason = prov.probe_conv_lock_sync(cid)
        if locked:
            raise AppRunningError(reason or f"Файл занят {prov.name}. Закройте диалог или приложение и повторите.")

    def _get_provider(self, name: str | None) -> BaseProvider:
        if not name or name not in self.providers:
            raise ValueError(f"Неизвестный провайдер: {name!r}")
        return self.providers[name]

    def _check_origin(self, request: web.Request):
        origin = request.headers.get("Origin")
        if origin and origin not in (f"http://{request.host}", f"https://{request.host}"):
            raise web.HTTPForbidden(text="Forbidden Origin")

    def _prune_downloads(self):
        now = time.time()
        for k in list(self.downloads.keys()):
            if now - self.downloads[k][2] > 300:
                self.downloads.pop(k, None)
        while len(self.downloads) > 10:
            oldest = min(self.downloads.items(), key=lambda kv: kv[1][2])[0]
            self.downloads.pop(oldest, None)

    async def broadcast(self, msg: dict):
        if not self.clients:
            return
        payload = json.dumps(msg, ensure_ascii=False)
        dead: list[str] = []

        async def _send(k: str, ws: web.WebSocketResponse):
            try:
                if ws.closed:
                    dead.append(k)
                else:
                    await ws.send_str(payload)
            except Exception:
                dead.append(k)

        await asyncio.gather(*(_send(k, w) for k, w in list(self.clients.items())), return_exceptions=True)
        for k in dead:
            self.clients.pop(k, None)

    def start_job(self, provider: BaseProvider, name: str, work_sync) -> str:
        loop = asyncio.get_running_loop()
        job_id = uuid.uuid4().hex[:12]
        job = {
            "id": job_id,
            "provider": provider.id,
            "name": name,
            "state": "running",
            "phase": "start",
            "result": None,
            "error": None,
        }
        for old_id in list(self.jobs.keys())[:-40]:
            if self.jobs[old_id]["state"] != "running":
                self.jobs.pop(old_id, None)
        self.jobs[job_id] = job

        def set_phase(ph: str):
            job["phase"] = ph
            snap = dict(job, result=None, error=None)
            loop.call_soon_threadsafe(lambda s=snap: self._spawn(self.broadcast({"type": "job", "job": s})))

        async def _runner():
            t0 = time.perf_counter()
            try:
                await self.broadcast({"type": "job", "job": dict(job, result=None, error=None)})
                async with provider.write_lock:
                    result = await asyncio.to_thread(work_sync, set_phase)
                job["state"] = "done"
                job["phase"] = "done"
                job["result"] = result
            except Exception as e:
                job["state"] = "error"
                job["error"] = _format_client_error(e)
                if not _is_expected_error(e):
                    traceback.print_exc()
            job["ms"] = int((time.perf_counter() - t0) * 1000)
            await self.broadcast({"type": "job", "job": job})

        self._spawn(_runner())
        return job_id

    def start_multi_job(self, providers: list[BaseProvider], name: str, work_async) -> str:
        job_id = uuid.uuid4().hex[:12]
        job = {
            "id": job_id,
            "provider": providers[0].id if len(providers) == 1 else "all",
            "name": name,
            "state": "running",
            "phase": "start",
            "result": None,
            "error": None,
        }
        for old_id in list(self.jobs.keys())[:-40]:
            if self.jobs[old_id]["state"] != "running":
                self.jobs.pop(old_id, None)
        self.jobs[job_id] = job

        async def set_phase(ph: str):
            job["phase"] = ph
            await self.broadcast({"type": "job", "job": dict(job, result=None, error=None)})

        async def _runner():
            t0 = time.perf_counter()
            try:
                await self.broadcast({"type": "job", "job": dict(job, result=None, error=None)})
                result = await work_async(set_phase)
                job["state"] = "done"
                job["phase"] = "done"
                job["result"] = result
            except Exception as e:
                job["state"] = "error"
                job["error"] = _format_client_error(e)
                if not _is_expected_error(e):
                    traceback.print_exc()
            job["ms"] = int((time.perf_counter() - t0) * 1000)
            await self.broadcast({"type": "job", "job": job})

        self._spawn(_runner())
        return job_id

    def _spawn(self, coro) -> asyncio.Task:
        t = asyncio.create_task(coro)
        self.tasks.add(t)
        t.add_done_callback(self.tasks.discard)
        return t

    async def handle_rpc(self, op: str, provider_id: str | None, d: dict, client_id: str | None = None) -> dict:
        if op == "ping":
            return {"pong": True, "ts": time.time()}

        if op == "set_view":
            if client_id:
                cid = d.get("cid")
                if provider_id and provider_id in self.providers and cid:
                    self.client_views[client_id] = (provider_id, str(cid))
                else:
                    self.client_views.pop(client_id, None)
            return {"ok": True}

        if op == "get_settings":
            return {"settings": dict(self.settings)}

        if op == "set_settings":
            if "auto_backup" in d:
                self.settings["auto_backup"] = bool(d["auto_backup"])
            await asyncio.to_thread(_save_settings, self.settings_path, dict(self.settings))
            snap = dict(self.settings)
            await self.broadcast({"type": "settings", "settings": snap})
            return {"settings": snap}

        if op == "state":
            force = bool(d.get("force"))
            running_map = await asyncio.to_thread(snapshot_processes, AG_MOD.DAEMON_DIR, force)
            self._last_procs_sig = {pid: tuple(sorted(running_map.get(pid, []))) for pid in self.providers}
            states = await asyncio.gather(
                *(asyncio.to_thread(p.state_sync, running_map) for p in self.providers.values())
            )
            return {"providers": {s["id"]: s for s in states}, "settings": dict(self.settings)}

        if op == "convs":
            refresh = bool(d.get("refresh"))
            if not provider_id or provider_id == "all":
                results = await asyncio.gather(
                    *(asyncio.to_thread(p.list_convs_sync, refresh) for p in self.providers.values()),
                    return_exceptions=True,
                )
                convs: list[dict] = []
                errors: dict[str, str] = {}
                for pid, res in zip(self.providers.keys(), results):
                    if isinstance(res, Exception):
                        errors[pid] = str(res)
                    else:
                        convs.extend(res)
                convs.sort(key=lambda x: x.get("mtime_ts", 0), reverse=True)
                counts = {pid: 0 for pid in self.providers}
                for c in convs:
                    counts[c["provider"]] = counts.get(c["provider"], 0) + 1
                return {"convs": convs, "counts": counts, "errors": errors}
            prov = self._get_provider(provider_id)
            convs = await asyncio.to_thread(prov.list_convs_sync, refresh)
            return {"convs": convs, "counts": {prov.id: len(convs)}, "errors": {}}

        if op == "entries":
            prov = self._get_provider(provider_id)
            cid = d["cid"]
            limit = int(d.get("limit", 80))
            before = d.get("before")
            include_tools = d.get("include_tools") is True
            res = await asyncio.to_thread(
                prov.get_entries_sync, cid, limit=limit, before=before, include_tools=include_tools
            )
            if client_id:
                self.client_views[client_id] = (prov.id, str(cid))
                self._last_conv_locks[(prov.id, str(cid))] = (
                    bool(res.get("file_locked")),
                    str(res.get("lock_reason") or ""),
                )
            return res

        if op == "edit_preview":
            prov = self._get_provider(provider_id)
            return await asyncio.to_thread(prov.edit_preview_sync, d["cid"], str(d["key"]))

        if op == "apply_edit":
            prov = self._get_provider(provider_id)
            cid, key, text = d["cid"], str(d["key"]), d.get("text") or ""
            if not text.strip():
                raise ValueError("Пустой текст, сохранение отменено.")
            if not d.get("confirm"):
                raise PermissionError("Требуется подтверждение.")
            await asyncio.to_thread(self._assert_writable, prov, cid)
            rev = d.get("revision")
            ab = self.auto_backup_enabled()
            job_id = self.start_job(
                prov, "edit", lambda sp: prov.apply_edit_sync(cid, key, text, rev, sp, auto_backup=ab)
            )
            return {"job_id": job_id}

        if op == "delete_conv":
            prov = self._get_provider(provider_id)
            cid = d["cid"]
            if not d.get("confirm"):
                raise PermissionError("Требуется подтверждение.")
            await asyncio.to_thread(self._assert_writable, prov, cid)
            ab = self.auto_backup_enabled()
            job_id = self.start_job(
                prov, "delete_conv", lambda sp: prov.delete_conv_sync(cid, sp, auto_backup=ab)
            )
            return {"job_id": job_id}

        if op == "preview_cut":
            prov = self._get_provider(provider_id)
            cid, key, mode = d["cid"], str(d["key"]), d.get("mode", "tail")
            if mode not in ("head", "tail"):
                raise ValueError("Недопустимый режим обрезки.")
            return await asyncio.to_thread(prov.preview_cut_sync, cid, key, mode)

        if op == "apply_cut":
            prov = self._get_provider(provider_id)
            cid, key, mode = d["cid"], str(d["key"]), d.get("mode", "tail")
            if not d.get("confirm"):
                raise PermissionError("Требуется подтверждение повторным нажатием.")
            if mode not in ("head", "tail"):
                raise ValueError("Недопустимый режим обрезки.")
            await asyncio.to_thread(self._assert_writable, prov, cid)
            rev = d.get("revision")
            ab = self.auto_backup_enabled()
            job_id = self.start_job(
                prov, "cut", lambda sp: prov.apply_cut_sync(cid, key, mode, rev, sp, auto_backup=ab)
            )
            return {"job_id": job_id}

        if op == "delete_single":
            prov = self._get_provider(provider_id)
            cid, key = d["cid"], str(d["key"])
            if not d.get("confirm"):
                raise PermissionError("Требуется подтверждение.")
            await asyncio.to_thread(self._assert_writable, prov, cid)
            rev = d.get("revision")
            ab = self.auto_backup_enabled()
            job_id = self.start_job(
                prov, "delete_single", lambda sp: prov.delete_single_sync(cid, key, rev, sp, auto_backup=ab)
            )
            return {"job_id": job_id}

        if op == "backups":
            cid = d.get("cid") or None
            if not provider_id or provider_id == "all":
                results = await asyncio.gather(
                    *(asyncio.to_thread(p.list_backups_sync, None) for p in self.providers.values()),
                    return_exceptions=True,
                )
                groups: list[dict] = []
                roots: dict[str, str] = {}
                summary: list[dict] = []
                total_count = 0
                total_bytes = 0
                for pid, res in zip(self.providers.keys(), results):
                    p_obj = self.providers[pid]
                    if isinstance(res, dict):
                        groups.extend(res.get("groups") or [])
                        roots[pid] = res.get("root", "")
                        cnt = int(res.get("total_count") or 0)
                        bts = int(res.get("total_bytes") or 0)
                        total_count += cnt
                        total_bytes += bts
                        summary.append({
                            "provider": pid,
                            "name": p_obj.name,
                            "short": p_obj.short,
                            "count": cnt,
                            "bytes": bts,
                        })
                return {
                    "groups": groups,
                    "roots": roots,
                    "summary": summary,
                    "total_count": total_count,
                    "total_bytes": total_bytes,
                }
            prov = self._get_provider(provider_id)
            res = await asyncio.to_thread(prov.list_backups_sync, cid)
            cnt = int(res.get("total_count") or 0)
            bts = int(res.get("total_bytes") or 0)
            return {
                "groups": res.get("groups") or [],
                "roots": {prov.id: res.get("root", "")},
                "summary": [{
                    "provider": prov.id,
                    "name": prov.name,
                    "short": prov.short,
                    "count": cnt,
                    "bytes": bts,
                }],
                "total_count": cnt,
                "total_bytes": bts,
            }

        if op == "backups_summary":
            targets = list(self.providers.values()) if (not provider_id or provider_id == "all") else [self._get_provider(provider_id)]
            rows = await asyncio.gather(*(asyncio.to_thread(p.backups_summary_sync) for p in targets))
            return {
                "providers": list(rows),
                "total_count": sum(r["count"] for r in rows),
                "total_bytes": sum(r["bytes"] for r in rows),
            }

        if op == "backup_entries":
            prov = self._get_provider(provider_id)
            bdir = d.get("backup_dir")
            if not bdir:
                raise ValueError("Требуется backup_dir.")
            limit = int(d.get("limit", 80))
            before = d.get("before")
            include_tools = d.get("include_tools") is True
            return await asyncio.to_thread(
                prov.inspect_backup_sync, bdir, limit=limit, before=before, include_tools=include_tools
            )

        if op == "delete_backup":
            prov = self._get_provider(provider_id)
            bdir = d.get("backup_dir")
            if not bdir or not d.get("confirm"):
                raise ValueError("Требуется backup_dir и confirm=true.")
            job_id = self.start_job(
                prov, "delete_backup", lambda sp: prov.delete_backup_sync(bdir, sp)
            )
            return {"job_id": job_id}

        if op == "delete_all_backups":
            if not d.get("confirm"):
                raise PermissionError("Требуется подтверждение.")
            targets = list(self.providers.values()) if (not provider_id or provider_id == "all") else [self._get_provider(provider_id)]

            async def _run_all(set_phase):
                await set_phase("cut")
                breakdown: list[dict] = []
                total_deleted = 0
                total_freed = 0
                for prov in targets:
                    async with prov.write_lock:
                        rep = await asyncio.to_thread(prov.delete_all_backups_sync, lambda _p: None)
                    if rep["deleted_count"] > 0:
                        breakdown.append(rep)
                    total_deleted += rep["deleted_count"]
                    total_freed += rep["freed_bytes"]
                await set_phase("done")
                return {
                    "breakdown": breakdown,
                    "total_deleted": total_deleted,
                    "freed_bytes": total_freed,
                }

            job_id = self.start_multi_job(targets, "delete_all_backups", _run_all)
            return {"job_id": job_id}

        if op == "restore":
            prov = self._get_provider(provider_id)
            bdir = d.get("backup_dir")
            if not bdir or not d.get("confirm"):
                raise ValueError("Требуется backup_dir и confirm=true.")
            target_cid = Path(bdir).parent.name
            await asyncio.to_thread(self._assert_writable, prov, target_cid)
            job_id = self.start_job(
                prov, "restore", lambda sp: prov.restore_backup_sync(bdir, sp)
            )
            return {"job_id": job_id}

        if op == "convert_chat":
            src_prov = self._get_provider(provider_id)
            dst_id = d.get("dst_provider")
            dst_prov = self._get_provider(dst_id)
            cid = d["cid"]
            title = (d.get("title") or "").strip() or None
            include_tools = bool(d.get("include_tools"))
            await asyncio.to_thread(self._assert_writable, dst_prov, None)
            job_id = self.start_job(
                dst_prov,
                "convert_chat",
                lambda sp: CC.convert_chat(
                    src_prov, cid, dst_prov, title=title, include_tools=include_tools, set_phase=sp
                ),
            )
            return {"job_id": job_id}

        if op == "unlock_chat":
            prov = self._get_provider(provider_id)
            cid = d["cid"]
            if not hasattr(prov, "unlock_chat_sync"):
                raise ValueError("Разблокировка чата не поддерживается для этого провайдера.")
            await asyncio.to_thread(self._assert_writable, prov, cid)
            ab = self.auto_backup_enabled()
            job_id = self.start_job(
                prov, "unlock_chat", lambda sp: prov.unlock_chat_sync(cid, sp, auto_backup=ab)
            )
            return {"job_id": job_id}

        if op == "lock_chat":
            prov = self._get_provider(provider_id)
            cid = d["cid"]
            message = (d.get("message") or "").strip()
            code = (d.get("code") or "misalignmentPolicyViolation").strip()
            if not message:
                raise ValueError("Укажите причину блокировки.")
            if not hasattr(prov, "lock_chat_sync"):
                raise ValueError("Блокировка чата не поддерживается для этого провайдера.")
            await asyncio.to_thread(self._assert_writable, prov, cid)
            ab = self.auto_backup_enabled()
            job_id = self.start_job(
                prov, "lock_chat", lambda sp: prov.lock_chat_sync(cid, message, code, sp, auto_backup=ab)
            )
            return {"job_id": job_id}

        raise ValueError(f"Неизвестная операция: {op}")

    async def handle_export_pack(self, request: web.Request) -> web.Response:
        self._check_origin(request)
        try:
            body = await request.json()
            prov = self._get_provider(body.get("provider"))
            cid = str(body.get("cid") or "")
            if not cid:
                raise ValueError("Требуется cid.")
            preset = str(body.get("preset") or "balanced")
            include_tools = body.get("include_tools") is not False
            packed, stats, filename = await asyncio.to_thread(
                CC.export_to_k1tty_bytes, prov, cid, preset=preset, include_tools=include_tools
            )
            self._prune_downloads()
            token = uuid.uuid4().hex
            self.downloads[token] = (packed, filename, time.time())
            return web.json_response({"ok": True, "token": token, "filename": filename, "stats": stats})
        except Exception as e:
            if not _is_expected_error(e):
                traceback.print_exc()
            return web.json_response({"ok": False, "error": _format_client_error(e)}, status=400)

    async def handle_download_pack(self, request: web.Request) -> web.Response:
        token = request.match_info.get("token") or ""
        item = self.downloads.get(token)
        if not item:
            raise web.HTTPNotFound(text="Expired or unknown export token")
        packed, filename, _ts = item
        disp = f"attachment; filename=\"chat.k1tty\"; filename*=UTF-8''{quote(filename)}"
        return web.Response(
            body=packed,
            content_type="application/octet-stream",
            headers={"Content-Disposition": disp, "Cache-Control": "no-store"},
        )

    async def handle_inspect_pack(self, request: web.Request) -> web.Response:
        self._check_origin(request)
        try:
            raw = await request.read()
            if not raw:
                raise ValueError("Пустой файл.")
            info = await asyncio.to_thread(KP.inspect_k1tty, raw)
            return web.json_response({"ok": True, "info": info})
        except Exception as e:
            return web.json_response({"ok": False, "error": _format_client_error(e)}, status=400)

    async def handle_import_pack(self, request: web.Request) -> web.Response:
        self._check_origin(request)
        try:
            dst_id = request.query.get("target") or ""
            dst_prov = self._get_provider(dst_id)
            await asyncio.to_thread(self._assert_writable, dst_prov, None)
            title = (request.query.get("title") or "").strip() or None
            inc_q = request.query.get("include_tools")
            include_tools = inc_q != "0"
            raw = await request.read()
            if not raw:
                raise ValueError("Пустой файл.")
            async with dst_prov.write_lock:
                res = await asyncio.to_thread(
                    CC.import_k1tty_bytes, raw, dst_prov, title=title, include_tools=include_tools
                )
            return web.json_response({"ok": True, "result": res})
        except Exception as e:
            if not _is_expected_error(e):
                traceback.print_exc()
            return web.json_response({"ok": False, "error": _format_client_error(e)}, status=400)

    async def handle_index(self, request: web.Request) -> web.StreamResponse:
        return web.FileResponse(
            STATIC_DIR / "index.html",
            headers={"Cache-Control": "no-cache"},
        )

    async def handle_static(self, request: web.Request) -> web.StreamResponse:
        name = request.match_info["name"]
        target = (STATIC_DIR / name).resolve()
        if not target.is_relative_to(STATIC_DIR.resolve()) or not target.is_file():
            raise web.HTTPNotFound()
        return web.FileResponse(target, headers={"Cache-Control": "no-cache"})

    async def handle_ws(self, request: web.Request) -> web.WebSocketResponse:
        self._check_origin(request)
        ws = web.WebSocketResponse(heartbeat=25.0, max_msg_size=64 * 1024 * 1024)
        await ws.prepare(request)

        cid_key = uuid.uuid4().hex[:8]
        self.clients[cid_key] = ws

        async def _send_reply(mid, ok: bool, payload: dict):
            if ws.closed:
                return
            try:
                await ws.send_str(json.dumps({"id": mid, "ok": ok, **payload}, ensure_ascii=False))
            except Exception:
                pass

        async def _dispatch(raw: str):
            try:
                m = json.loads(raw)
            except Exception:
                await _send_reply(None, False, {"error": "Некорректный JSON"})
                return
            mid = m.get("id")
            op = m.get("op")
            prov = m.get("provider")
            data = m.get("data") or {}
            try:
                res = await self.handle_rpc(op, prov, data, client_id=cid_key)
                await _send_reply(mid, True, res)
            except Exception as e:
                if not _is_expected_error(e):
                    traceback.print_exc()
                await _send_reply(mid, False, {"error": _format_client_error(e)})

        try:
            await ws.send_str(json.dumps({"type": "hello", "server": "k1tty-aiohttp", "boot_id": self.boot_id}, ensure_ascii=False))
            async for msg in ws:
                if msg.type == WSMsgType.TEXT:
                    self._spawn(_dispatch(msg.data))
                elif msg.type == WSMsgType.ERROR:
                    break
        finally:
            self.clients.pop(cid_key, None)
            self.client_views.pop(cid_key, None)
        return ws

    async def _process_watcher(self):
        while True:
            try:
                await asyncio.sleep(1.0)
                if not self.clients:
                    continue
                running_map = await asyncio.to_thread(snapshot_processes, AG_MOD.DAEMON_DIR, True)
                sig = {pid: tuple(sorted(running_map.get(pid, []))) for pid in self.providers}
                prev = self._last_procs_sig
                if prev is None or sig != prev:
                    changed = [pid for pid in self.providers if prev is None or sig[pid] != prev.get(pid)]
                    self._last_procs_sig = sig
                    if changed:
                        states = await asyncio.gather(
                            *(asyncio.to_thread(self.providers[pid].state_sync, running_map) for pid in changed),
                            return_exceptions=True,
                        )
                        delta = {s["id"]: s for s in states if isinstance(s, dict)}
                        if delta:
                            await self.broadcast({"type": "procs", "providers": delta})

                views = set(self.client_views.values())
                for stale_k in list(self._last_conv_locks.keys()):
                    if stale_k not in views:
                        self._last_conv_locks.pop(stale_k, None)
                for pid, cid in views:
                    prov = self.providers.get(pid)
                    if prov is None or prov.write_lock.locked():
                        continue
                    try:
                        locked, reason = await asyncio.to_thread(prov.probe_conv_lock_sync, cid)
                    except Exception:
                        continue
                    cur_val = (bool(locked), str(reason or ""))
                    if self._last_conv_locks.get((pid, cid)) != cur_val:
                        self._last_conv_locks[(pid, cid)] = cur_val
                        await self.broadcast({
                            "type": "conv_lock",
                            "provider": pid,
                            "cid": cid,
                            "locked": cur_val[0],
                            "reason": cur_val[1],
                        })
            except asyncio.CancelledError:
                break
            except Exception:
                await asyncio.sleep(2.0)

    async def on_startup(self, app: web.Application):
        self._watcher_task = asyncio.create_task(self._process_watcher())

    async def on_shutdown(self, app: web.Application):
        if self._watcher_task is not None:
            self._watcher_task.cancel()
            try:
                await self._watcher_task
            except asyncio.CancelledError:
                pass
            self._watcher_task = None
        for ws in list(self.clients.values()):
            await ws.close()
        self.clients.clear()
        for t in list(self.tasks):
            t.cancel()
        if self.tasks:
            await asyncio.gather(*self.tasks, return_exceptions=True)

    def create_app(self) -> web.Application:
        app = web.Application(client_max_size=512 * 1024 * 1024)
        app.router.add_get("/", self.handle_index)
        app.router.add_get("/ws", self.handle_ws)
        app.router.add_post("/api/export_pack", self.handle_export_pack)
        app.router.add_get("/api/download_pack/{token}", self.handle_download_pack)
        app.router.add_post("/api/inspect_pack", self.handle_inspect_pack)
        app.router.add_post("/api/import_pack", self.handle_import_pack)
        app.router.add_get("/static/{name:.+}", self.handle_static)
        app.on_startup.append(self.on_startup)
        app.on_shutdown.append(self.on_shutdown)
        return app


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("-")]
    flags = {a for a in sys.argv[1:] if a.startswith("-")}
    port = int(args[0]) if args else 8780
    no_open = "--no-open" in flags or os.environ.get("NO_BROWSER") == "1"

    try:
        for line in legacy_layout.migrate(HERE):
            print(line, flush=True)
    except Exception as e:
        print(f"Could not migrate data from k1tty_editor/: {e!r}", flush=True)

    srv = K1ttyServer()
    app = srv.create_app()
    url = f"http://127.0.0.1:{port}"
    print(f"K1tty Tools: {url}", flush=True)

    if not no_open:
        async def _open_browser(_app):
            async def _later():
                await asyncio.sleep(0.6)
                webbrowser.open(url)
            asyncio.create_task(_later())
        app.on_startup.append(_open_browser)

    web.run_app(app, host="127.0.0.1", port=port, print=None)


if __name__ == "__main__":
    main()
