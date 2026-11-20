from __future__ import annotations

import os
from contextlib import contextmanager
from pathlib import Path


def win_long_path(p: Path | str) -> str:
    s = os.path.abspath(os.fspath(p))
    if os.name != "nt":
        return s
    if s.startswith("\\\\?\\"):
        return s
    if s.startswith("\\\\"):
        return "\\\\?\\UNC\\" + s[2:]
    return "\\\\?\\" + s


@contextmanager
def lock_files(paths):
    if os.name != "nt":
        import errno
        import fcntl

        handles = []
        opened_paths = []
        try:
            for path in paths:
                p_str = win_long_path(path)
                if not os.path.exists(p_str):
                    continue
                fh = open(p_str, "r+b", buffering=1024 * 1024)
                try:
                    fcntl.flock(fh.fileno(), fcntl.LOCK_EX | fcntl.LOCK_NB)
                except OSError as e:
                    fh.close()
                    if e.errno in (errno.EAGAIN, errno.EWOULDBLOCK, errno.EACCES):
                        raise OSError(e.errno, f"Файл занят другим процессом или недоступен: {path}") from e
                    raise
                handles.append(fh)
                opened_paths.append(path)
            yield dict(zip(opened_paths, handles))
        finally:
            for handle in reversed(handles):
                try:
                    fcntl.flock(handle.fileno(), fcntl.LOCK_UN)
                except OSError:
                    pass
                try:
                    handle.close()
                except Exception:
                    pass
        return

    import ctypes
    import msvcrt
    from ctypes import wintypes

    kernel = ctypes.WinDLL("kernel32", use_last_error=True)
    create = kernel.CreateFileW
    create.argtypes = [
        wintypes.LPCWSTR, wintypes.DWORD, wintypes.DWORD, wintypes.LPVOID,
        wintypes.DWORD, wintypes.DWORD, wintypes.HANDLE
    ]
    create.restype = wintypes.HANDLE
    close = kernel.CloseHandle
    close.argtypes = [wintypes.HANDLE]
    close.restype = wintypes.BOOL
    invalid = ctypes.c_void_p(-1).value

    handles = []
    try:
        for path in paths:
            p_str = win_long_path(path)
            if not os.path.exists(p_str):
                continue
            handle = create(p_str, 0x80000000 | 0x40000000, 0x1, None, 3, 0x80, None)
            if handle == invalid:
                err = ctypes.get_last_error()
                raise OSError(err, f"Файл занят другим процессом или недоступен: {path}")
            try:
                fd = msvcrt.open_osfhandle(handle, os.O_RDWR | os.O_BINARY)
                handles.append(os.fdopen(fd, "r+b", buffering=1024 * 1024))
            except Exception:
                close(handle)
                raise
        yield dict(zip(paths, handles))
    finally:
        for handle in reversed(handles):
            try:
                handle.close()
            except Exception:
                pass


def atomic_write_text(path: Path | str, content: str, encoding: str = "utf-8"):
    p = Path(path)
    tmp = p.with_name(f".{os.getpid()}.tmp")
    lp_target = win_long_path(p)
    lp_tmp = win_long_path(tmp)
    try:
        with open(lp_tmp, "w", encoding=encoding, newline="\n") as f:
            f.write(content)
            f.flush()
            os.fsync(f.fileno())
        os.replace(lp_tmp, lp_target)
    finally:
        if os.path.exists(lp_tmp):
            try:
                os.unlink(lp_tmp)
            except Exception:
                pass
