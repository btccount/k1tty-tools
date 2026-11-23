use anyhow::{bail, ensure, Context, Result};
use chrono::{DateTime, SecondsFormat, Utc};
use flate2::{write::ZlibEncoder, Compression, Decompress, FlushDecompress, Status};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    io::{Read, Write},
    time::Instant,
};

const MAGIC: &[u8; 8] = b"K1TTYP1\0";
const PREFIX_SIZE: usize = 72;
const HEADER_SIZE: usize = 104;
const MIB: usize = 1024 * 1024;
const MAX_INDEX: usize = 32 * MIB;
const MAX_BLOCK: usize = 64 * MIB;
const CHUNK_SIZE: usize = 64 * 1024;
const MAX_TOTAL: usize = 2 * 1024 * MIB;
const MAX_ENTRIES: usize = 1_000_000;

fn digest(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
fn string(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        _ => v.to_string(),
    }
}
fn field(v: &Value, name: &str) -> String {
    string(&v[name])
}
fn size(v: &Value) -> Result<usize> {
    let n = v.as_u64().context("Некорректный размер в архиве")?;
    usize::try_from(n).context("Размер архива не поддерживается этой платформой")
}
fn array<'a>(v: &'a Value, name: &str) -> Result<&'a Vec<Value>> {
    v[name]
        .as_array()
        .with_context(|| format!("Некорректная таблица {name} в архиве"))
}
fn checked_add(a: usize, b: usize) -> Result<usize> {
    a.checked_add(b).context("Переполнение размера архива")
}
fn reserve_reconstructed(used: &mut usize, bytes: usize, limit: usize) -> Result<()> {
    let next = checked_add(*used, bytes)?;
    ensure!(next <= limit, "Распакованный диалог превышает лимит 2 ГиБ");
    *used = next;
    Ok(())
}
fn lossy_utf8_len(mut bytes: &[u8]) -> Result<usize> {
    let mut len = 0;
    loop {
        match std::str::from_utf8(bytes) {
            Ok(text) => return checked_add(len, text.len()),
            Err(error) => {
                len = checked_add(len, error.valid_up_to())?;
                len = checked_add(len, '\u{fffd}'.len_utf8())?;
                match error.error_len() {
                    Some(invalid_len) => bytes = &bytes[error.valid_up_to() + invalid_len..],
                    None => return Ok(len),
                }
            }
        }
    }
}

pub(crate) fn timestamp_ms(ts: &Value) -> i64 {
    if let Some(n) = ts.as_f64() {
        return if n < 10_000_000_000.0 {
            (n * 1000.0) as i64
        } else {
            n as i64
        };
    }
    let Some(s) = ts.as_str() else {
        return 0;
    };
    DateTime::parse_from_rfc3339(s.trim())
        .map(|v| v.timestamp_millis())
        .or_else(|_| {
            chrono::NaiveDateTime::parse_from_str(s.trim(), "%Y-%m-%dT%H:%M:%S%.f")
                .map(|v| v.and_utc().timestamp_millis())
        })
        .unwrap_or(0)
}
fn iso_ms(ms: i64) -> String {
    if ms <= 0 {
        return String::new();
    }
    DateTime::<Utc>::from_timestamp_millis(ms)
        .map(|d| {
            d.to_rfc3339_opts(
                if ms % 1000 == 0 {
                    SecondsFormat::Secs
                } else {
                    SecondsFormat::Micros
                },
                true,
            )
        })
        .unwrap_or_default()
}

fn encode_zstd(raw: &[u8], level: i32) -> Result<Vec<u8>> {
    let mut encoder = zstd::stream::write::Encoder::new(Vec::new(), level)?;
    let window_log =
        ((usize::BITS - raw.len().max(1).saturating_sub(1).leading_zeros()).max(20)).min(27);
    encoder.window_log(window_log)?;
    encoder.long_distance_matching(true)?;
    encoder.set_pledged_src_size(Some(raw.len() as u64))?;
    encoder.write_all(raw)?;
    Ok(encoder.finish()?)
}
fn encode_lzma(raw: &[u8]) -> Result<Vec<u8>> {
    use xz2::stream::{Check, Filters, LzmaOptions, Stream};
    let mut options = LzmaOptions::new_preset(9 | (1 << 31))?;
    let dict = raw.len().max(MIB).next_power_of_two().min(64 * MIB) as u32;
    options
        .dict_size(dict)
        .literal_context_bits(3)
        .literal_position_bits(0)
        .position_bits(0);
    let mut filters = Filters::new();
    filters.lzma2(&options);
    let stream = Stream::new_stream_encoder(&filters, Check::Crc64)?;
    let mut encoder = xz2::write::XzEncoder::new_stream(Vec::new(), stream);
    encoder.write_all(raw)?;
    Ok(encoder.finish()?)
}
fn encode_block(raw: &[u8], preset: &str) -> Result<(String, Vec<u8>)> {
    let mut best = ("store".to_owned(), raw.to_vec());
    let level = match preset {
        "fast" => 6,
        "ultra" => 22,
        _ => 19,
    };
    let (zstd, lzma) = if preset == "ultra" {
        std::thread::scope(|scope| -> Result<_> {
            let zstd = scope.spawn(|| encode_zstd(raw, level));
            let lzma = encode_lzma(raw)?;
            let zstd = zstd
                .join()
                .map_err(|_| anyhow::anyhow!("Сбой потока сжатия"))??;
            Ok((zstd, Some(lzma)))
        })?
    } else {
        (encode_zstd(raw, level)?, None)
    };
    if zstd.len() < best.1.len() {
        best = ("zstd".into(), zstd);
    }
    if let Some(lzma) = lzma {
        if lzma.len() < best.1.len() {
            best = ("lzma2".into(), lzma);
        }
    }
    Ok(best)
}
fn decode_block(codec: &str, packed: &[u8], expected: usize) -> Result<Vec<u8>> {
    ensure!(
        expected > 0 && expected <= MAX_BLOCK && packed.len() <= MAX_BLOCK,
        "Размер блока превышает лимит формата"
    );
    let mut out = Vec::with_capacity(expected.min(MIB));
    match codec {
        "store" => out.extend_from_slice(packed),
        "zstd" => {
            let mut decoder = zstd::stream::read::Decoder::new(packed)?;
            decoder.window_log_max(27)?;
            decoder.take(expected as u64 + 1).read_to_end(&mut out)?;
        }
        "lzma2" => {
            let stream = xz2::stream::Stream::new_stream_decoder((256 * MIB) as u64, 0)?;
            let mut decoder = xz2::read::XzDecoder::new_stream(packed, stream);
            (&mut decoder)
                .take(expected as u64 + 1)
                .read_to_end(&mut out)?;
            ensure!(
                decoder.total_in() == packed.len() as u64,
                "Повреждённый поток LZMA2"
            );
        }
        _ => bail!("Неизвестный кодек: {codec}"),
    }
    ensure!(
        out.len() == expected,
        "Несовпадение размера распакованного блока"
    );
    Ok(out)
}

fn intern(map: &mut HashMap<String, usize>, strings: &mut Vec<String>, s: String) -> usize {
    if let Some(i) = map.get(&s) {
        return *i;
    }
    let i = strings.len();
    strings.push(s.clone());
    map.insert(s, i);
    i
}

pub fn pack_chat(doc: &Value, preset: &str, include_tools: bool) -> Result<(Vec<u8>, Value)> {
    let started = Instant::now();
    let preset = match preset {
        "fast" | "ultra" => preset,
        _ => "balanced",
    };
    let entries = doc["entries"]
        .as_array()
        .context("Отсутствуют записи диалога")?;
    ensure!(
        entries.len() <= MAX_ENTRIES,
        "Слишком много записей в архиве"
    );
    let mut strings = vec![String::new()];
    let mut intern_map = HashMap::from([(String::new(), 0)]);
    let mut columns: Vec<Vec<usize>> = (0..6).map(|_| Vec::new()).collect();
    let mut deltas: Vec<Option<i64>> = Vec::new();
    let mut body_ids: Vec<i64> = Vec::new();
    let mut unique_bodies: Vec<Vec<u8>> = Vec::new();
    let mut body_hashes = HashMap::new();
    let mut body_groups: Vec<(String, String)> = Vec::new();
    let mut previous = 0i64;
    let (mut total_body, mut exact_dedup, mut total_expanded) = (0usize, 0usize, 0usize);
    for entry in entries {
        let kind = if field(entry, "kind").is_empty() {
            "assistant".into()
        } else {
            field(entry, "kind")
        };
        for (i, key) in ["kind", "subtype", "label", "tool", "status", "code"]
            .iter()
            .enumerate()
        {
            let value = if i == 0 {
                kind.clone()
            } else {
                field(entry, key)
            };
            reserve_reconstructed(&mut total_expanded, value.len(), MAX_TOTAL)?;
            columns[i].push(intern(&mut intern_map, &mut strings, value));
        }
        let ms = timestamp_ms(&entry["ts"]);
        if ms != 0 {
            deltas.push(Some(ms.checked_sub(previous).context("Некорректная дата")?));
            previous = ms;
            reserve_reconstructed(&mut total_expanded, iso_ms(ms).len(), MAX_TOTAL)?;
        } else {
            deltas.push(None);
        }
        let text = field(entry, "text").into_bytes();
        total_body = checked_add(total_body, text.len())?;
        reserve_reconstructed(&mut total_expanded, text.len(), MAX_TOTAL)?;
        if text.is_empty() {
            body_ids.push(-1);
            continue;
        }
        let hash = digest(&text);
        let bid = if let Some(bid) = body_hashes.get(&hash) {
            exact_dedup += text.len();
            *bid
        } else {
            let id = unique_bodies.len();
            let tool = if field(entry, "tool").is_empty() {
                field(entry, "subtype")
            } else {
                field(entry, "tool")
            };
            unique_bodies.push(text);
            body_hashes.insert(hash, id);
            body_groups.push((kind, tool));
            id
        };
        body_ids.push(bid as i64);
    }
    let mut chunks: Vec<Vec<u8>> = Vec::new();
    let mut chunk_hashes = HashMap::new();
    let mut chunk_groups = Vec::new();
    let mut body_chunks: Vec<Vec<usize>> = Vec::new();
    let mut chunk_dedup = 0usize;
    for (bi, body) in unique_bodies.iter().enumerate() {
        let mut refs = Vec::new();
        for chunk in body.chunks(CHUNK_SIZE) {
            let hash = digest(chunk);
            let cid = if let Some(cid) = chunk_hashes.get(&hash) {
                chunk_dedup += chunk.len();
                *cid
            } else {
                let cid = chunks.len();
                chunks.push(chunk.to_vec());
                chunk_hashes.insert(hash, cid);
                chunk_groups.push((body_groups[bi].0.clone(), body_groups[bi].1.clone(), cid));
                cid
            };
            refs.push(cid);
        }
        body_chunks.push(refs);
    }
    let meta = doc["meta"].as_object().cloned().unwrap_or_default();
    let col_obj = json!({"meta":meta,"strings":strings,"kinds":columns[0],"subtypes":columns[1],
        "labels":columns[2],"tools":columns[3],"statuses":columns[4],"codes":columns[5],
        "ts_deltas":deltas,"body_ids":body_ids,"body_chunks":body_chunks});
    let col_bytes = serde_json::to_vec(&col_obj)?;
    let stored_bytes = chunks.iter().try_fold(col_bytes.len(), |total, chunk| {
        checked_add(total, chunk.len())
    })?;
    ensure!(
        stored_bytes <= MAX_TOTAL,
        "Распакованный архив превышает лимит 2 ГиБ"
    );
    let block_limit = match preset {
        "fast" => 8 * MIB,
        "ultra" => 48 * MIB,
        _ => 24 * MIB,
    };
    let mut raw_blocks: Vec<Vec<u8>> = vec![Vec::new()];
    let mut append = |part: &[u8]| -> [usize; 3] {
        let last = raw_blocks.last().unwrap();
        if !last.is_empty() && last.len() + part.len() > block_limit {
            raw_blocks.push(Vec::new());
        }
        let index = raw_blocks.len() - 1;
        let block = &mut raw_blocks[index];
        let loc = [index, block.len(), part.len()];
        block.extend_from_slice(part);
        loc
    };
    let col_locs: Vec<_> = col_bytes.chunks(CHUNK_SIZE).map(&mut append).collect();
    let mut chunk_locs = vec![[0usize; 3]; chunks.len()];
    chunk_groups.sort();
    for (_, _, cid) in chunk_groups {
        chunk_locs[cid] = append(&chunks[cid]);
    }
    let mut blocks = Vec::new();
    let mut encoded = Vec::new();
    let mut codec_counts = serde_json::Map::new();
    let mut offset = HEADER_SIZE;
    let workers = std::thread::available_parallelism()
        .map(usize::from)
        .unwrap_or(1)
        .min(6);
    for batch in raw_blocks.chunks(workers) {
        let results = std::thread::scope(|scope| -> Result<Vec<_>> {
            let tasks: Vec<_> = batch
                .iter()
                .map(|block| scope.spawn(move || encode_block(block, preset)))
                .collect();
            tasks
                .into_iter()
                .map(|task| {
                    task.join()
                        .map_err(|_| anyhow::anyhow!("Сбой потока сжатия"))?
                })
                .collect()
        })?;
        for (block, (codec, packed)) in batch.iter().zip(results) {
            blocks.push(json!({"o":offset,"p":packed.len(),"r":block.len(),"c":codec,"h":hex(&digest(block))}));
            let count = codec_counts
                .get(&codec)
                .and_then(Value::as_u64)
                .unwrap_or(0)
                + 1;
            codec_counts.insert(codec, json!(count));
            offset += packed.len();
            encoded.push(packed);
        }
    }
    let raw_bytes =
        (total_body + col_bytes.len()).max(doc["meta"]["raw_size"].as_u64().unwrap_or(0) as usize);
    let dedup_bytes = exact_dedup + chunk_dedup;
    let index = json!({"format":"K1TTY-PACK","version":1,"preset":preset,
        "created_at":Utc::now().to_rfc3339_opts(SecondsFormat::Micros,true),"include_tools":include_tools,
        "meta":{"source_provider":field(&doc["meta"],"source_provider"),"source_id":field(&doc["meta"],"source_id"),
            "title":field(&doc["meta"],"title"),"directory":field(&doc["meta"],"directory"),"entry_count":entries.len(),
            "user_count":entries.iter().filter(|e| e["kind"]=="user").count(),
            "assistant_count":entries.iter().filter(|e| e["kind"]=="assistant").count(),"raw_bytes":raw_bytes},
        "col_locs":col_locs,"chunk_locs":chunk_locs,"blocks":blocks,
        "stats":{"dedup_bytes":dedup_bytes,"unique_chunks":chunks.len(),"codecs":codec_counts}});
    let index_raw = serde_json::to_vec(&index)?;
    ensure!(
        index_raw.len() <= MAX_INDEX,
        "Индекс архива превышает лимит"
    );
    let mut zlib = ZlibEncoder::new(Vec::new(), Compression::best());
    zlib.write_all(&index_raw)?;
    let index_packed = zlib.finish()?;
    let mut output = Vec::new();
    output.extend_from_slice(MAGIC);
    output.extend_from_slice(&1u16.to_le_bytes());
    output.extend_from_slice(&(u16::from(include_tools)).to_le_bytes());
    output.extend_from_slice(
        &(match preset {
            "fast" => 0u32,
            "ultra" => 2,
            _ => 1,
        })
        .to_le_bytes(),
    );
    for n in [offset, index_packed.len(), index_raw.len()] {
        output.extend_from_slice(&(n as u64).to_le_bytes());
    }
    output.extend_from_slice(&digest(&index_packed));
    output.extend_from_slice(&digest(&output));
    for bytes in encoded {
        output.extend_from_slice(&bytes);
    }
    output.extend_from_slice(&index_packed);
    let ratio = if raw_bytes > 0 {
        ((1.0 - output.len() as f64 / raw_bytes as f64) * 1000.0)
            .max(0.0)
            .round()
            / 10.0
    } else {
        0.0
    };
    let summary = json!({"raw_bytes":raw_bytes,"packed_bytes":output.len(),"ratio_pct":ratio,
        "dedup_bytes":dedup_bytes,"preset":preset,"codecs":codec_counts,"entries":entries.len(),
        "elapsed_ms":started.elapsed().as_millis() as u64});
    Ok((output, summary))
}

fn read_header_index(data: &[u8]) -> Result<(Value, usize)> {
    ensure!(
        data.len() >= HEADER_SIZE,
        "Файл слишком маленький для .k1tty"
    );
    ensure!(
        digest(&data[..PREFIX_SIZE]).as_slice() == &data[PREFIX_SIZE..HEADER_SIZE],
        "Повреждён заголовок архива .k1tty"
    );
    ensure!(
        &data[..8] == MAGIC,
        "Неверная сигнатура файла (ожидается .k1tty)"
    );
    let version = u16::from_le_bytes(data[8..10].try_into()?);
    ensure!(version == 1, "Неподдерживаемая версия формата: {version}");
    let get = |at| -> Result<usize> {
        usize::try_from(u64::from_le_bytes(data[at..at + 8].try_into()?))
            .context("Размер не поддерживается")
    };
    let (offset, packed_len, raw_len) = (get(16)?, get(24)?, get(32)?);
    ensure!(
        offset >= HEADER_SIZE
            && packed_len > 0
            && packed_len <= MAX_INDEX
            && raw_len > 0
            && raw_len <= MAX_INDEX,
        "Некорректные границы индекса"
    );
    ensure!(
        checked_add(offset, packed_len)? == data.len(),
        "Размер файла не совпадает с индексом"
    );
    let packed = &data[offset..];
    ensure!(
        digest(packed).as_slice() == &data[40..72],
        "Контрольная сумма индекса не совпадает"
    );
    let mut decoder = Decompress::new(true);
    let mut raw = vec![0; raw_len + 1];
    let status = decoder.decompress(packed, &mut raw, FlushDecompress::Finish)?;
    ensure!(
        status == Status::StreamEnd
            && decoder.total_out() == raw_len as u64
            && decoder.total_in() == packed.len() as u64,
        "Некорректный сжатый индекс"
    );
    raw.truncate(raw_len);
    let index: Value = serde_json::from_slice(&raw).context("Ошибка чтения индекса")?;
    ensure!(index.is_object(), "Некорректный индекс архива");
    Ok((index, offset))
}
pub fn inspect_k1tty(data: &[u8]) -> Result<Value> {
    let (index, _) = read_header_index(data)?;
    let meta = &index["meta"];
    let raw = meta["raw_bytes"].as_u64().unwrap_or(0);
    let ratio = if raw > 0 {
        ((1.0 - data.len() as f64 / raw as f64) * 1000.0)
            .max(0.0)
            .round()
            / 10.0
    } else {
        0.0
    };
    Ok(
        json!({"format":index["format"],"version":index["version"],"preset":index["preset"],
        "created_at":index["created_at"],"include_tools":index["include_tools"].as_bool().unwrap_or(true),
        "source_provider":field(meta,"source_provider"),"source_id":field(meta,"source_id"),"title":field(meta,"title"),
        "directory":field(meta,"directory"),"entry_count":meta["entry_count"].as_u64().unwrap_or(0),
        "user_count":meta["user_count"].as_u64().unwrap_or(0),"assistant_count":meta["assistant_count"].as_u64().unwrap_or(0),
        "raw_bytes":raw,"packed_bytes":data.len(),"ratio_pct":ratio,"stats":index["stats"]}),
    )
}
fn location<'a>(loc: &Value, blocks: &'a [Vec<u8>]) -> Result<&'a [u8]> {
    let loc = loc
        .as_array()
        .context("Некорректное расположение фрагмента")?;
    ensure!(loc.len() == 3, "Некорректное расположение фрагмента");
    let (index, offset, len) = (size(&loc[0])?, size(&loc[1])?, size(&loc[2])?);
    let block = blocks.get(index).context("Неизвестный блок данных")?;
    block
        .get(offset..checked_add(offset, len)?)
        .context("Фрагмент выходит за пределы блока")
}
pub fn unpack_chat(data: &[u8]) -> Result<Value> {
    unpack_chat_with_limit(data, MAX_TOTAL)
}
fn unpack_chat_with_limit(data: &[u8], reconstruction_limit: usize) -> Result<Value> {
    let (index, index_offset) = read_header_index(data)?;
    let metas = array(&index, "blocks")?;
    ensure!(
        metas.len() <= MAX_TOTAL / CHUNK_SIZE,
        "Слишком много блоков в архиве"
    );
    let mut total = 0usize;
    let mut previous_end = HEADER_SIZE;
    let mut blocks = Vec::new();
    for bm in metas {
        let (offset, packed_len, raw_len) = (size(&bm["o"])?, size(&bm["p"])?, size(&bm["r"])?);
        let end = checked_add(offset, packed_len)?;
        ensure!(
            offset >= previous_end && end <= index_offset,
            "Смещение блока выходит за пределы файла"
        );
        previous_end = end;
        total = checked_add(total, raw_len)?;
        ensure!(
            total <= MAX_TOTAL,
            "Распакованный архив превышает лимит 2 ГиБ"
        );
        let block = decode_block(
            bm["c"].as_str().context("Некорректный кодек")?,
            &data[offset..end],
            raw_len,
        )?;
        ensure!(
            hex(&digest(&block)) == field(bm, "h"),
            "Повреждён блок данных (SHA-256 не совпал)"
        );
        blocks.push(block);
    }
    let mut col_bytes = Vec::new();
    for loc in array(&index, "col_locs")? {
        let bytes = location(loc, &blocks)?;
        ensure!(
            checked_add(col_bytes.len(), bytes.len())? <= MAX_TOTAL,
            "Слишком большая таблица записей"
        );
        col_bytes.extend_from_slice(bytes);
    }
    let cols: Value = serde_json::from_slice(&col_bytes).context("Повреждена таблица записей")?;
    let strings = array(&cols, "strings")?;
    let kinds = array(&cols, "kinds")?;
    ensure!(kinds.len() <= MAX_ENTRIES, "Слишком много записей в архиве");
    let mut columns = Vec::new();
    for name in [
        "kinds",
        "subtypes",
        "labels",
        "tools",
        "statuses",
        "codes",
        "ts_deltas",
        "body_ids",
    ] {
        let col = array(&cols, name)?;
        ensure!(
            col.len() == kinds.len(),
            "Длины колонок архива не совпадают"
        );
        columns.push(col);
    }
    let body_chunks = array(&cols, "body_chunks")?;
    let chunk_locs = array(&index, "chunk_locs")?;
    let mut body_cache: HashMap<usize, String> = HashMap::new();
    let mut entries = Vec::with_capacity(kinds.len());
    let mut ms = 0i64;
    let mut reconstructed = 0usize;
    for i in 0..kinds.len() {
        let mut fields = Vec::new();
        for col in &columns[..6] {
            fields.push(
                strings
                    .get(size(&col[i])?)
                    .and_then(Value::as_str)
                    .context("Некорректная ссылка на строку")?,
            );
        }
        for field in &fields {
            reserve_reconstructed(&mut reconstructed, field.len(), reconstruction_limit)?;
        }
        let ts = if columns[6][i].is_null() {
            String::new()
        } else {
            ms = ms
                .checked_add(columns[6][i].as_i64().context("Некорректная дата")?)
                .context("Переполнение даты")?;
            iso_ms(ms)
        };
        reserve_reconstructed(&mut reconstructed, ts.len(), reconstruction_limit)?;
        let bid = columns[7][i]
            .as_i64()
            .context("Некорректный номер сообщения")?;
        let text = if bid < 0 {
            String::new()
        } else {
            let bid = usize::try_from(bid)?;
            if let Some(text) = body_cache.get(&bid) {
                reserve_reconstructed(&mut reconstructed, text.len(), reconstruction_limit)?;
                text.clone()
            } else {
                let refs = body_chunks
                    .get(bid)
                    .and_then(Value::as_array)
                    .context("Неизвестное сообщение")?;
                let mut raw_len = 0;
                for ch in refs {
                    let loc = chunk_locs.get(size(ch)?).context("Неизвестный фрагмент")?;
                    let part = location(loc, &blocks)?;
                    raw_len = checked_add(raw_len, part.len())?;
                    ensure!(
                        raw_len <= reconstruction_limit.saturating_sub(reconstructed),
                        "Слишком большое сообщение"
                    );
                }
                let mut bytes = Vec::with_capacity(raw_len);
                for ch in refs {
                    let loc = chunk_locs.get(size(ch)?).context("Неизвестный фрагмент")?;
                    let part = location(loc, &blocks)?;
                    bytes.extend_from_slice(part);
                }
                reserve_reconstructed(
                    &mut reconstructed,
                    lossy_utf8_len(&bytes)?,
                    reconstruction_limit,
                )?;
                let text = String::from_utf8_lossy(&bytes).into_owned();
                body_cache.insert(bid, text.clone());
                text
            }
        };
        entries.push(json!({"kind":fields[0],"subtype":fields[1],"label":fields[2],
            "tool":if fields[3].is_empty(){None}else{Some(fields[3])},
            "status":if fields[4].is_empty(){None}else{Some(fields[4])},"code":fields[5],"ts":ts,"text":text}));
    }
    Ok(
        json!({"meta":if cols["meta"].as_object().is_some_and(|m|!m.is_empty()){&cols["meta"]}else{&index["meta"]},"entries":entries,
        "pack_info":{"preset":index["preset"],"created_at":index["created_at"],
            "include_tools":index["include_tools"].as_bool().unwrap_or(true),"stats":index["stats"]}}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repeated_interned_fields_cannot_bypass_expansion_limit() {
        let label = "Подробная подпись 🐱 ".repeat(128);
        let doc = json!({"meta":{},"entries":(0..8).map(|_| json!({
            "kind":"assistant","label":label,"text":""
        })).collect::<Vec<_>>()});
        let (bytes, _) = pack_chat(&doc, "fast", true).unwrap();
        assert!(unpack_chat_with_limit(&bytes, label.len() * 2).is_err());
        let decoded = unpack_chat(&bytes).unwrap();
        assert_eq!(decoded["entries"].as_array().unwrap().len(), 8);
        assert_eq!(decoded["entries"][7]["label"], label);
    }

    fn stored_body_archive(body: &[u8]) -> Vec<u8> {
        let cols = json!({"meta":{},"strings":["","assistant"],
            "kinds":[1],"subtypes":[0],"labels":[0],"tools":[0],"statuses":[0],"codes":[0],
            "ts_deltas":[null],"body_ids":[0],"body_chunks":[[0]]});
        let mut block = serde_json::to_vec(&cols).unwrap();
        let col_len = block.len();
        block.extend_from_slice(body);
        let index = json!({"col_locs":[[0,0,col_len]],"chunk_locs":[[0,col_len,body.len()]],
            "blocks":[{"o":HEADER_SIZE,"p":block.len(),"r":block.len(),"c":"store","h":hex(&digest(&block))}]});
        let index_raw = serde_json::to_vec(&index).unwrap();
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::best());
        encoder.write_all(&index_raw).unwrap();
        let index_packed = encoder.finish().unwrap();
        let mut archive = Vec::from(*MAGIC);
        archive.extend_from_slice(&1u16.to_le_bytes());
        archive.extend_from_slice(&1u16.to_le_bytes());
        archive.extend_from_slice(&0u32.to_le_bytes());
        for n in [
            HEADER_SIZE + block.len(),
            index_packed.len(),
            index_raw.len(),
        ] {
            archive.extend_from_slice(&(n as u64).to_le_bytes());
        }
        archive.extend_from_slice(&digest(&index_packed));
        archive.extend_from_slice(&digest(&archive));
        archive.extend_from_slice(&block);
        archive.extend_from_slice(&index_packed);
        archive
    }

    #[test]
    fn invalid_utf8_expansion_is_budgeted_before_replacement() {
        let body = [b'A', 0xff, 0xff, 0xe2, 0x82, b'B', 0xf0, 0x9f];
        let archive = stored_body_archive(&body);
        let expected = String::from_utf8_lossy(&body);
        let encoded_size = "assistant".len() + body.len();
        assert!(unpack_chat_with_limit(&archive, encoded_size).is_err());
        let expanded_size = "assistant".len() + expected.len();
        assert_eq!(
            unpack_chat_with_limit(&archive, expanded_size).unwrap()["entries"][0]["text"],
            expected.as_ref()
        );
    }

    #[test]
    fn roundtrip_deduplicated_unicode_and_dates() {
        let text = "Привет 🐱!".repeat(10_000);
        let doc = json!({"meta":{"title":"Тест","source_provider":"claude"},"entries":[
            {"kind":"user","text":text,"ts":"2026-11-20T01:02:03.123Z"},
            {"kind":"assistant","text":text,"ts":"2026-11-20T01:02:04Z"},
            {"kind":"tool","subtype":"thinking","text":"","ts":null}]});
        for preset in ["fast", "balanced", "ultra"] {
            let (bytes, summary) = pack_chat(&doc, preset, true).unwrap();
            assert!(summary["dedup_bytes"].as_u64().unwrap() > 0);
            let out = unpack_chat(&bytes).unwrap();
            assert_eq!(out["entries"][0]["text"], doc["entries"][0]["text"]);
            assert_eq!(out["entries"][1]["text"], doc["entries"][1]["text"]);
            assert_eq!(out["entries"][0]["ts"], "2026-11-20T01:02:03.123000Z");
            assert_eq!(inspect_k1tty(&bytes).unwrap()["entry_count"], 3);
            let mut corrupt = bytes.clone();
            corrupt[40] ^= 1;
            assert!(unpack_chat(&corrupt).is_err());
            let mut corrupt = bytes;
            corrupt[HEADER_SIZE] ^= 1;
            assert!(unpack_chat(&corrupt).is_err());
        }
    }
}
