from contextlib import contextmanager
import os


@contextmanager
def lock_rollouts(paths):
    if os.name != "nt":
        raise RuntimeError("Безопасная блокировка rollout-файлов реализована только для Windows")
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
