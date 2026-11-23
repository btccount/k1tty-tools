use anyhow::{bail, ensure, Result};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq)]
pub enum FieldValue {
    Varint(u64),
    Bytes(Vec<u8>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Field {
    pub number: u32,
    pub wire_type: u8,
    pub value: FieldValue,
    pub raw: Vec<u8>,
}
impl Field {
    pub fn bytes(&self) -> Option<&[u8]> {
        match &self.value {
            FieldValue::Bytes(v) => Some(v),
            _ => None,
        }
    }
    pub fn varint(&self) -> Option<u64> {
        match self.value {
            FieldValue::Varint(v) => Some(v),
            _ => None,
        }
    }
}

pub fn read_varint(buf: &[u8], offset: &mut usize) -> Result<u64> {
    let mut result = 0u64;
    for shift in (0..70).step_by(7) {
        let b = *buf
            .get(*offset)
            .ok_or_else(|| anyhow::anyhow!("Truncated protobuf varint"))?;
        *offset += 1;
        ensure!(shift < 63 || b <= 1, "Protobuf varint overflow");
        result |= u64::from(b & 0x7f) << shift;
        if b & 0x80 == 0 {
            return Ok(result);
        }
    }
    bail!("Protobuf varint overflow")
}

pub fn encode_varint(mut value: u64) -> Vec<u8> {
    let mut out = Vec::with_capacity(10);
    while value >= 128 {
        out.push((value as u8 & 127) | 128);
        value >>= 7;
    }
    out.push(value as u8);
    out
}

pub fn encode_field(number: u32, wire_type: u8, payload: &[u8]) -> Vec<u8> {
    let mut out = encode_varint((u64::from(number) << 3) | u64::from(wire_type));
    if wire_type == 2 {
        out.extend(encode_varint(payload.len() as u64));
    }
    out.extend_from_slice(payload);
    out
}

fn parse_one(buf: &[u8], offset: &mut usize) -> Result<Field> {
    let start = *offset;
    let tag = read_varint(buf, offset)?;
    let number = u32::try_from(tag >> 3)?;
    ensure!(
        number > 0 && number <= 0x1fff_ffff,
        "Invalid protobuf field number"
    );
    let wire_type = (tag & 7) as u8;
    let value = match wire_type {
        0 => FieldValue::Varint(read_varint(buf, offset)?),
        1 | 2 | 5 => {
            let length = match wire_type {
                1 => 8,
                5 => 4,
                _ => usize::try_from(read_varint(buf, offset)?)?,
            };
            let end = offset
                .checked_add(length)
                .ok_or_else(|| anyhow::anyhow!("Protobuf length overflow"))?;
            ensure!(end <= buf.len(), "Truncated protobuf field");
            let bytes = buf[*offset..end].to_vec();
            *offset = end;
            FieldValue::Bytes(bytes)
        }
        _ => bail!("Unsupported protobuf wire type {wire_type}"),
    };
    Ok(Field {
        number,
        wire_type,
        value,
        raw: buf[start..*offset].to_vec(),
    })
}

pub fn try_parse_fields(buf: &[u8]) -> Result<Vec<Field>> {
    let mut offset = 0;
    let mut fields = Vec::new();
    while offset < buf.len() {
        fields.push(parse_one(buf, &mut offset)?);
    }
    Ok(fields)
}

pub fn parse_fields(buf: &[u8]) -> Vec<Field> {
    let mut offset = 0;
    let mut fields = Vec::new();
    while offset < buf.len() {
        match parse_one(buf, &mut offset) {
            Ok(f) => fields.push(f),
            Err(_) => break,
        }
    }
    fields
}

pub fn bytes_at(buf: &[u8], chain: &[u32]) -> Option<Vec<u8>> {
    if chain.is_empty() {
        return Some(buf.to_vec());
    }
    parse_fields(buf)
        .into_iter()
        .find(|f| f.number == chain[0] && f.wire_type == 2)
        .and_then(|f| f.bytes().and_then(|b| bytes_at(b, &chain[1..])))
}

pub fn string_at(buf: &[u8], chain: &[u32]) -> Option<String> {
    bytes_at(buf, chain).map(|v| String::from_utf8_lossy(&v).into_owned())
}

pub fn replace_nested(
    buf: &[u8],
    chain: &[u32],
    replacement: &[u8],
    all: bool,
    required: bool,
) -> Result<Vec<u8>> {
    ensure!(!chain.is_empty(), "Empty protobuf field path");
    fn replace(
        buf: &[u8],
        chain: &[u32],
        replacement: &[u8],
        all: bool,
        changed: &mut bool,
    ) -> Vec<u8> {
        let mut out = Vec::with_capacity(buf.len());
        let mut pos = 0;
        while pos < buf.len() {
            let start = pos;
            let field = match parse_one(buf, &mut pos) {
                Ok(f) => f,
                Err(_) => {
                    out.extend_from_slice(&buf[start..]);
                    break;
                }
            };
            if field.number == chain[0] && field.wire_type == 2 && (all || !*changed) {
                if chain.len() == 1 {
                    out.extend(encode_field(field.number, 2, replacement));
                    *changed = true;
                } else {
                    out.extend(encode_field(
                        field.number,
                        2,
                        &replace(
                            field.bytes().unwrap_or_default(),
                            &chain[1..],
                            replacement,
                            all,
                            changed,
                        ),
                    ));
                }
            } else {
                out.extend(field.raw);
            }
        }
        out
    }
    let mut changed = false;
    let out = replace(buf, chain, replacement, all, &mut changed);
    ensure!(
        !required || changed,
        "В шаге не найдено текстовое поле для замены"
    );
    Ok(out)
}

pub fn patch_varints(buf: &[u8], updates: &BTreeMap<u32, u64>) -> Vec<u8> {
    let mut pos = 0;
    let mut out = Vec::with_capacity(buf.len());
    while pos < buf.len() {
        let start = pos;
        let field = match parse_one(buf, &mut pos) {
            Ok(f) => f,
            Err(_) => {
                out.extend_from_slice(&buf[start..]);
                break;
            }
        };
        if let Some(value) = updates.get(&field.number).filter(|_| field.wire_type == 0) {
            out.extend(encode_field(field.number, 0, &encode_varint(*value)));
        } else {
            out.extend(field.raw);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn edits_preserve_unknown_fields_and_noncanonical_wire_bytes() {
        let mut inner = vec![0x98, 0x06, 0x81, 0x00];
        inner.extend(encode_field(1, 2, b"old"));
        inner.extend(encode_field(8, 2, b"old"));
        let mut message = encode_field(20, 2, &inner);
        message.extend_from_slice(&[0xa8, 0x06, 0x81, 0x00]);
        let edited =
            replace_nested(&message, &[20, 1], "новый текст".as_bytes(), true, true).unwrap();
        let result = bytes_at(&edited, &[20]).unwrap();
        assert!(result.starts_with(&[0x98, 0x06, 0x81, 0x00]));
        assert!(edited.ends_with(&[0xa8, 0x06, 0x81, 0x00]));
        assert_eq!(string_at(&edited, &[20, 8]).as_deref(), Some("old"));
        assert_eq!(string_at(&edited, &[20, 1]).as_deref(), Some("новый текст"));
    }
    #[test]
    fn malformed_input_never_panics_or_drops_trailing_bytes() {
        let mut message = encode_field(1, 2, b"old");
        message.extend_from_slice(&[0x82, 0xff]);
        let edited = replace_nested(&message, &[1], b"new", true, true).unwrap();
        assert!(edited.ends_with(&[0x82, 0xff]));
        assert!(try_parse_fields(&message).is_err());
        assert!(read_varint(&[255; 11], &mut 0).is_err());
    }
}
