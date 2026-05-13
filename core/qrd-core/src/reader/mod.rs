//! File reader for QRD format.
//!
//! Implements footer-first parsing and optional partial column reads.

use crate::error::{Error, Result};
use crate::file_footer::{decode_footer_envelope, FooterContent, FOOTER_LENGTH_FIELD_SIZE};
use crate::file_header::{FileHeader, FILE_HEADER_SIZE};
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

    /// Reads a column chunk from the file at given offset.
    /// Returns (ColumnChunkHeader, raw_chunk_bytes).
    /// Note: encrypted column chunks are not automatically decrypted here.
    pub fn read_column_chunk_at(&mut self, offset: u64) -> Result<(crate::column_chunk::ColumnChunkHeader, Vec<u8>)> {
        self.reader
            .seek(SeekFrom::Start(offset))
            .map_err(|_| Error::FileTooSmall { file_size: self.file_size })?;

        let mut base_buf = vec![0u8; crate::column_chunk::COLUMN_CHUNK_HEADER_BASE_SIZE];
        self.reader
            .read_exact(&mut base_buf)
            .map_err(|_| Error::FileTooSmall { file_size: self.file_size })?;

        let (header, _) = crate::column_chunk::ColumnChunkHeader::parse(&base_buf)
            .map_err(|_| Error::InvalidColumnChunkHeader)?;

        let mut payload = vec![0u8; header.compressed_size as usize];
        self.reader
            .read_exact(&mut payload)
            .map_err(|_| Error::FileTooSmall { file_size: self.file_size })?;

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