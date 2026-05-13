//! File reader for QRD format.
//!
//! Implements footer-first parsing and optional partial column reads.

use crate::column_chunk::{ColumnChunkHeader, COLUMN_CHUNK_ENCRYPTION_NONCE_SIZE, COLUMN_CHUNK_ENCRYPTION_TAG_SIZE, COLUMN_CHUNK_HEADER_BASE_SIZE};
use crate::compression::{self, CompressionId};
use crate::encoding::{self, EncodingId};
use crate::encryption::Cipher;
use crate::error::{Error, Result};
use crate::file_footer::{decode_footer_envelope, FooterContent, FooterRowGroupEntry, FOOTER_LENGTH_FIELD_SIZE};
use crate::file_header::{FileHeader, FILE_HEADER_SIZE};
use crate::integrity;
use crate::schema::Schema;
use std::io::{Read, Seek, SeekFrom};

/// File reader that parses QRD format footer-first.
pub struct FileReader<R: Read> {
    reader: R,
    /// Parsed file header
    file_header: FileHeader,
    /// Parsed schema from footer
    schema: Schema,
    /// File size in bytes
    file_size: u64,
    /// Footer-extracted total rows
    footer_total_rows: u64,
    /// Footer-extracted row group count
    footer_row_group_count: u16,
    /// Row-group entries parsed from footer for offset-based access
    footer_row_groups: Vec<FooterRowGroupEntry>,
}

impl<R: Read + Seek> FileReader<R> {
    /// Creates a new file reader from a seekable readable source.
    /// Reads and validates file header and parses footer.
    pub fn new(mut reader: R) -> Result<Self> {
        let file_size = reader
            .seek(SeekFrom::End(0))
            .map_err(|_| Error::FileTooSmall { file_size: 0 })?;

        if file_size < FILE_HEADER_SIZE as u64 + FOOTER_LENGTH_FIELD_SIZE as u64 {
            return Err(Error::FileTooSmall { file_size });
        }

        reader
            .seek(SeekFrom::Start(0))
            .map_err(|_| Error::FileTooSmall { file_size })?;
        let mut header_bytes = [0u8; FILE_HEADER_SIZE];
        reader
            .read_exact(&mut header_bytes)
            .map_err(|_| Error::FileTooSmall { file_size })?;

        let file_header = FileHeader::parse(&header_bytes)?;
        file_header.validate_major_version(1)?;

        reader
            .seek(SeekFrom::Start(file_size - FOOTER_LENGTH_FIELD_SIZE as u64))
            .map_err(|_| Error::FileTooSmall { file_size })?;
        let mut footer_length_bytes = [0u8; FOOTER_LENGTH_FIELD_SIZE];
        reader
            .read_exact(&mut footer_length_bytes)
            .map_err(|_| Error::FileTooSmall { file_size })?;

        let footer_length = u32::from_le_bytes(footer_length_bytes) as u64;
        if footer_length < FOOTER_LENGTH_FIELD_SIZE as u64 {
            return Err(Error::InvalidFooterLength {
                footer_len: footer_length as u32,
                file_size,
            });
        }

        if footer_length + FOOTER_LENGTH_FIELD_SIZE as u64 > file_size {
            return Err(Error::InvalidFooterLength {
                footer_len: footer_length as u32,
                file_size,
            });
        }

        let footer_start = file_size - FOOTER_LENGTH_FIELD_SIZE as u64 - footer_length;
        reader
            .seek(SeekFrom::Start(footer_start))
            .map_err(|_| Error::FileTooSmall { file_size })?;
        let mut envelope = vec![0u8; footer_length as usize + FOOTER_LENGTH_FIELD_SIZE];
        reader
            .read_exact(&mut envelope)
            .map_err(|_| Error::FileTooSmall { file_size })?;

        let footer_body = decode_footer_envelope(&envelope)?;
        let footer = FooterContent::parse(&footer_body, file_header.flags)?;

        let schema_id = footer.schema.schema_id().map_err(Error::Schema)?;
        if schema_id != file_header.schema_id {
            return Err(Error::SchemaIdMismatch);
        }

        let footer_total_rows = footer.total_rows();
        let footer_row_group_count = u16::try_from(footer.row_group_count()).map_err(|_| Error::InvalidFooterLength {
            footer_len: footer_body.len() as u32,
            file_size,
        })?;

        if file_header.row_group_count != footer.row_group_count() {
            return Err(Error::HeaderRowGroupCountMismatch {
                header: file_header.row_group_count,
                footer: footer.row_group_count(),
            });
        }

        Ok(Self {
            reader,
            file_header,
            schema: footer.schema,
            file_size,
            footer_total_rows,
            footer_row_group_count,
            footer_row_groups: footer.row_groups,
        })
    }

    /// Returns reference to parsed file header.
    pub fn file_header(&self) -> &FileHeader {
        &self.file_header
    }

    /// Returns reference to parsed schema.
    pub fn schema(&self) -> &Schema {
        &self.schema
    }

    /// Returns total row count across all row groups.
    pub fn total_rows(&self) -> u64 {
        self.footer_total_rows
    }

    /// Returns number of row groups in file.
    pub fn row_group_count(&self) -> u32 {
        self.footer_row_group_count as u32
    }

    /// Returns row group count as read from footer.
    pub fn footer_row_group_count(&self) -> u16 {
        self.footer_row_group_count
    }

    /// Returns row-group offsets parsed from footer.
    pub fn row_group_offsets(&self) -> Vec<u64> {
        self.footer_row_groups
            .iter()
            .map(|entry| entry.byte_offset)
            .collect()
    }

    /// Reads a column chunk from the file at given offset.
    /// Returns (ColumnChunkHeader, raw_chunk_bytes).
    /// Note: encrypted column chunks are not automatically decrypted here.
    pub fn read_column_chunk_at(&mut self, offset: u64) -> Result<(ColumnChunkHeader, Vec<u8>)> {
        self.reader
            .seek(SeekFrom::Start(offset))
            .map_err(|_| Error::FileTooSmall { file_size: self.file_size })?;

        let mut base_buf = vec![0u8; COLUMN_CHUNK_HEADER_BASE_SIZE];
        self.reader
            .read_exact(&mut base_buf)
            .map_err(|_| Error::FileTooSmall { file_size: self.file_size })?;

        let encryption_id = base_buf[2];
        let mut header_buf = base_buf;
        if encryption_id == 0x01 {
            let fixed_len = COLUMN_CHUNK_ENCRYPTION_NONCE_SIZE + COLUMN_CHUNK_ENCRYPTION_TAG_SIZE + 2;
            let mut fixed = vec![0u8; fixed_len];
            self.reader
                .read_exact(&mut fixed)
                .map_err(|_| Error::FileTooSmall { file_size: self.file_size })?;

            let key_len_offset = fixed_len - 2;
            let key_id_len = u16::from_le_bytes([fixed[key_len_offset], fixed[key_len_offset + 1]]) as usize;
            let mut key_id = vec![0u8; key_id_len];
            self.reader
                .read_exact(&mut key_id)
                .map_err(|_| Error::FileTooSmall { file_size: self.file_size })?;

            header_buf.extend_from_slice(&fixed);
            header_buf.extend_from_slice(&key_id);
        }

        let (header, _) = ColumnChunkHeader::parse(&header_buf)
            .map_err(|_| Error::InvalidColumnChunkHeader)?;

        let mut payload = vec![0u8; header.compressed_size as usize];
        self.reader
            .read_exact(&mut payload)
            .map_err(|_| Error::FileTooSmall { file_size: self.file_size })?;

        Ok((header, payload))
    }

    /// Reads a column chunk and validates/decodes payload into logical encoded bytes.
    /// For encrypted chunks, `master_key` must be provided.
    pub fn read_decoded_column_chunk_at(
        &mut self,
        offset: u64,
        master_key: Option<&[u8; 32]>,
    ) -> Result<Vec<u8>> {
        let (header, payload, stored_checksum) = self.read_column_chunk_with_checksum_at(offset)?;

        let compressed = if header.encryption_id == 0x01 {
            let encryption = header.encryption.as_ref().ok_or(Error::InvalidColumnChunkHeader)?;
            let key = master_key.ok_or(Error::KeyDerivationFailed)?;
            let mut ciphertext_with_tag = payload;
            ciphertext_with_tag.extend_from_slice(&encryption.auth_tag);
            let cipher = Cipher::new(key);
            cipher.decrypt(&encryption.nonce, &ciphertext_with_tag)?
        } else {
            payload
        };

        let decompressed = compression::decompress(
            CompressionId::try_from(header.compression_id)
                .map_err(|_| Error::UnknownCompression { id: header.compression_id })?,
            &compressed,
        )?;

        if decompressed.len() != header.uncompressed_size as usize {
            return Err(Error::InvalidColumnChunkHeader);
        }

        let decoded = encoding::decode(
            EncodingId::try_from(header.encoding_id)
                .map_err(|_| Error::UnknownEncoding { id: header.encoding_id })?,
            &decompressed,
        )?;

        let computed_checksum = integrity::crc32_bytes(&decoded);
        if computed_checksum != stored_checksum {
            return Err(Error::ChunkChecksumMismatch);
        }

        Ok(decoded)
    }

    fn read_column_chunk_with_checksum_at(
        &mut self,
        offset: u64,
    ) -> Result<(ColumnChunkHeader, Vec<u8>, u32)> {
        let (header, payload) = self.read_column_chunk_at(offset)?;

        let mut checksum_bytes = [0u8; 4];
        self.reader
            .read_exact(&mut checksum_bytes)
            .map_err(|_| Error::FileTooSmall { file_size: self.file_size })?;
        let checksum = u32::from_le_bytes(checksum_bytes);

        Ok((header, payload, checksum))
    }
}

/// Represents metadata for a single row group read.
#[derive(Debug, Clone)]
pub struct RowGroupMetadata {
    /// Byte offset from file start
    pub offset: u64,
    /// Number of rows in this group
    pub row_count: u32,
    /// Number of columns
    pub column_count: u16,
}

/// Column reader for partial/subset reads.
pub struct ColumnReader {
    /// Selected column indices
    column_indices: Vec<usize>,
    /// Row filter predicate (optional, for pushdown)
    row_filter: Option<Box<dyn Fn(u32) -> bool>>,
}

impl ColumnReader {
    /// Creates a column reader for specific columns.
    pub fn new(column_indices: Vec<usize>) -> Self {
        Self {
            column_indices,
            row_filter: None,
        }
    }

    /// Returns selected column indices.
    pub fn columns(&self) -> &[usize] {
        &self.column_indices
    }

    /// Sets a row filter predicate for predicate pushdown.
    pub fn with_row_filter(mut self, filter: Box<dyn Fn(u32) -> bool>) -> Self {
        self.row_filter = Some(filter);
        self
    }

    /// Returns whether a row filter is set.
    pub fn has_row_filter(&self) -> bool {
        self.row_filter.is_some()
    }
}