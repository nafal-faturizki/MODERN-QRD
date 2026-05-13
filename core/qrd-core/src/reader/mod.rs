//! File reader for QRD format.
//! 
//! Implements footer-first parsing and optional partial column reads.

use crate::error::{Error, Result};
use crate::file_header::FileHeader;
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
}

impl<R: Read + Seek> FileReader<R> {
    /// Creates a new file reader from a seekable readable source.
    /// Reads and validates file header and parses footer.
    pub fn new(mut reader: R) -> Result<Self> {
        // Determine file size by seeking to end
        let file_size = reader.seek(SeekFrom::End(0)).map_err(|_| Error::FileTooSmall { file_size: 0 })?;

        if file_size < 32 {
            return Err(Error::FileTooSmall { file_size });
        }

        // Read file header (first 32 bytes)
        reader.seek(SeekFrom::Start(0)).map_err(|_| Error::FileTooSmall { file_size })?;
        let mut header_bytes = [0u8; 32];
        reader.read_exact(&mut header_bytes)
            .map_err(|_| Error::FileTooSmall { file_size })?;

        let file_header = FileHeader::parse(&header_bytes)?;
        file_header.validate_major_version(1)?;

        // Read entire file into memory to parse footer envelope (small files expected)
        reader.seek(SeekFrom::Start(0)).map_err(|_| Error::FileTooSmall { file_size })?;
        let mut all_bytes = Vec::with_capacity(file_size as usize);
        reader.read_to_end(&mut all_bytes)
            .map_err(|_| Error::FileTooSmall { file_size })?;

        // Decode footer body and parse basic metadata
        let footer_body = crate::file_footer::decode_footer_body(&all_bytes)?;

        // footer format (writer): [rg_count:u16][total_rows:u64][schema_version:u16][...]
        if footer_body.len() < 2 + 8 + 2 {
            return Err(Error::InvalidFooterLength { footer_len: footer_body.len() as u32, file_size });
        }

        let mut idx = 0usize;
        let rg_count = u16::from_le_bytes([footer_body[idx], footer_body[idx+1]]);
        idx += 2;

        let mut total_rows_bytes = [0u8; 8];
        total_rows_bytes.copy_from_slice(&footer_body[idx..idx+8]);
        let total_rows = u64::from_le_bytes(total_rows_bytes);
        idx += 8;

        let schema_version = u16::from_le_bytes([footer_body[idx], footer_body[idx+1]]);

        // Build a minimal schema with version (fields unknown)
        let schema = Schema::builder().schema_version(schema_version).build().map_err(|_| Error::SchemaIdMismatch)?;

        // Use extracted footer metadata
        Ok(Self {
            reader,
            file_header,
            schema,
            file_size,
            footer_total_rows: total_rows,
            footer_row_group_count: rg_count,
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
        // Prefer footer-provided row group count if present
        self.footer_row_group_count as u32
    }

    /// Returns row group count as read from footer.
    pub fn footer_row_group_count(&self) -> u16 {
        self.footer_row_group_count
    }

    /// Reads a column chunk from the file at given offset.
    /// Returns (ColumnChunkHeader, raw_chunk_bytes).
    /// Note: encrypted column chunks are not automatically decrypted here.
    pub fn read_column_chunk_at(&mut self, offset: u64) -> Result<(crate::column_chunk::ColumnChunkHeader, Vec<u8>)> {
        // Seek to offset
        self.reader.seek(SeekFrom::Start(offset)).map_err(|_| Error::FileTooSmall { file_size: self.file_size })?;

        // Read enough bytes for base header
        let mut base_buf = vec![0u8; crate::column_chunk::COLUMN_CHUNK_HEADER_BASE_SIZE];
        self.reader.read_exact(&mut base_buf).map_err(|_| Error::FileTooSmall { file_size: self.file_size })?;

        // Parse header (may need extra bytes for encryption section)
        // To handle this, read a larger buffer from offset
        // Read up to compressed_size after parsing base header
        let mut lookahead = Vec::new();
        // Start with base_buf
        lookahead.extend_from_slice(&base_buf);

        // Try parsing; if encrypted, ColumnChunkHeader::parse will check lengths
        let (header, consumed) = crate::column_chunk::ColumnChunkHeader::parse(&lookahead)
            .map_err(|_| Error::InvalidColumnChunkHeader)?;

        // If parse didn't include encryption section, consumed equals base size
        // Now read compressed payload
        let mut payload = vec![0u8; header.compressed_size as usize];
        self.reader.read_exact(&mut payload).map_err(|_| Error::FileTooSmall { file_size: self.file_size })?;

        Ok((header, payload))
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
