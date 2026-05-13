use std::convert::TryFrom;

use crate::{Error, Result};

pub const COLUMN_CHUNK_HEADER_BASE_SIZE: usize = 28;
pub const COLUMN_CHUNK_ENCRYPTION_NONCE_SIZE: usize = 12;
pub const COLUMN_CHUNK_ENCRYPTION_TAG_SIZE: usize = 16;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChunkEncryptionMetadata {
    pub nonce: [u8; COLUMN_CHUNK_ENCRYPTION_NONCE_SIZE],
    pub auth_tag: [u8; COLUMN_CHUNK_ENCRYPTION_TAG_SIZE],
    pub key_id: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColumnChunkHeader {
    pub encoding_id: u8,
    pub compression_id: u8,
    pub encryption_id: u8,
    pub chunk_flags: u8,
    pub compressed_size: u32,
    pub uncompressed_size: u32,
    pub null_count: u32,
    pub row_count_chunk: u32,
    pub row_offset: u64,
    pub encryption: Option<ChunkEncryptionMetadata>,
}

impl ColumnChunkHeader {
    pub fn new_plain(
        encoding_id: u8,
        compression_id: u8,
        chunk_flags: u8,
        compressed_size: u32,
        uncompressed_size: u32,
        null_count: u32,
        row_count_chunk: u32,
        row_offset: u64,
    ) -> Self {
        Self {
            encoding_id,
            compression_id,
            encryption_id: 0x00,
            chunk_flags,
            compressed_size,
            uncompressed_size,
            null_count,
            row_count_chunk,
            row_offset,
            encryption: None,
        }
    }

    pub fn new_encrypted(
        encoding_id: u8,
        compression_id: u8,
        chunk_flags: u8,
        compressed_size: u32,
        uncompressed_size: u32,
        null_count: u32,
        row_count_chunk: u32,
        row_offset: u64,
        encryption: ChunkEncryptionMetadata,
    ) -> Self {
        Self {
            encoding_id,
            compression_id,
            encryption_id: 0x01,
            chunk_flags,
            compressed_size,
            uncompressed_size,
            null_count,
            row_count_chunk,
            row_offset,
            encryption: Some(encryption),
        }
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        let mut out = Vec::new();
        out.push(self.encoding_id);
        out.push(self.compression_id);
        out.push(self.encryption_id);
        out.push(self.chunk_flags);
        out.extend_from_slice(&self.compressed_size.to_le_bytes());
        out.extend_from_slice(&self.uncompressed_size.to_le_bytes());
        out.extend_from_slice(&self.null_count.to_le_bytes());
        out.extend_from_slice(&self.row_count_chunk.to_le_bytes());
        out.extend_from_slice(&self.row_offset.to_le_bytes());

        if self.encryption_id != 0x00 {
            let encryption = self
                .encryption
                .as_ref()
                .ok_or(Error::InvalidColumnChunkHeader)?;
            let key_id_len = u16::try_from(encryption.key_id.len())
                .map_err(|_| Error::InvalidColumnChunkHeader)?;
            out.extend_from_slice(&encryption.nonce);
            out.extend_from_slice(&encryption.auth_tag);
            out.extend_from_slice(&key_id_len.to_le_bytes());
            out.extend_from_slice(&encryption.key_id);
        }

        Ok(out)
    }

    pub fn parse(bytes: &[u8]) -> Result<(Self, usize)> {
        if bytes.len() < COLUMN_CHUNK_HEADER_BASE_SIZE {
            return Err(Error::FileTooSmall {
                file_size: bytes.len() as u64,
            });
        }

        let encryption_id = bytes[2];
        if encryption_id != 0x00 && encryption_id != 0x01 {
            return Err(Error::UnknownEncryption { id: encryption_id });
        }

        let mut header = Self {
            encoding_id: bytes[0],
            compression_id: bytes[1],
            encryption_id,
            chunk_flags: bytes[3],
            compressed_size: u32::from_le_bytes(bytes[4..8].try_into().unwrap()),
            uncompressed_size: u32::from_le_bytes(bytes[8..12].try_into().unwrap()),
            null_count: u32::from_le_bytes(bytes[12..16].try_into().unwrap()),
            row_count_chunk: u32::from_le_bytes(bytes[16..20].try_into().unwrap()),
            row_offset: u64::from_le_bytes(bytes[20..28].try_into().unwrap()),
            encryption: None,
        };

        let mut consumed = COLUMN_CHUNK_HEADER_BASE_SIZE;
        if encryption_id != 0x00 {
            let encryption_section_len = COLUMN_CHUNK_ENCRYPTION_NONCE_SIZE
                + COLUMN_CHUNK_ENCRYPTION_TAG_SIZE
                + 2;
            if bytes.len() < consumed + encryption_section_len {
                return Err(Error::FileTooSmall {
                    file_size: bytes.len() as u64,
                });
            }

            let mut nonce = [0u8; COLUMN_CHUNK_ENCRYPTION_NONCE_SIZE];
            nonce.copy_from_slice(&bytes[consumed..consumed + COLUMN_CHUNK_ENCRYPTION_NONCE_SIZE]);
            consumed += COLUMN_CHUNK_ENCRYPTION_NONCE_SIZE;

            let mut auth_tag = [0u8; COLUMN_CHUNK_ENCRYPTION_TAG_SIZE];
            auth_tag.copy_from_slice(&bytes[consumed..consumed + COLUMN_CHUNK_ENCRYPTION_TAG_SIZE]);
            consumed += COLUMN_CHUNK_ENCRYPTION_TAG_SIZE;

            let key_id_len = u16::from_le_bytes(bytes[consumed..consumed + 2].try_into().unwrap()) as usize;
            consumed += 2;

            if bytes.len() < consumed + key_id_len {
                return Err(Error::FileTooSmall {
                    file_size: bytes.len() as u64,
                });
            }

            let key_id = bytes[consumed..consumed + key_id_len].to_vec();
            consumed += key_id_len;

            header.encryption = Some(ChunkEncryptionMetadata {
                nonce,
                auth_tag,
                key_id,
            });
        }

        Ok((header, consumed))
    }
}
