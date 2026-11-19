from __future__ import annotations

try:
    from compression import zstd as _stdlib_zstd
except ImportError:
    try:
        from backports import zstd as _stdlib_zstd
    except ImportError:
        _stdlib_zstd = None

if _stdlib_zstd is None:
    try:
        import zstandard as _zstandard
    except ImportError:
        _zstandard = None
else:
    _zstandard = None

_HAS_ZSTD = (_stdlib_zstd is not None) or (_zstandard is not None)

from concurrent.futures import ThreadPoolExecutor
from datetime import datetime, timezone
import hashlib
import json
import lzma
import os
from pathlib import Path
import struct
import time
import zlib

MAGIC = b"K1TTYP1\x00"
VERSION = 1
PREFIX = struct.Struct("<8sHHIQQQ32s")
HEADER_SIZE = PREFIX.size + 32
MIB = 1024 * 1024
MAX_INDEX = 32 * MIB
MAX_BLOCK = 64 * MIB
CHUNK_SIZE = 64 * 1024

PRESET_IDS = {"fast": 0, "balanced": 1, "ultra": 2}
BLOCK_LIMITS = {"fast": 8 * MIB, "balanced": 24 * MIB, "ultra": 48 * MIB}


class K1ttyPackError(RuntimeError):
    pass


def _zstd_wlog(raw_len: int) -> int:
    wlog = 20
    while (1 << wlog) < raw_len and wlog < 27:
        wlog += 1
    return min(27, max(20, wlog))


def _zstd_options(level: int, raw_len: int) -> dict[int, int]:
    return {
        _stdlib_zstd.CompressionParameter.compression_level: level,
        _stdlib_zstd.CompressionParameter.window_log: _zstd_wlog(raw_len),
        _stdlib_zstd.CompressionParameter.enable_long_distance_matching: 1,
    }


def _zstd_compress(raw: bytes, level: int) -> bytes:
    if _stdlib_zstd is not None:
        return _stdlib_zstd.compress(raw, options=_zstd_options(level, len(raw)))
    if _zstandard is not None:
        params = _zstandard.ZstdCompressionParameters.from_level(
            level,
            window_log=_zstd_wlog(len(raw)),
            enable_ldm=True,
        )
        return _zstandard.ZstdCompressor(compression_params=params).compress(raw)
    raise K1ttyPackError("Кодек zstd недоступен: установите zstandard или используйте Python 3.14+")


def _zstd_decompress(packed: bytes, expected: int) -> bytes:
    if _stdlib_zstd is not None:
        return _stdlib_zstd.decompress(packed)
    if _zstandard is not None:
        return _zstandard.ZstdDecompressor().decompress(packed, max_output_size=expected + 1)
    raise K1ttyPackError("Для распаковки блока zstd установите пакет zstandard или используйте Python 3.14+")


def _lzma_dict(raw_len: int) -> int:
    d = 1 * MIB
    while d < raw_len and d < 64 * MIB:
        d *= 2
    return min(64 * MIB, max(1 * MIB, d))


def _lzma_compress(raw: bytes, preset_level: int = 9 | lzma.PRESET_EXTREME) -> bytes:
    filt = {
        "id": lzma.FILTER_LZMA2,
        "preset": preset_level,
        "dict_size": _lzma_dict(len(raw)),
        "lc": 3,
        "lp": 0,
        "pb": 0,
    }
    return lzma.compress(raw, format=lzma.FORMAT_XZ, check=lzma.CHECK_CRC64, filters=[filt])


def _encode_block(raw: bytes, preset: str) -> tuple[str, bytes]:
    best_codec = "store"
    best = raw

    def try_codec(name: str, data: bytes):
        nonlocal best_codec, best
        if len(data) < len(best):
            best_codec = name
            best = data

    if preset == "fast":
        if _HAS_ZSTD:
            try_codec("zstd", _zstd_compress(raw, 6))
        else:
            try_codec("lzma2", _lzma_compress(raw, 3))
    elif preset == "balanced":
        if _HAS_ZSTD:
            try_codec("zstd", _zstd_compress(raw, 19))
        else:
            try_codec("lzma2", _lzma_compress(raw, 6))
    else:
        def _run_zstd():
            return "zstd", _zstd_compress(raw, 22)

        def _run_lzma():
            return "lzma2", _lzma_compress(raw)

        if _HAS_ZSTD:
            with ThreadPoolExecutor(max_workers=2) as race:
                fz = race.submit(_run_zstd)
                fl = race.submit(_run_lzma)
                for fut in (fz, fl):
                    c_name, c_data = fut.result()
                    try_codec(c_name, c_data)
        else:
            try_codec("lzma2", _lzma_compress(raw))

    return best_codec, best


def _decode_block(codec: str, packed: bytes, expected: int) -> bytes:
    if expected <= 0 or expected > MAX_BLOCK or len(packed) > MAX_BLOCK:
        raise K1ttyPackError("Размер блока превышает лимит формата")
    if codec == "store":
        out = packed
    elif codec == "zstd":
        out = _zstd_decompress(packed, expected)
    elif codec == "lzma2":
        dec = lzma.LZMADecompressor(format=lzma.FORMAT_XZ, memlimit=256 * MIB)
        out = dec.decompress(packed, max_length=expected + 1)
        if not dec.eof or dec.unused_data:
            raise K1ttyPackError("Повреждённый поток LZMA2")
    else:
        raise K1ttyPackError(f"Неизвестный кодек: {codec}")
    if len(out) != expected:
        raise K1ttyPackError("Несовпадение размера распакованного блока")
    return out


def _ts_to_ms(ts) -> int:
    if not ts:
        return 0
    if isinstance(ts, (int, float)):
        return int(ts * 1000) if ts < 10_000_000_000 else int(ts)
    try:
        s = str(ts).strip()
        if s.endswith("Z"):
            s = s[:-1] + "+00:00"
        return int(datetime.fromisoformat(s).timestamp() * 1000)
    except Exception:
        return 0


def _ms_to_iso(ms: int) -> str:
    if not ms:
        return ""
    try:
        return datetime.fromtimestamp(ms / 1000.0, tz=timezone.utc).isoformat().replace("+00:00", "Z")
    except Exception:
        return ""


def _build_columnar_streams(doc: dict) -> tuple[bytes, list[bytes], dict]:
    meta = dict(doc.get("meta") or {})
    entries = doc.get("entries") or []

    str_map: dict[str, int] = {"": 0}
    strings: list[str] = [""]

    def intern(val) -> int:
        s = "" if val is None else str(val)
        idx = str_map.get(s)
        if idx is None:
            idx = len(strings)
            str_map[s] = idx
            strings.append(s)
        return idx

    kinds: list[int] = []
    subtypes: list[int] = []
    labels: list[int] = []
    tools: list[int] = []
    statuses: list[int] = []
    codes: list[int] = []
    ts_deltas: list[int] = []
    body_ids: list[int] = []

    body_hash_map: dict[bytes, int] = {}
    unique_bodies: list[bytes] = []
    body_groups: list[tuple[str, str]] = []
    exact_dedup_bytes = 0
    total_body_bytes = 0
    prev_ts = 0

    for it in entries:
        k_str = str(it.get("kind") or "assistant")
        t_str = str(it.get("tool") or it.get("subtype") or "")
        kinds.append(intern(k_str))
        subtypes.append(intern(it.get("subtype") or ""))
        labels.append(intern(it.get("label") or ""))
        tools.append(intern(it.get("tool") or ""))
        statuses.append(intern(it.get("status") or ""))
        codes.append(intern(it.get("code") or ""))

        ms = _ts_to_ms(it.get("ts"))
        if ms:
            ts_deltas.append(ms - prev_ts)
            prev_ts = ms
        else:
            ts_deltas.append(None)

        b = str(it.get("text") or "").encode("utf-8")
        total_body_bytes += len(b)
        if not b:
            body_ids.append(-1)
            continue
        bh = hashlib.sha256(b).digest()
        bid = body_hash_map.get(bh)
        if bid is None:
            bid = len(unique_bodies)
            body_hash_map[bh] = bid
            unique_bodies.append(b)
            body_groups.append((k_str, t_str))
        else:
            exact_dedup_bytes += len(b)
        body_ids.append(bid)

    chunk_map: dict[bytes, int] = {}
    unique_chunks: list[bytes] = []
    chunk_groups: list[tuple[str, str, int]] = []
    body_chunk_refs: list[list[int]] = []
    chunk_dedup_bytes = 0

    for bi, b in enumerate(unique_bodies):
        grp = body_groups[bi]
        refs: list[int] = []
        for off in range(0, len(b), CHUNK_SIZE):
            part = b[off:off + CHUNK_SIZE]
            ch = hashlib.sha256(part).digest()
            cid = chunk_map.get(ch)
            if cid is None:
                cid = len(unique_chunks)
                chunk_map[ch] = cid
                unique_chunks.append(part)
                chunk_groups.append((grp[0], grp[1], cid))
            else:
                chunk_dedup_bytes += len(part)
            refs.append(cid)
        body_chunk_refs.append(refs)

    col_obj = {
        "meta": meta,
        "strings": strings,
        "kinds": kinds,
        "subtypes": subtypes,
        "labels": labels,
        "tools": tools,
        "statuses": statuses,
        "codes": codes,
        "ts_deltas": ts_deltas,
        "body_ids": body_ids,
        "body_chunks": body_chunk_refs,
    }
    col_bytes = json.dumps(col_obj, ensure_ascii=False, separators=(",", ":")).encode("utf-8")
    dedup_stats = {
        "entries": len(entries),
        "total_body_bytes": total_body_bytes,
        "exact_dedup_bytes": exact_dedup_bytes,
        "chunk_dedup_bytes": chunk_dedup_bytes,
        "unique_bodies": len(unique_bodies),
        "unique_chunks": len(unique_chunks),
        "col_bytes": len(col_bytes),
        "chunk_order": [g[2] for g in sorted(chunk_groups)],
    }
    return col_bytes, unique_chunks, dedup_stats


def _make_header(flags: int, preset_id: int, index_offset: int, index_packed: bytes, index_raw_len: int) -> bytes:
    prefix = PREFIX.pack(
        MAGIC,
        VERSION,
        flags,
        preset_id,
        index_offset,
        len(index_packed),
        index_raw_len,
        hashlib.sha256(index_packed).digest(),
    )
    return prefix + hashlib.sha256(prefix).digest()


def pack_chat(doc: dict, output: Path | None = None, *, preset: str = "balanced", include_tools: bool = True) -> tuple[bytes | None, dict]:
    if preset not in PRESET_IDS:
        preset = "balanced"
    t0 = time.perf_counter()

    col_bytes, unique_chunks, dstats = _build_columnar_streams(doc)
    block_limit = BLOCK_LIMITS[preset]

    raw_blocks: list[bytes] = []
    chunk_locs: list[list[int]] = []

    cur = bytearray()
    cur_blk = 0

    def flush_block():
        nonlocal cur, cur_blk
        if cur:
            raw_blocks.append(bytes(cur))
            cur = bytearray()
            cur_blk += 1

    col_locs: list[list[int]] = []
    for off in range(0, len(col_bytes), CHUNK_SIZE):
        part = col_bytes[off:off + CHUNK_SIZE]
        if len(cur) + len(part) > block_limit and cur:
            flush_block()
        col_locs.append([cur_blk, len(cur), len(part)])
        cur += part

    chunk_locs: list[list[int]] = [[0, 0, 0]] * len(unique_chunks)
    for cid in dstats.get("chunk_order") or range(len(unique_chunks)):
        part = unique_chunks[cid]
        if len(cur) + len(part) > block_limit and cur:
            flush_block()
        chunk_locs[cid] = [cur_blk, len(cur), len(part)]
        cur += part

    flush_block()

    workers = min(max(1, os.cpu_count() or 1), max(1, len(raw_blocks)), 6)
    encoded: list[tuple[str, bytes, int, str]] = []
    if len(raw_blocks) <= 1:
        for rb in raw_blocks:
            codec, pk = _encode_block(rb, preset)
            encoded.append((codec, pk, len(rb), hashlib.sha256(rb).hexdigest()))
    else:
        def _job(rb: bytes):
            c, pk = _encode_block(rb, preset)
            return c, pk, len(rb), hashlib.sha256(rb).hexdigest()
        with ThreadPoolExecutor(max_workers=workers) as ex:
            encoded = list(ex.map(_job, raw_blocks))

    blocks_meta: list[dict] = []
    codec_counts: dict[str, int] = {}
    offset = HEADER_SIZE
    for codec, pk, rlen, hhex in encoded:
        blocks_meta.append({"o": offset, "p": len(pk), "r": rlen, "c": codec, "h": hhex})
        codec_counts[codec] = codec_counts.get(codec, 0) + 1
        offset += len(pk)

    meta = dict(doc.get("meta") or {})
    raw_json_est = dstats["total_body_bytes"] + dstats["col_bytes"]
    if meta.get("raw_size"):
        raw_json_est = max(raw_json_est, int(meta["raw_size"]))

    index_obj = {
        "format": "K1TTY-PACK",
        "version": VERSION,
        "preset": preset,
        "created_at": datetime.now(timezone.utc).isoformat().replace("+00:00", "Z"),
        "include_tools": bool(include_tools),
        "meta": {
            "source_provider": str(meta.get("source_provider") or ""),
            "source_id": str(meta.get("source_id") or ""),
            "title": str(meta.get("title") or ""),
            "directory": str(meta.get("directory") or ""),
            "entry_count": dstats["entries"],
            "user_count": sum(1 for e in (doc.get("entries") or []) if e.get("kind") == "user"),
            "assistant_count": sum(1 for e in (doc.get("entries") or []) if e.get("kind") == "assistant"),
            "raw_bytes": raw_json_est,
        },
        "col_locs": col_locs,
        "chunk_locs": chunk_locs,
        "blocks": blocks_meta,
        "stats": {
            "dedup_bytes": dstats["exact_dedup_bytes"] + dstats["chunk_dedup_bytes"],
            "unique_chunks": dstats["unique_chunks"],
            "codecs": codec_counts,
        },
    }
    index_raw = json.dumps(index_obj, ensure_ascii=False, separators=(",", ":")).encode("utf-8")
    if len(index_raw) > MAX_INDEX:
        raise K1ttyPackError("Индекс архива превышает лимит")
    index_packed = zlib.compress(index_raw, 9)

    flags = 1 if include_tools else 0
    header = _make_header(flags, PRESET_IDS[preset], offset, index_packed, len(index_raw))
    total_packed = HEADER_SIZE + sum(len(x[1]) for x in encoded) + len(index_packed)
    elapsed_ms = int((time.perf_counter() - t0) * 1000)

    ratio = (1.0 - (total_packed / raw_json_est)) * 100.0 if raw_json_est > 0 else 0.0
    summary = {
        "raw_bytes": raw_json_est,
        "packed_bytes": total_packed,
        "ratio_pct": round(max(0.0, ratio), 1),
        "dedup_bytes": dstats["exact_dedup_bytes"] + dstats["chunk_dedup_bytes"],
        "preset": preset,
        "codecs": codec_counts,
        "entries": dstats["entries"],
        "elapsed_ms": elapsed_ms,
    }

    if output is not None:
        out_p = Path(output)
        out_p.parent.mkdir(parents=True, exist_ok=True)
        tmp = out_p.with_suffix(out_p.suffix + ".tmp")
        with tmp.open("wb") as f:
            f.write(header)
            for _codec, pk, _rlen, _hhex in encoded:
                f.write(pk)
            f.write(index_packed)
        tmp.replace(out_p)
        return None, summary

    buf = bytearray()
    buf += header
    for _codec, pk, _rlen, _hhex in encoded:
        buf += pk
    buf += index_packed
    return bytes(buf), summary


def _read_header_and_index(data: bytes | memoryview) -> tuple[dict, int]:
    total_len = len(data)
    if total_len < HEADER_SIZE:
        raise K1ttyPackError("Файл слишком маленький для .k1tty")
    prefix_bytes = bytes(data[:PREFIX.size])
    hdr_hash = bytes(data[PREFIX.size:HEADER_SIZE])
    if hashlib.sha256(prefix_bytes).digest() != hdr_hash:
        raise K1ttyPackError("Повреждён заголовок архива .k1tty")
    magic, version, _flags, _preset_id, idx_off, idx_packed_len, idx_raw_len, idx_hash = PREFIX.unpack(prefix_bytes)
    if magic != MAGIC:
        raise K1ttyPackError("Неверная сигнатура файла (ожидается .k1tty)")
    if version != VERSION:
        raise K1ttyPackError(f"Неподдерживаемая версия формата: {version}")
    if idx_off < HEADER_SIZE or idx_packed_len <= 0 or idx_packed_len > MAX_INDEX or idx_raw_len <= 0 or idx_raw_len > MAX_INDEX:
        raise K1ttyPackError("Некорректные границы индекса")
    if idx_off + idx_packed_len != total_len:
        raise K1ttyPackError("Размер файла не совпадает с индексом")
    idx_packed = bytes(data[idx_off:idx_off + idx_packed_len])
    if hashlib.sha256(idx_packed).digest() != idx_hash:
        raise K1ttyPackError("Контрольная сумма индекса не совпадает")
    try:
        dec = zlib.decompressobj()
        idx_raw = dec.decompress(idx_packed, idx_raw_len + 1)
        if not dec.eof or dec.unused_data or len(idx_raw) != idx_raw_len:
            raise K1ttyPackError("Некорректный сжатый индекс")
        index_obj = json.loads(idx_raw.decode("utf-8"))
    except K1ttyPackError:
        raise
    except Exception as e:
        raise K1ttyPackError(f"Ошибка чтения индекса: {e}") from e
    return index_obj, total_len


def inspect_k1tty(source: bytes | Path) -> dict:
    if isinstance(source, (str, Path)):
        p = Path(source)
        fsize = p.stat().st_size
        if fsize < HEADER_SIZE:
            raise K1ttyPackError("Файл слишком маленький для .k1tty")
        with p.open("rb") as f:
            hdr = f.read(HEADER_SIZE)
            prefix_bytes = hdr[:PREFIX.size]
            if hashlib.sha256(prefix_bytes).digest() != hdr[PREFIX.size:]:
                raise K1ttyPackError("Повреждён заголовок архива .k1tty")
            magic, version, _flags, _pid, idx_off, idx_plen, idx_rlen, idx_hash = PREFIX.unpack(prefix_bytes)
            if magic != MAGIC or version != VERSION or idx_off + idx_plen != fsize:
                raise K1ttyPackError("Некорректный файл .k1tty")
            f.seek(idx_off)
            idx_packed = f.read(idx_plen)
        if hashlib.sha256(idx_packed).digest() != idx_hash:
            raise K1ttyPackError("Контрольная сумма индекса не совпадает")
        idx_raw = zlib.decompress(idx_packed)
        if len(idx_raw) != idx_rlen:
            raise K1ttyPackError("Размер индекса не совпадает")
        index_obj = json.loads(idx_raw.decode("utf-8"))
        packed_bytes = fsize
    else:
        index_obj, packed_bytes = _read_header_and_index(source)

    meta = index_obj.get("meta") or {}
    raw_bytes = int(meta.get("raw_bytes") or 0)
    ratio = (1.0 - (packed_bytes / raw_bytes)) * 100.0 if raw_bytes > 0 else 0.0
    return {
        "format": index_obj.get("format", "K1TTY-PACK"),
        "version": index_obj.get("version", VERSION),
        "preset": index_obj.get("preset", "balanced"),
        "created_at": index_obj.get("created_at", ""),
        "include_tools": bool(index_obj.get("include_tools", True)),
        "source_provider": meta.get("source_provider", ""),
        "source_id": meta.get("source_id", ""),
        "title": meta.get("title", ""),
        "directory": meta.get("directory", ""),
        "entry_count": int(meta.get("entry_count") or 0),
        "user_count": int(meta.get("user_count") or 0),
        "assistant_count": int(meta.get("assistant_count") or 0),
        "raw_bytes": raw_bytes,
        "packed_bytes": packed_bytes,
        "ratio_pct": round(max(0.0, ratio), 1),
        "stats": index_obj.get("stats") or {},
    }


def unpack_chat(source: bytes | Path) -> dict:
    raw_file = Path(source).read_bytes() if isinstance(source, (str, Path)) else bytes(source)
    mv = memoryview(raw_file)
    index_obj, _total_len = _read_header_and_index(mv)
    blocks_meta = index_obj.get("blocks") or []

    def _dec_one(bm: dict) -> bytes:
        o = int(bm["o"])
        p = int(bm["p"])
        r = int(bm["r"])
        c = str(bm["c"])
        h = str(bm["h"])
        if o < HEADER_SIZE or o + p > len(raw_file):
            raise K1ttyPackError("Смещение блока выходит за пределы файла")
        out = _decode_block(c, bytes(mv[o:o + p]), r)
        if hashlib.sha256(out).hexdigest() != h:
            raise K1ttyPackError("Повреждён блок данных (SHA-256 не совпал)")
        return out

    if len(blocks_meta) <= 1:
        decoded_blocks = [_dec_one(bm) for bm in blocks_meta]
    else:
        workers = min(max(1, os.cpu_count() or 1), len(blocks_meta), 6)
        with ThreadPoolExecutor(max_workers=workers) as ex:
            decoded_blocks = list(ex.map(_dec_one, blocks_meta))

    col_buf = bytearray()
    for blk_i, off, ln in index_obj.get("col_locs") or []:
        col_buf += decoded_blocks[blk_i][off:off + ln]
    col_obj = json.loads(col_buf.decode("utf-8"))

    chunk_locs = index_obj.get("chunk_locs") or []
    strings = col_obj["strings"]
    kinds = col_obj["kinds"]
    subtypes = col_obj["subtypes"]
    labels = col_obj["labels"]
    tools = col_obj["tools"]
    statuses = col_obj["statuses"]
    codes = col_obj["codes"]
    ts_deltas = col_obj["ts_deltas"]
    body_ids = col_obj["body_ids"]
    body_chunks = col_obj["body_chunks"]

    body_cache: dict[int, str] = {}

    def get_body(bid: int) -> str:
        if bid < 0:
            return ""
        cached = body_cache.get(bid)
        if cached is not None:
            return cached
        bb = bytearray()
        for ch_idx in body_chunks[bid]:
            blk_i, off, ln = chunk_locs[ch_idx]
            bb += decoded_blocks[blk_i][off:off + ln]
        txt = bb.decode("utf-8", "replace")
        body_cache[bid] = txt
        return txt

    entries: list[dict] = []
    cur_ms = 0
    for i in range(len(kinds)):
        delta = ts_deltas[i]
        if delta is not None:
            cur_ms += int(delta)
            ts_str = _ms_to_iso(cur_ms) if cur_ms > 0 else ""
        else:
            ts_str = ""
        entries.append({
            "kind": strings[kinds[i]],
            "subtype": strings[subtypes[i]],
            "label": strings[labels[i]],
            "tool": strings[tools[i]] or None,
            "status": strings[statuses[i]] or None,
            "code": strings[codes[i]],
            "ts": ts_str,
            "text": get_body(int(body_ids[i])),
        })

    return {
        "meta": col_obj.get("meta") or index_obj.get("meta") or {},
        "entries": entries,
        "pack_info": {
            "preset": index_obj.get("preset", "balanced"),
            "created_at": index_obj.get("created_at", ""),
            "include_tools": bool(index_obj.get("include_tools", True)),
            "stats": index_obj.get("stats") or {},
        },
    }
