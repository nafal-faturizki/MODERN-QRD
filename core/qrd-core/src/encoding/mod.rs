//! Encoding algorithms for QRD format.
//! 
//! Encoding is applied BEFORE compression and encryption.
//! Each column chunk stores its ENCODING_ID in the header.

use crate::error::{Error, Result};
use std::convert::TryFrom;

/// Encoding algorithm identifier.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncodingId {
    /// Raw serialized values, baseline (ID: 0x00)
    Plain = 0x00,
    /// Run-length encoding (ID: 0x01)
    Rle = 0x01,
    /// Bit-packed integers (ID: 0x02)
    BitPacked = 0x02,
    /// Delta encoding for binary integers (ID: 0x03)
    DeltaBinary = 0x03,
    /// Delta encoding for byte arrays (ID: 0x04)
    DeltaByteArray = 0x04,
    /// Byte-stream split for floating-point (ID: 0x05)
    ByteStreamSplit = 0x05,
    /// Dictionary RLE for low-cardinality (ID: 0x06)
    DictionaryRle = 0x06,
}

impl TryFrom<u8> for EncodingId {
    type Error = Error;

    fn try_from(byte: u8) -> Result<Self> {
        match byte {
            0x00 => Ok(EncodingId::Plain),
            0x01 => Ok(EncodingId::Rle),
            0x02 => Ok(EncodingId::BitPacked),
            0x03 => Ok(EncodingId::DeltaBinary),
            0x04 => Ok(EncodingId::DeltaByteArray),
            0x05 => Ok(EncodingId::ByteStreamSplit),
            0x06 => Ok(EncodingId::DictionaryRle),
            id => Err(Error::UnknownEncoding { id }),
        }
    }
}

fn take_bytes<'a>(data: &'a [u8], cursor: &mut usize, len: usize, algorithm: &'static str) -> Result<&'a [u8]> {
    let end = cursor.checked_add(len).ok_or(Error::EncodingTruncated { algorithm })?;
    let slice = data.get(*cursor..end).ok_or(Error::EncodingTruncated { algorithm })?;
    *cursor = end;
    Ok(slice)
}

fn read_u32_le(data: &[u8], cursor: &mut usize, algorithm: &'static str) -> Result<u32> {
    let bytes = take_bytes(data, cursor, 4, algorithm)?;
    let mut out = [0u8; 4];
    out.copy_from_slice(bytes);
    Ok(u32::from_le_bytes(out))
}

fn read_i32_le(data: &[u8], cursor: &mut usize, algorithm: &'static str) -> Result<i32> {
    let bytes = take_bytes(data, cursor, 4, algorithm)?;
    let mut out = [0u8; 4];
    out.copy_from_slice(bytes);
    Ok(i32::from_le_bytes(out))
}

fn read_i16_le(data: &[u8], cursor: &mut usize, algorithm: &'static str) -> Result<i16> {
    let bytes = take_bytes(data, cursor, 2, algorithm)?;
    let mut out = [0u8; 2];
    out.copy_from_slice(bytes);
    Ok(i16::from_le_bytes(out))
}

fn write_u32_le(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

/// PLAIN encoding (0x00): raw serialized values.
pub mod plain {
    use crate::error::Result;

    /// Encodes data as-is (no transformation).
    pub fn encode(data: &[u8]) -> Result<Vec<u8>> {
        Ok(data.to_vec())
    }

    /// Decodes plain data (no-op).
    pub fn decode(data: &[u8]) -> Result<Vec<u8>> {
        Ok(data.to_vec())
    }
}

/// RLE encoding (0x01): run-length encoding for repeated values.
pub mod rle {
    use crate::error::{Error, Result};

    /// Encodes data using RLE: pairs of (count: u32LE, value_byte)
    /// Works on a per-byte basis for simplicity.
    pub fn encode(data: &[u8]) -> Result<Vec<u8>> {
        if data.is_empty() {
            return Ok(Vec::new());
        }

        let mut out = Vec::new();
        let mut i = 0;

        while i < data.len() {
            let current_byte = data[i];
            let mut run_length = 1u32;

            while (i + (run_length as usize)) < data.len()
                && data[i + (run_length as usize)] == current_byte
                && run_length < u32::MAX
            {
                run_length += 1;
            }

            out.extend_from_slice(&run_length.to_le_bytes());
            out.push(current_byte);

            i += run_length as usize;
        }

        Ok(out)
    }

    /// Decodes RLE by reading (count, value) pairs.
    pub fn decode(data: &[u8]) -> Result<Vec<u8>> {
        let mut out = Vec::new();
        let mut i = 0;

        while i < data.len() {
            if i + 5 > data.len() {
                return Err(Error::EncodingTruncated { algorithm: "RLE" });
            }

            let run_length = u32::from_le_bytes([data[i], data[i + 1], data[i + 2], data[i + 3]]);
            let value = data[i + 4];

            for _ in 0..run_length {
                out.push(value);
            }

            i += 5;
        }

        Ok(out)
    }
}

/// BIT_PACKED encoding (0x02): bit-level packing for integers.
pub mod bit_packed {
    use crate::error::{Error, Result};
    use super::{read_u32_le, write_u32_le};

    /// Encodes integers using minimum bit-width.
    /// Format: [orig_len: u32LE][bit_width: u8][packed_bits...]
    pub fn encode(data: &[u8]) -> Result<Vec<u8>> {
        let mut out = Vec::new();
        write_u32_le(&mut out, data.len() as u32);

        if data.is_empty() {
            out.push(0);
            return Ok(out);
        }

        let max_value = *data.iter().max().unwrap_or(&0);
        let bit_width = if max_value == 0 { 0 } else { 8 - max_value.leading_zeros() as u8 };
        out.push(bit_width);

        if bit_width == 0 {
            return Ok(out);
        }

        let mut bit_buffer: u64 = 0;
        let mut bits_in_buffer: u8 = 0;
        let mask = (1u64 << bit_width) - 1;

        for &byte_val in data {
            let masked = (byte_val as u64) & mask;
            bit_buffer |= masked << bits_in_buffer;
            bits_in_buffer += bit_width;

            while bits_in_buffer >= 8 {
                out.push(bit_buffer as u8);
                bit_buffer >>= 8;
                bits_in_buffer -= 8;
            }
        }

        if bits_in_buffer > 0 {
            out.push(bit_buffer as u8);
        }

        Ok(out)
    }

    /// Decodes bit-packed data.
    pub fn decode(data: &[u8]) -> Result<Vec<u8>> {
        if data.len() < 5 {
            return Err(Error::EncodingTruncated { algorithm: "BIT_PACKED" });
        }

        let mut cursor = 0usize;
        let orig_len = read_u32_le(data, &mut cursor, "BIT_PACKED")? as usize;
        let bit_width = data[4];

        if bit_width > 8 {
            return Err(Error::EncodingInvalid { algorithm: "BIT_PACKED" });
        }

        if orig_len == 0 {
            return Ok(Vec::new());
        }

        if bit_width == 0 {
            return Ok(vec![0u8; orig_len]);
        }

        let mut out = Vec::with_capacity(orig_len);
        let mut bit_buffer: u64 = 0;
        let mut bits_in_buffer: u8 = 0;
        let mut cursor = 5usize;
        let mask = (1u64 << bit_width) - 1;

        while out.len() < orig_len {
            while bits_in_buffer < bit_width {
                let Some(&next_byte) = data.get(cursor) else {
                    return Err(Error::EncodingTruncated { algorithm: "BIT_PACKED" });
                };
                bit_buffer |= (next_byte as u64) << bits_in_buffer;
                bits_in_buffer += 8;
                cursor += 1;
            }

            let value = (bit_buffer & mask) as u8;
            out.push(value);
            bit_buffer >>= bit_width;
            bits_in_buffer -= bit_width;
        }

        Ok(out)
    }
}

/// DELTA_BINARY encoding (0x03): delta encoding for integers.
pub mod delta_binary {
    use crate::error::{Error, Result};
    use super::{read_i32_le, read_u32_le, write_u32_le};

    /// Encodes as: [value_count: u32LE] [first_value: u32LE] [delta_1: i32LE] ...
    pub fn encode(data: &[u8]) -> Result<Vec<u8>> {
        if data.is_empty() {
            return Ok(vec![0, 0, 0, 0]);
        }

        if data.len() % 4 != 0 {
            return Err(Error::EncodingInvalid { algorithm: "DELTA_BINARY" });
        }

        let value_count = data.len() / 4;
        let mut values = Vec::with_capacity(value_count);

        for i in 0..value_count {
            let bytes = &data[i * 4..(i + 1) * 4];
            let val = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
            values.push(val);
        }

        let mut out = Vec::new();
        write_u32_le(&mut out, value_count as u32);
        out.extend_from_slice(&values[0].to_le_bytes());

        for i in 1..values.len() {
            let delta = values[i] as i64 - values[i - 1] as i64;
            out.extend_from_slice(&(delta as i32).to_le_bytes());
        }

        Ok(out)
    }

    /// Decodes delta-binary data.
    pub fn decode(data: &[u8]) -> Result<Vec<u8>> {
        if data.len() < 8 {
            return Err(Error::EncodingTruncated { algorithm: "DELTA_BINARY" });
        }

        let mut cursor = 0usize;
        let value_count = read_u32_le(data, &mut cursor, "DELTA_BINARY")? as usize;
        if value_count == 0 {
            return Ok(Vec::new());
        }

        let expected_len = 4 + 4 + (value_count.saturating_sub(1) * 4);
        if data.len() != expected_len {
            return Err(Error::EncodingTruncated { algorithm: "DELTA_BINARY" });
        }

        let mut out = Vec::with_capacity(value_count * 4);
        let mut current = read_u32_le(data, &mut cursor, "DELTA_BINARY")?;
        out.extend_from_slice(&current.to_le_bytes());

        for _ in 1..value_count {
            let delta = read_i32_le(data, &mut cursor, "DELTA_BINARY")?;
            current = current.wrapping_add(delta as u32);
            out.extend_from_slice(&current.to_le_bytes());
        }

        Ok(out)
    }
}

/// DELTA_BYTE_ARRAY encoding (0x04): prefix sharing for byte arrays.
pub mod delta_byte_array {
    use crate::error::{Error, Result};
    use super::{read_i16_le, read_u32_le, write_u32_le};

    /// Encodes bytes using delta from the previous byte.
    /// Format: [orig_len: u32LE][first_byte: u8][delta_i: i16LE ...]
    pub fn encode(data: &[u8]) -> Result<Vec<u8>> {
        let mut out = Vec::new();
        write_u32_le(&mut out, data.len() as u32);

        if data.is_empty() {
            return Ok(out);
        }

        out.push(data[0]);
        for i in 1..data.len() {
            let delta = data[i] as i16 - data[i - 1] as i16;
            out.extend_from_slice(&delta.to_le_bytes());
        }

        Ok(out)
    }

    /// Decodes delta byte array data.
    pub fn decode(data: &[u8]) -> Result<Vec<u8>> {
        if data.len() < 4 {
            return Err(Error::EncodingTruncated { algorithm: "DELTA_BYTE_ARRAY" });
        }

        let mut cursor = 0usize;
        let orig_len = read_u32_le(data, &mut cursor, "DELTA_BYTE_ARRAY")? as usize;
        if orig_len == 0 {
            return Ok(Vec::new());
        }

        if data.len() != 4 + 1 + (orig_len.saturating_sub(1) * 2) {
            return Err(Error::EncodingTruncated { algorithm: "DELTA_BYTE_ARRAY" });
        }

        let mut out = Vec::with_capacity(orig_len);
        let mut current = data[4];
        out.push(current);

        let mut cursor = 5usize;
        for _ in 1..orig_len {
            let delta = read_i16_le(data, &mut cursor, "DELTA_BYTE_ARRAY")?;
            current = ((current as i16) + delta) as u8;
            out.push(current);
        }

        Ok(out)
    }
}

/// BYTE_STREAM_SPLIT encoding (0x05): byte-level splitting for floats.
pub mod byte_stream_split {
    use crate::error::{Error, Result};
    use super::{read_u32_le, take_bytes, write_u32_le};

    /// Encodes floating-point data by splitting into byte streams.
    pub fn encode(data: &[u8]) -> Result<Vec<u8>> {
        let mut out = Vec::new();
        write_u32_le(&mut out, data.len() as u32);

        let mut streams = [Vec::new(), Vec::new(), Vec::new(), Vec::new()];
        for (index, byte) in data.iter().enumerate() {
            streams[index % 4].push(*byte);
        }

        for stream in streams.iter() {
            out.extend_from_slice(stream);
        }

        Ok(out)
    }

    /// Decodes byte-stream split data.
    pub fn decode(data: &[u8]) -> Result<Vec<u8>> {
        if data.len() < 4 {
            return Err(Error::EncodingTruncated { algorithm: "BYTE_STREAM_SPLIT" });
        }

        let mut cursor = 0usize;
        let orig_len = read_u32_le(data, &mut cursor, "BYTE_STREAM_SPLIT")? as usize;
        if orig_len == 0 {
            return Ok(Vec::new());
        }

        let stream_lengths = [
            (orig_len + 3) / 4,
            (orig_len + 2) / 4,
            (orig_len + 1) / 4,
            orig_len / 4,
        ];

        let mut cursor = 4usize;
        let mut streams = [Vec::new(), Vec::new(), Vec::new(), Vec::new()];
        for (idx, len) in stream_lengths.iter().enumerate() {
            streams[idx].extend_from_slice(take_bytes(data, &mut cursor, *len, "BYTE_STREAM_SPLIT")?);
        }

        if cursor != data.len() {
            return Err(Error::EncodingInvalid { algorithm: "BYTE_STREAM_SPLIT" });
        }

        let mut out = Vec::with_capacity(orig_len);
        let mut positions = [0usize; 4];
        for index in 0..orig_len {
            let stream_idx = index % 4;
            let byte = *streams[stream_idx]
                .get(positions[stream_idx])
                .ok_or(Error::EncodingTruncated { algorithm: "BYTE_STREAM_SPLIT" })?;
            positions[stream_idx] += 1;
            out.push(byte);
        }

        Ok(out)
    }
}

/// DICTIONARY_RLE encoding (0x06): dictionary + RLE for low-cardinality.
pub mod dictionary_rle {
    use crate::error::{Error, Result};
    use super::{read_u32_le, take_bytes, write_u32_le};

    /// Encodes using dictionary for unique values + RLE indices.
    pub fn encode(data: &[u8]) -> Result<Vec<u8>> {
        let mut dictionary = Vec::new();
        let mut index_by_value = [u16::MAX; 256];

        for &byte in data {
            if index_by_value[byte as usize] == u16::MAX {
                index_by_value[byte as usize] = dictionary.len() as u16;
                dictionary.push(byte);
            }
        }

        let mut out = Vec::new();
        write_u32_le(&mut out, data.len() as u32);
        out.push(dictionary.len() as u8);
        out.extend_from_slice(&dictionary);

        if data.is_empty() {
            return Ok(out);
        }

        let mut i = 0usize;
        while i < data.len() {
            let current = data[i];
            let index = index_by_value[current as usize] as u8;
            let mut run_length = 1u32;

            while i + (run_length as usize) < data.len()
                && data[i + run_length as usize] == current
                && run_length < u32::MAX
            {
                run_length += 1;
            }

            write_u32_le(&mut out, run_length);
            out.push(index);
            i += run_length as usize;
        }

        Ok(out)
    }

    /// Decodes dictionary RLE data.
    pub fn decode(data: &[u8]) -> Result<Vec<u8>> {
        if data.len() < 5 {
            return Err(Error::EncodingTruncated { algorithm: "DICTIONARY_RLE" });
        }

        let mut cursor = 0usize;
        let orig_len = read_u32_le(data, &mut cursor, "DICTIONARY_RLE")? as usize;
        let dict_len = data[4] as usize;

        if data.len() < 5 + dict_len {
            return Err(Error::EncodingTruncated { algorithm: "DICTIONARY_RLE" });
        }

        let dictionary = &data[5..5 + dict_len];
        let mut cursor = 5 + dict_len;
        let mut out = Vec::with_capacity(orig_len);

        while out.len() < orig_len {
            let run_length = read_u32_le(data, &mut cursor, "DICTIONARY_RLE")? as usize;
            let index = take_bytes(data, &mut cursor, 1, "DICTIONARY_RLE")?[0] as usize;

            let value = *dictionary.get(index).ok_or(Error::EncodingInvalid { algorithm: "DICTIONARY_RLE" })?;
            out.extend(std::iter::repeat(value).take(run_length));
        }

        if out.len() != orig_len || cursor != data.len() {
            return Err(Error::EncodingInvalid { algorithm: "DICTIONARY_RLE" });
        }

        Ok(out)
    }
}

/// Encodes data using the specified encoding algorithm.
pub fn encode(algorithm: EncodingId, data: &[u8]) -> Result<Vec<u8>> {
    match algorithm {
        EncodingId::Plain => plain::encode(data),
        EncodingId::Rle => rle::encode(data),
        EncodingId::BitPacked => bit_packed::encode(data),
        EncodingId::DeltaBinary => delta_binary::encode(data),
        EncodingId::DeltaByteArray => delta_byte_array::encode(data),
        EncodingId::ByteStreamSplit => byte_stream_split::encode(data),
        EncodingId::DictionaryRle => dictionary_rle::encode(data),
    }
}

/// Decodes data using the specified encoding algorithm.
pub fn decode(algorithm: EncodingId, data: &[u8]) -> Result<Vec<u8>> {
    match algorithm {
        EncodingId::Plain => plain::decode(data),
        EncodingId::Rle => rle::decode(data),
        EncodingId::BitPacked => bit_packed::decode(data),
        EncodingId::DeltaBinary => delta_binary::decode(data),
        EncodingId::DeltaByteArray => delta_byte_array::decode(data),
        EncodingId::ByteStreamSplit => byte_stream_split::decode(data),
        EncodingId::DictionaryRle => dictionary_rle::decode(data),
    }
}
