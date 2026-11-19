from __future__ import annotations

import asyncio
import os
from pathlib import Path
import shutil
import sys

ROOT_DIR = Path(__file__).resolve().parent.parent
ENGINES_DIR = ROOT_DIR / "engines"
if str(ENGINES_DIR) not in sys.path:
    sys.path.insert(0, str(ENGINES_DIR))
if str(ROOT_DIR) not in sys.path:
    sys.path.insert(0, str(ROOT_DIR))


def _win_lp(p: Path) -> str:
    s = str(p.resolve()) if not p.is_absolute() else str(p)
    if os.name == "nt" and not s.startswith("\\\\?\\") and len(s) >= 240:
        return "\\\\?\\" + s
    return s


def dir_size_bytes(path: Path) -> int:
    total = 0
    stack = [_win_lp(path)]
    while stack:
        cur = stack.pop()
        try:
            with os.scandir(cur) as it:
                for entry in it:
                    try:
                        if entry.is_dir(follow_symlinks=False):
                            stack.append(entry.path)
                        elif entry.is_file(follow_symlinks=False):
                            total += entry.stat(follow_symlinks=False).st_size
                    except OSError:
                        pass
        except OSError:
            pass
    return total


def safe_rmtree(path: Path):
    target = _win_lp(path)
    if os.path.isdir(target):
        shutil.rmtree(target, ignore_errors=False)


class BaseProvider:
    id: str = ""
    name: str = ""
    short: str = ""

    def __init__(self):
        self.write_lock = asyncio.Lock()

    def state_sync(self, running_map: dict[str, list[str]]) -> dict:
        raise NotImplementedError

    def list_convs_sync(self, refresh: bool = False) -> list[dict]:
        raise NotImplementedError

    def get_entries_sync(self, cid: str, *, limit: int = 80, before=None, include_tools: bool = False) -> dict:
        raise NotImplementedError

    def probe_conv_lock_sync(self, cid: str | None = None) -> tuple[bool, str]:
        return False, ""

    def edit_preview_sync(self, cid: str, key: str) -> dict:
        raise NotImplementedError

    def apply_edit_sync(self, cid: str, key: str, text: str, revision: str | None, set_phase, *, auto_backup: bool = True) -> dict:
        raise NotImplementedError

    def preview_cut_sync(self, cid: str, key: str, mode: str) -> dict:
        raise NotImplementedError

    def apply_cut_sync(self, cid: str, key: str, mode: str, revision: str | None, set_phase, *, auto_backup: bool = True) -> dict:
        raise NotImplementedError

    def delete_single_sync(self, cid: str, key: str, revision: str | None, set_phase, *, auto_backup: bool = True) -> dict:
        raise NotImplementedError("Удаление одиночной записи не поддерживается этим провайдером")

    def delete_conv_sync(self, cid: str, set_phase, *, auto_backup: bool = True) -> dict:
        raise NotImplementedError("Полное удаление диалога не поддерживается этим провайдером")

    def backup_root_path(self) -> Path:
        raise NotImplementedError

    def list_backups_sync(self, cid: str | None = None) -> dict:
        raise NotImplementedError

    def restore_backup_sync(self, backup_dir: str, set_phase) -> dict:
        raise NotImplementedError

    def inspect_backup_sync(self, backup_dir: str, *, limit: int = 80, before=None, include_tools: bool = False) -> dict:
        raise NotImplementedError("Просмотр резервной копии не поддерживается этим провайдером")

    def backups_summary_sync(self) -> dict:
        broot = self.backup_root_path()
        count = 0
        total_bytes = 0
        if broot.exists() and broot.is_dir():
            for cdir in broot.iterdir():
                if not cdir.is_dir():
                    continue
                for bdir in cdir.iterdir():
                    if not bdir.is_dir():
                        continue
                    count += 1
                    total_bytes += dir_size_bytes(bdir)
        return {
            "provider": self.id,
            "name": self.name,
            "short": self.short,
            "count": count,
            "bytes": total_bytes,
            "root": str(broot),
        }

    def delete_backup_sync(self, backup_dir: str, set_phase) -> dict:
        broot = self.backup_root_path().resolve()
        p = Path(backup_dir).resolve()
        if not p.is_relative_to(broot) or p == broot or p.parent == broot or not p.is_dir():
            raise PermissionError(f"Недопустимый путь резервной копии {self.name}.")
        set_phase("cut")
        freed = dir_size_bytes(p)
        parent = p.parent
        safe_rmtree(p)
        try:
            if parent.is_relative_to(broot) and parent != broot and not any(parent.iterdir()):
                parent.rmdir()
        except OSError:
            pass
        set_phase("done")
        return {
            "provider": self.id,
            "name": self.name,
            "deleted_count": 1,
            "freed_bytes": freed,
            "backup_dir": str(p),
        }

    def delete_all_backups_sync(self, set_phase) -> dict:
        broot = self.backup_root_path().resolve()
        deleted = 0
        freed = 0
        if not broot.exists() or not broot.is_dir():
            set_phase("done")
            return {"provider": self.id, "name": self.name, "deleted_count": 0, "freed_bytes": 0}
        set_phase("cut")
        for cdir in list(broot.iterdir()):
            if not cdir.is_dir():
                continue
            for bdir in list(cdir.iterdir()):
                if not bdir.is_dir():
                    continue
                freed += dir_size_bytes(bdir)
                safe_rmtree(bdir)
                deleted += 1
            try:
                if not any(cdir.iterdir()):
                    cdir.rmdir()
            except OSError:
                pass
        set_phase("done")
        return {
            "provider": self.id,
            "name": self.name,
            "deleted_count": deleted,
            "freed_bytes": freed,
        }


def build_providers() -> dict[str, BaseProvider]:
    from providers.antigravity import AntigravityProvider, build_cli_provider
    from providers.codex import CodexProvider
    from providers.claude import ClaudeProvider
    from providers.opencode import OpenCodeProvider

    return {
        "antigravity": AntigravityProvider(),
        "antigravity_cli": build_cli_provider(),
        "codex": CodexProvider(),
        "claude": ClaudeProvider(),
        "opencode": OpenCodeProvider(),
    }
