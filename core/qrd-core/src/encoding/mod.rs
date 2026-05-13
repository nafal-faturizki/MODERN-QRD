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
    use crate::error::Result;

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

        while i + 5 <= data.len() {
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
    use crate::error::Result;

    /// Encodes integers using minimum bit-width.
    /// Format: [bit_width: u8] [packed_bits...]
    pub fn encode(data: &[u8]) -> Result<Vec<u8>> {
        if data.is_empty() {
            return Ok(vec![0u8]); // 0 bit-width
        }

        // Find maximum byte value to determine bit-width
        let max_value = *data.iter().max().unwrap_or(&0);
        let bit_width = 8 - max_value.leading_zeros() as u8;

        let mut out = vec![bit_width];

        if bit_width == 0 {
            return Ok(out);
        }

        // Pack bits (simple implementation)
        let mut bit_buffer = 0u8;
        let mut bits_in_buffer = 0u8;

        for &byte_val in data {
            let masked = byte_val & ((1 << bit_width) - 1);

            if bits_in_buffer + bit_width <= 8 {
                bit_buffer |= masked << bits_in_buffer;
                bits_in_buffer += bit_width;

                if bits_in_buffer == 8 {
                    out.push(bit_buffer);
                    bit_buffer = 0;
                    bits_in_buffer = 0;
                }
            } else {
                // Split across bytes
                let remaining = 8 - bits_in_buffer;
                bit_buffer |= (masked & ((1 << remaining) - 1)) << bits_in_buffer;
                out.push(bit_buffer);

                bit_buffer = masked >> remaining;
                bits_in_buffer = bit_width - remaining;
            }
        }

        if bits_in_buffer > 0 {
            out.push(bit_buffer);
        }

        Ok(out)
    }

    /// Decodes bit-packed data.
    pub fn decode(data: &[u8]) -> Result<Vec<u8>> {
        if data.is_empty() {
            return Ok(Vec::new());
        }

        let bit_width = data[0];

        if bit_width == 0 {
            return Ok(vec![0u8]); // All zeros
        }

        let mut out = Vec::new();
        let mut bit_buffer = 0u32;
        let mut bits_in_buffer = 0u32;
        let mask = (1u32 << bit_width) - 1;

        for &byte_val in &data[1..] {
            bit_buffer |= (byte_val as u32) << bits_in_buffer;
            bits_in_buffer += 8;

            while bits_in_buffer >= bit_width as u32 {
                let value = (bit_buffer & mask) as u8;
                out.push(value);
                bit_buffer >>= bit_width;
                bits_in_buffer -= bit_width as u32;
            }
        }

        Ok(out)
    }
}

/// DELTA_BINARY encoding (0x03): delta encoding for integers.
pub mod delta_binary {
    use crate::error::Result;

    /// Encodes as: [first_value: u32LE] [min_delta: u32LE] [bit_width: u8] [deltas_packed...]
    pub fn encode(data: &[u8]) -> Result<Vec<u8>> {
        if data.len() < 4 {
            // Not enough data for a single u32
            return Ok(data.to_vec());
        }

        // Treat as u32LE values
        let value_count = data.len() / 4;
        let mut values = Vec::new();

        for i in 0..value_count {
            let bytes = &data[i * 4..(i + 1) * 4];
            let val = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
            values.push(val);
        }

        if values.is_empty() {
            return Ok(Vec::new());
        }

        let first_value = values[0];
        let mut deltas = Vec::new();
        let mut min_delta = i32::MAX;

        for i in 1..values.len() {
            let delta = (values[i] as i32) - (values[i - 1] as i32);
            deltas.push(delta);
            min_delta = min_delta.min(delta);
        }

        let mut out = Vec::new();
        out.extend_from_slice(&first_value.to_le_bytes());
        out.extend_from_slice(&(min_delta as u32).to_le_bytes());

        if deltas.is_empty() {
            out.push(0);  // 0 bit-width if no deltas
            return Ok(out);
        }

        let adjusted_deltas: Vec<u32> = deltas
            .iter()
            .map(|&d| (d - min_delta) as u32)
            .collect();

        let max_adjusted = adjusted_deltas.iter().max().copied().unwrap_or(0);
        let bit_width = 32 - max_adjusted.leading_zeros() as u8;
        out.push(bit_width);

        // Pack adjusted deltas using bit_packed
        let delta_bytes: Vec<u8> = adjusted_deltas
            .iter()
            .flat_map(|v| v.to_le_bytes().to_vec())
            .collect();

        // Simple encoding: just store as-is for now
        out.extend_from_slice(&delta_bytes);

        Ok(out)
    }

    /// Decodes delta-binary data.
    pub fn decode(data: &[u8]) -> Result<Vec<u8>> {
        if data.len() < 9 {
            return Ok(Vec::new());
        }

        let first_value = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        let min_delta = i32::from_le_bytes([data[4], data[5], data[6], data[7]]) as i32;
        let _bit_width = data[8];

        let mut out = Vec::new();
        out.extend_from_slice(&first_value.to_le_bytes());

        let mut current = first_value as i32;

        // Read deltas (simplified: assume u32LE format)
        let mut i = 9;
        while i + 4 <= data.len() {
            let delta_encoded = u32::from_le_bytes([data[i], data[i + 1], data[i + 2], data[i + 3]]);
            let delta = delta_encoded as i32 + min_delta;
            current = current.wrapping_add(delta);
            out.extend_from_slice(&(current as u32).to_le_bytes());
            i += 4;
        }

        Ok(out)
    }
}

/// DELTA_BYTE_ARRAY encoding (0x04): prefix sharing for byte arrays.
pub mod delta_byte_array {
    use crate::error::Result;

    /// Encodes byte arrays using prefix sharing.
    /// Format: [prefix_len: u32LE][suffix...] for each value
    pub fn encode(data: &[u8]) -> Result<Vec<u8>> {
        Ok(data.to_vec()) // Placeholder: simplified implementation
    }

    /// Decodes delta byte array data.
    pub fn decode(data: &[u8]) -> Result<Vec<u8>> {
        Ok(data.to_vec()) // Placeholder: simplified implementation
    }
}

/// BYTE_STREAM_SPLIT encoding (0x05): byte-level splitting for floats.
pub mod byte_stream_split {
    use crate::error::Result;

    /// Encodes floating-point data by splitting into byte streams.
    pub fn encode(data: &[u8]) -> Result<Vec<u8>> {
        Ok(data.to_vec()) // Placeholder: simplified implementation
    }

    /// Decodes byte-stream split data.
    pub fn decode(data: &[u8]) -> Result<Vec<u8>> {
        Ok(data.to_vec()) // Placeholder: simplified implementation
    }
}

/// DICTIONARY_RLE encoding (0x06): dictionary + RLE for low-cardinality.
pub mod dictionary_rle {
    use crate::error::Result;

    /// Encodes using dictionary for unique values + RLE indices.
    pub fn encode(data: &[u8]) -> Result<Vec<u8>> {
        Ok(data.to_vec()) // Placeholder: simplified implementation
    }

    /// Decodes dictionary RLE data.
    pub fn decode(data: &[u8]) -> Result<Vec<u8>> {
        Ok(data.to_vec()) // Placeholder: simplified implementation
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
