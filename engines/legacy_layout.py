"""One-time migration of local data from the old ``k1tty_editor/`` layout.

Before the project was flattened, backups and settings lived inside
``k1tty_editor/``. Those files are gitignored, so they stay on disk after
``git pull`` and the app would no longer find them. This module moves them
to the new locations at the repository root without overwriting anything.
"""
from __future__ import annotations

import shutil
from pathlib import Path


def _merge_dir(src: Path, dst: Path, moved: list[str], conflicts: list[str]) -> None:
    dst.mkdir(parents=True, exist_ok=True)
    for child in sorted(src.iterdir()):
        target = dst / child.name
        if not target.exists():
            shutil.move(str(child), str(target))
            moved.append(str(target))
        elif child.is_dir() and target.is_dir():
            _merge_dir(child, target, moved, conflicts)
        else:
            conflicts.append(str(child))
    _remove_if_empty(src)


def _remove_if_empty(path: Path) -> None:
    try:
        path.rmdir()
    except OSError:
        pass


def _prune_legacy_root(legacy: Path) -> None:
    for cache in legacy.rglob("__pycache__"):
        shutil.rmtree(cache, ignore_errors=True)
    for d in sorted((p for p in legacy.rglob("*") if p.is_dir()), key=lambda p: len(p.parts), reverse=True):
        _remove_if_empty(d)
    _remove_if_empty(legacy)


def migrate(root: Path) -> list[str]:
    """Move data from ``root/k1tty_editor`` into ``root``. Returns log lines."""
    legacy = root / "k1tty_editor"
    if not legacy.is_dir():
        return []

    moved: list[str] = []
    conflicts: list[str] = []

    dir_pairs = (
        (legacy / "backups", root / "backups"),
        # Old standalone default of ag_history_tool.BACKUP_ROOT.
        (legacy / "engines" / "backups", root / "backups" / "antigravity"),
    )
    for src, dst in dir_pairs:
        if src.is_dir():
            _merge_dir(src, dst, moved, conflicts)

    settings_src = legacy / "k1tty_settings.json"
    settings_dst = root / "k1tty_settings.json"
    if settings_src.is_file():
        if settings_dst.exists():
            conflicts.append(str(settings_src))
        else:
            shutil.move(str(settings_src), str(settings_dst))
            moved.append(str(settings_dst))

    _prune_legacy_root(legacy)

    log: list[str] = []
    if moved:
        log.append(f"Migrated {len(moved)} item(s) from the old k1tty_editor/ folder.")
    if conflicts:
        log.append("Not migrated (already exist in the new location), left in place:")
        log.extend(f"  {c}" for c in conflicts)
    elif legacy.exists():
        log.append(f"Old folder kept because it still contains files: {legacy}")
    return log
