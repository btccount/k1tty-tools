import sys
import struct


def read_varint(buf, i):
    result = 0
    shift = 0
    while True:
        b = buf[i]
        i += 1
        result |= (b & 0x7F) << shift
        if not (b & 0x80):
            break
        shift += 7
    return result, i


def parse_fields(buf):
    i = 0
    out = []
    while i < len(buf):
        try:
            tag, i = read_varint(buf, i)
        except (IndexError, ValueError):
            break
        field = tag >> 3
        wt = tag & 7
        if wt == 0:
            try:
                val, i = read_varint(buf, i)
            except (IndexError, ValueError):
                break
        elif wt == 1:
            if i + 8 > len(buf):
                break
            val = buf[i:i + 8]
            i += 8
        elif wt == 2:
            try:
                ln, i = read_varint(buf, i)
            except (IndexError, ValueError):
                break
            if i + ln > len(buf):
                break
            val = buf[i:i + ln]
            i += ln
        elif wt == 5:
            if i + 4 > len(buf):
                break
            val = buf[i:i + 4]
            i += 4
        else:
            out.append((field, wt, b"??"))
            break
        out.append((field, wt, val))
    return out


def is_printable(b):
    try:
        s = b.decode("utf-8")
    except UnicodeDecodeError:
        return False
    return all((c >= " " and c != "") or c in "\n\r\t" for c in s)


def try_nested(b):
    if not b:
        return None
    try:
        fields = parse_fields(b)
    except Exception:
        return None
    if not fields:
        return None
    valid = True
    consumed = 0
    for f, wt, v in fields:
        if f == 0 or f > 50000:
            valid = False
            break
        consumed += 1
    return fields if valid else None


def dump(buf, indent=0, max_len=400):
    pad = "  " * indent
    for field, wt, val in parse_fields(buf):
        if wt == 0:
            print(f"{pad}field {field} (varint): {val}")
        elif wt in (1, 5):
            n = struct.unpack("<d" if wt == 1 else "<f", val)
            print(f"{pad}field {field} (fixed{64 if wt == 1 else 32}): {val.hex()} f={n}")
        elif wt == 2:
            nested = try_nested(val)
            printable = is_printable(val)
            if nested and not printable:
                print(f"{pad}field {field} (message, len={len(val)}):")
                dump(val, indent + 1, max_len)
            elif printable:
                s = val.decode("utf-8")
                if len(s) > max_len:
                    s = s[:max_len] + f" ...[+{len(s)-max_len} chars]"
                print(f"{pad}field {field} (string, len={len(val)}): {s!r}")
            else:
                hx = val.hex()
                if len(hx) > max_len:
                    hx = hx[:max_len] + "..."
                print(f"{pad}field {field} (bytes, len={len(val)}): {hx}")
        else:
            print(f"{pad}field {field} (wire={wt}): ?")


if __name__ == "__main__":
    data = open(sys.argv[1], "rb").read() if len(sys.argv) > 1 else b""
    dump(data)
