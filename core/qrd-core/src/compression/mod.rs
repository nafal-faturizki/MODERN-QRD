//! Compression codecs for QRD format.
//! 
//! Compression is applied AFTER encoding and BEFORE encryption.

use crate::error::{Error, Result};
use std::convert::TryFrom;

/// Compression codec identifier.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionId {
    /// No compression (ID: 0x00)
    None = 0x00,
    /// ZSTD compression (ID: 0x01)
    Zstd = 0x01,
    /// LZ4 Frame compression (ID: 0x02)
    Lz4 = 0x02,
}

impl TryFrom<u8> for CompressionId {
    type Error = Error;

    fn try_from(byte: u8) -> Result<Self> {
        match byte {
            0x00 => Ok(CompressionId::None),
            0x01 => Ok(CompressionId::Zstd),
            0x02 => Ok(CompressionId::Lz4),
            id => Err(Error::UnknownCompression { id }),
        }
    }
}

/// No compression codec (0x00).
pub mod none {
    use crate::error::Result;

    /// Passes data through uncompressed.
    pub fn compress(data: &[u8]) -> Result<Vec<u8>> {
        Ok(data.to_vec())
    }

    /// Passes data through uncompressed.
    pub fn decompress(data: &[u8]) -> Result<Vec<u8>> {
        Ok(data.to_vec())
    }
}

/// ZSTD compression codec (0x01).
pub mod zstd_codec {
    use crate::error::Result;

    /// Compresses data using ZSTD with default level (3).
    pub fn compress(data: &[u8]) -> Result<Vec<u8>> {
        zstd::encode_all(data, 3).map_err(|_| crate::error::Error::FileTooSmall { file_size: 0 })
    }

    /// Decompresses ZSTD data.
    pub fn decompress(data: &[u8]) -> Result<Vec<u8>> {
        zstd::decode_all(data).map_err(|_| crate::error::Error::FileTooSmall { file_size: 0 })
    }
}

/// LZ4 Frame compression codec (0x02).
pub mod lz4_codec {
    use crate::error::Result;
    use lz4::Decoder;
    use lz4::EncoderBuilder;
    use std::io::Write;

    /// Compresses data using LZ4 Frame format.
    pub fn compress(data: &[u8]) -> Result<Vec<u8>> {
        let mut encoder = EncoderBuilder::new()
            .level(4)
            .build(Vec::new())
            .map_err(|_| crate::error::Error::FileTooSmall { file_size: 0 })?;
        encoder
            .write_all(data)
            .map_err(|_| crate::error::Error::FileTooSmall { file_size: 0 })?;
        let (compressed, finish_result) = encoder.finish();
        finish_result.map_err(|_| crate::error::Error::FileTooSmall { file_size: 0 })?;
        Ok(compressed)
    }

    /// Decompresses LZ4 Frame data.
    pub fn decompress(data: &[u8]) -> Result<Vec<u8>> {
        use std::io::Read;

        let mut decoder = Decoder::new(data)
            .map_err(|_| crate::error::Error::FileTooSmall { file_size: 0 })?;
        let mut out = Vec::new();
        decoder
            .read_to_end(&mut out)
            .map_err(|_| crate::error::Error::FileTooSmall { file_size: 0 })?;
        Ok(out)
    }
}

/// Compresses data using the specified codec.
pub fn compress(codec: CompressionId, data: &[u8]) -> Result<Vec<u8>> {
    match codec {
        CompressionId::None => none::compress(data),
        CompressionId::Zstd => zstd_codec::compress(data),
        CompressionId::Lz4 => lz4_codec::compress(data),
    }
}

/// Decompresses data using the specified codec.
pub fn decompress(codec: CompressionId, data: &[u8]) -> Result<Vec<u8>> {
    match codec {
        CompressionId::None => none::decompress(data),
        CompressionId::Zstd => zstd_codec::decompress(data),
        CompressionId::Lz4 => lz4_codec::decompress(data),
    }
}

/// Estimates compression efficiency and returns recommended codec.
pub fn adaptive_select(data: &[u8]) -> CompressionId {
    if data.len() < 100 {
        CompressionId::None // Too small to compress
    } else {
        let entropy = estimate_entropy(data);
        if entropy < 0.5 {
            CompressionId::Zstd // Good compressibility
        } else if entropy < 0.7 {
            CompressionId::Lz4 // Moderate compressibility
        } else {
            CompressionId::None // High entropy, skip compression
        }
    }
}

/// Estimates Shannon entropy of data (0.0 = highly compressible, 1.0 = random).
fn estimate_entropy(data: &[u8]) -> f64 {
    let mut freq = [0u32; 256];
    for &byte in data {
        freq[byte as usize] += 1;
    }

    let len = data.len() as f64;
    let mut entropy = 0.0;

    for count in freq.iter() {
        if *count > 0 {
            let p = (*count as f64) / len;
            entropy -= p * p.log2();
        }
    }

    entropy / 8.0 // Normalize to 0-1 range
}
