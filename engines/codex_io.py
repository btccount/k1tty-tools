from contextlib import contextmanager
import errno
import os
from pathlib import Path
import re

_UUID_RE = re.compile(r"([0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12})$", re.I)


def _writer_lock_for_rollout(path: Path) -> Path | None:
    p = Path(path)
    m = _UUID_RE.search(p.stem)
    if not m:
        return None
    tid = m.group(1)
    for par in p.parents:
        if par.name == "sessions":
            locks_dir = par.parent / "thread-writer-locks"
            if locks_dir.is_dir():
                wlock = locks_dir / f"{tid}.lock"
                if wlock.is_file():
                    return wlock
            break
    return None


@contextmanager
def lock_rollouts(paths):
    if os.name != "nt":
        import fcntl

        handles = []
        lock_fds: list[int] = []
        try:
            for path in paths:
                p = Path(path)
                wlock = _writer_lock_for_rollout(p)
                if wlock is not None:
                    try:
                        wfd = os.open(str(wlock), os.O_RDWR)
                    except PermissionError:
                        wfd = os.open(str(wlock), os.O_RDONLY)
                    try:
                        fcntl.flock(wfd, fcntl.LOCK_EX | fcntl.LOCK_NB)
                    except OSError as e:
                        os.close(wfd)
                        if e.errno in (errno.EAGAIN, errno.EWOULDBLOCK, errno.EACCES):
                            raise OSError(e.errno, f"Файл занят Codex: {path}") from e
                        raise
                    lock_fds.append(wfd)

                is_sqlite = p.suffix.lower() in (".sqlite", ".sqlite3", ".db")
                fh = open(str(p), "rb" if is_sqlite else "r+b", buffering=1024 * 1024)
                if not is_sqlite:
                    try:
                        fcntl.flock(fh.fileno(), fcntl.LOCK_EX | fcntl.LOCK_NB)
                    except OSError as e:
                        fh.close()
                        if e.errno in (errno.EAGAIN, errno.EWOULDBLOCK, errno.EACCES):
                            raise OSError(e.errno, f"Файл занят другим процессом: {path}") from e
                        raise
                handles.append((fh, is_sqlite))
            yield dict(zip(paths, [h for h, _ in handles]))
        finally:
            for handle, is_sqlite in reversed(handles):
                if not is_sqlite:
                    try:
                        fcntl.flock(handle.fileno(), fcntl.LOCK_UN)
                    except OSError:
                        pass
                try:
                    handle.close()
                except Exception:
                    pass
            for wfd in reversed(lock_fds):
                try:
                    fcntl.flock(wfd, fcntl.LOCK_UN)
                except OSError:
                    pass
                try:
                    os.close(wfd)
                except OSError:
                    pass
        return

    import ctypes
    import msvcrt
    from ctypes import wintypes

    kernel = ctypes.WinDLL("kernel32", use_last_error=True)
    create = kernel.CreateFileW
    create.argtypes = [wintypes.LPCWSTR, wintypes.DWORD, wintypes.DWORD, wintypes.LPVOID,
                       wintypes.DWORD, wintypes.DWORD, wintypes.HANDLE]
    create.restype = wintypes.HANDLE
    close = kernel.CloseHandle
    close.argtypes = [wintypes.HANDLE]
    close.restype = wintypes.BOOL
    invalid = ctypes.c_void_p(-1).value
    handles = []
    try:
        for path in paths:
            handle = create(str(path), 0x80000000 | 0x40000000, 0x1, None, 3, 0x80, None)
            if handle == invalid:
                raise OSError(ctypes.get_last_error(), f"Файл занят или недоступен: {path}")
            try:
                fd = msvcrt.open_osfhandle(handle, os.O_RDWR | os.O_BINARY)
            except Exception:
                close(handle)
                raise
            handles.append(os.fdopen(fd, "r+b", buffering=1024 * 1024))
        yield dict(zip(paths, handles))
    finally:
        for handle in reversed(handles):
            handle.close()
