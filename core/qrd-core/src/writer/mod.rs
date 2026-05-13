//! Streaming writer for QRD format.
//!
//! Provides row-by-row ingestion with automatic row group flushing,
//! pipeline-based encoding/compression/encryption, and footer generation.

use crate::error::{Error, Result};
use crate::file_footer::{encode_footer_envelope, FooterContent, FooterRowGroupEntry};
use crate::file_header::FileHeader;
use crate::schema::Schema;
use std::io::{Seek, SeekFrom, Write};
use std::time::{SystemTime, UNIX_EPOCH};

/// Streaming writer for QRD files.
///
/// Manages row-by-row ingestion, automatic row group flushing,
/// and footer generation.
pub struct StreamingWriter<W: Write + Seek> {
    writer: W,
    schema: Schema,
    /// File header written at start
    file_header: FileHeader,
    /// Current row group buffer
    pub current_row_group: RowGroupBuffer,
    /// Accumulated row group offsets for footer
    row_group_offsets: Vec<RowGroupMetadata>,
    /// Number of rows flushed so far
    pub total_rows_written: u64,
    /// Row group size limit (rows per group)
    row_group_size: u32,
    /// Whether finish() has been called
    is_finished: bool,
}

/// Buffer for accumulating rows before flushing.
#[derive(Debug)]
pub struct RowGroupBuffer {
    /// Rows accumulated so far (each row is a Vec of column values)
    rows: Vec<Vec<Vec<u8>>>,
    /// Column count expected per row
    column_count: u16,
    /// Current row count
    pub row_count: u32,
}

/// Metadata for flushed row group.
#[derive(Clone)]
struct RowGroupMetadata {
    /// Byte offset where row group starts
    offset: u64,
    /// Number of rows in this group
    row_count: u32,
}

impl<W: Write + Seek> StreamingWriter<W> {
    /// Creates a new streaming writer.
    ///
    /// Writes file header immediately.
    pub fn new(mut writer: W, schema: Schema) -> Result<Self> {
        let schema_id = schema.schema_id().map_err(Error::Schema)?;

        let created_at_sec = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as u32)
            .unwrap_or(0);

        let file_header = FileHeader::new(1, 0, schema_id, 0, 0, created_at_sec);
        writer
            .seek(SeekFrom::Start(0))
            .map_err(|_| Error::FileTooSmall { file_size: 0 })?;
        writer
            .write_all(&file_header.to_bytes())
            .map_err(|_| Error::FileTooSmall { file_size: 0 })?;

        let field_count = u16::try_from(schema.fields().len())
            .map_err(|_| Error::FileTooSmall { file_size: 0 })?;

        Ok(Self {
            writer,
            schema,
            file_header,
            current_row_group: RowGroupBuffer::new(field_count),
            row_group_offsets: Vec::new(),
            total_rows_written: 0,
            row_group_size: 128_000,
            is_finished: false,
        })
    }

    /// Sets row group size limit (rows per group).
    pub fn with_row_group_size(mut self, size: u32) -> Self {
        self.row_group_size = size;
        self
    }

    /// Adds a row (as column values) to the current row group.
    /// Automatically flushes if row group is full.
    pub fn write_row(&mut self, row: Vec<Vec<u8>>) -> Result<()> {
        if self.is_finished {
            return Err(Error::FileTooSmall { file_size: 0 });
        }

        self.current_row_group.add_row(row)?;
        self.total_rows_written += 1;

        if self.current_row_group.row_count >= self.row_group_size {
            self.flush_row_group()?;
        }

        Ok(())
    }

    /// Manually flushes current row group buffer.
    pub fn flush_row_group(&mut self) -> Result<()> {
        if self.current_row_group.row_count == 0 {
            return Ok(());
        }

        let offset = self
            .writer
            .seek(SeekFrom::Current(0))
            .map_err(|_| Error::FileTooSmall { file_size: 0 })?;
        let row_count = self.current_row_group.row_count;
        let column_count = u16::try_from(self.schema.fields().len())
            .map_err(|_| Error::FileTooSmall { file_size: 0 })?;

        self.row_group_offsets.push(RowGroupMetadata {
            offset,
            row_count,
        });

        self.current_row_group = RowGroupBuffer::new(column_count);

        Ok(())
    }

    /// Finalizes the file: flushes any pending row group and writes footer.
    pub fn finish(mut self) -> Result<W> {
        if self.is_finished {
            return Err(Error::FileTooSmall { file_size: 0 });
        }

        if self.current_row_group.row_count > 0 {
            self.flush_row_group()?;
        }

        self.is_finished = true;

        self.file_header.row_group_count = self.row_group_offsets.len() as u32;
        self.writer
            .seek(SeekFrom::Start(0))
            .map_err(|_| Error::FileTooSmall { file_size: 0 })?;
        self.writer
            .write_all(&self.file_header.to_bytes())
            .map_err(|_| Error::FileTooSmall { file_size: 0 })?;

        self.writer
            .seek(SeekFrom::End(0))
            .map_err(|_| Error::FileTooSmall { file_size: 0 })?;

        let footer_content = FooterContent {
            footer_version: crate::file_footer::FOOTER_VERSION,
            schema: self.schema.clone(),
            row_groups: self
                .row_group_offsets
                .iter()
                .map(|entry| FooterRowGroupEntry {
                    byte_offset: entry.offset,
                    row_count: entry.row_count,
                })
                .collect(),
            statistics_flag: 0x00,
            statistics_bytes: Vec::new(),
            encryption_metadata: None,
            schema_signature: None,
            file_metadata: Vec::new(),
        };

        let footer_bytes = footer_content.to_bytes()?;
        let footer_envelope = encode_footer_envelope(&footer_bytes)?;

        self.writer
            .write_all(&footer_envelope)
            .map_err(|_| Error::FileTooSmall { file_size: 0 })?;

        Ok(self.writer)
    }
}

impl RowGroupBuffer {
    fn new(column_count: u16) -> Self {
        Self {
            rows: Vec::new(),
            column_count,
            row_count: 0,
        }
    }

    fn add_row(&mut self, row: Vec<Vec<u8>>) -> Result<()> {
        if row.len() as u16 != self.column_count {
            return Err(crate::error::Error::SchemaIdMismatch);
        }

        self.rows.push(row);
        self.row_count += 1;
        Ok(())
    }
}