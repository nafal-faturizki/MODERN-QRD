//! Error Correction Code (ECC) module using Reed-Solomon.
//! 
//! Supports configurable RS(N, K) where:
//! - N = total chunks
//! - K = data chunks
//! - NK = parity chunks (error correction capacity)

use crate::error::{Error, Result};
use reed_solomon_erasure::galois_8::ReedSolomon as RS;

/// Reed-Solomon error correction configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EccConfig {
    /// Total chunks (data + parity)
    pub n: usize,
    /// Data chunks
    pub k: usize,
}

impl EccConfig {
    /// Creates new ECC config. Validates n > k.
    pub fn new(n: usize, k: usize) -> Result<Self> {
        if n <= k || k == 0 || n == 0 {
            return Err(Error::FileTooSmall { file_size: 0 });
        }
        Ok(Self { n, k })
    }

    /// Returns parity chunk count.
    pub fn parity_count(&self) -> usize {
        self.n - self.k
    }
}

/// Encodes data chunks into parity chunks using Reed-Solomon.
pub fn encode(config: &EccConfig, data_chunks: &[&[u8]]) -> Result<Vec<Vec<u8>>> {
    if data_chunks.len() != config.k {
        return Err(Error::FileTooSmall { file_size: 0 });
    }

    let chunk_len = data_chunks
        .first()
        .map(|c| c.len())
        .unwrap_or(0);

    // Ensure all chunks have the same length
    for chunk in data_chunks {
        if chunk.len() != chunk_len {
            return Err(Error::FileTooSmall { file_size: 0 });
        }
    }

    let rs = RS::new(config.k, config.parity_count())
        .map_err(|_| Error::FileTooSmall { file_size: 0 })?;

    // Convert to OwnedShards by copying data
    let mut all_chunks: Vec<Vec<u8>> = data_chunks.iter().map(|&c| c.to_vec()).collect();

    // Create empty parity chunks
    for _ in 0..config.parity_count() {
        all_chunks.push(vec![0u8; chunk_len]);
    }

    // Convert to slice for RS encode
    let mut chunk_refs: Vec<&mut [u8]> = all_chunks.iter_mut().map(|c| c.as_mut_slice()).collect();

    rs.encode(&mut chunk_refs)
        .map_err(|_| Error::FileTooSmall { file_size: 0 })?;

    // Return only parity chunks
    Ok(all_chunks[config.k..].to_vec())
}

/// Decodes data chunks with erasure correction using Reed-Solomon.
/// Pass None for missing chunks.
pub fn decode(
    config: &EccConfig,
    chunks: &[Option<Vec<u8>>],
) -> Result<Vec<Vec<u8>>> {
    if chunks.len() != config.n {
        return Err(Error::FileTooSmall { file_size: 0 });
    }

    let chunk_len = chunks
        .iter()
        .find_map(|c| c.as_ref().map(|v| v.len()))
        .unwrap_or(0);

    let rs = RS::new(config.k, config.parity_count())
        .map_err(|_| Error::FileTooSmall { file_size: 0 })?;

    // Wrap chunks in Option for reconstruction (as required by library)
    let mut chunk_opts: Vec<Option<Vec<u8>>> = chunks.to_vec();

    // Fill missing chunks with zeros for reconstruction
    for chunk in &mut chunk_opts {
        if chunk.is_none() {
            *chunk = Some(vec![0u8; chunk_len]);
        }
    }

    rs.reconstruct(&mut chunk_opts)
        .map_err(|_| Error::FileTooSmall { file_size: 0 })?;

    // Return data chunks only
    Ok(chunk_opts
        .into_iter()
        .take(config.k)
        .filter_map(|c| c)
        .collect())
}
