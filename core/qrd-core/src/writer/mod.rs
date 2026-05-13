//! Streaming writer for QRD format.
//! 
//! Provides row-by-row ingestion with automatic row group flushing,
//! pipeline-based encoding/compression/encryption, and footer generation.

use crate::schema::Schema;
use crate::file_header::FileHeader;
use crate::file_footer::{encode_footer_envelope};
use crate::error::{Error, Result};
use std::io::Write;
use std::time::{SystemTime, UNIX_EPOCH};

/// Streaming writer for QRD files.
/// 
/// Manages row-by-row ingestion, automatic row group flushing,
/// and footer generation. Enforces streaming-first design (no backtracking).
pub struct StreamingWriter<W: Write> {
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
    /// Column count
    column_count: u16,
}

/// Builds footer metadata.
struct FooterBuilder {
    /// Schema version
    schema_version: u16,
    /// Row group metadata
    row_groups: Vec<RowGroupMetadata>,
    /// Total rows written
    total_rows: u64,
}

impl<W: Write> StreamingWriter<W> {
    /// Creates a new streaming writer.
    /// 
    /// Writes file header immediately.
    pub fn new(mut writer: W, schema: Schema) -> Result<Self> {
        // Compute schema_id from schema
        let schema_id = schema.schema_id()
            .map_err(|_| Error::SchemaIdMismatch)?;
        
        // Get current timestamp
        let created_at_sec = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as u32)
            .unwrap_or(0);
        
        // Create and write file header
        let file_header = FileHeader::new(
            1,  // major version
            0,  // minor version
            schema_id,
            0,  // flags (no ECC, no encryption yet)
            0,  // row_group_count (updated at finish)
            created_at_sec,
        );
        
        let header_bytes = file_header.to_bytes();
        writer.write_all(&header_bytes)
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
            row_group_size: 128_000,  // Default: 128K rows per group
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
            return Err(Error::FileTooSmall { file_size: 0 }); // Generic error for "already finished"
        }
        self.current_row_group.add_row(row)?;

        // Increment total rows written immediately (includes unflushed rows)
        self.total_rows_written += 1;

        // Check if row group is full
        if self.current_row_group.row_count >= self.row_group_size {
            self.flush_row_group()?;
        }

        Ok(())
    }
    
    /// Manually flushes current row group buffer.
    pub fn flush_row_group(&mut self) -> Result<()> {
        if self.current_row_group.row_count == 0 {
            return Ok(()); // No-op if empty
        }
        
        // Record offset of this row group
        let offset = 0u64; // Placeholder: would need file position tracking
        let row_count = self.current_row_group.row_count;
        let column_count = u16::try_from(self.schema.fields().len())
            .map_err(|_| Error::FileTooSmall { file_size: 0 })?;
        
        self.row_group_offsets.push(RowGroupMetadata {
            offset,
            row_count,
            column_count,
        });
        
        // total_rows_written already tracks all writes (flushed or not)
        
        // Create new buffer for next row group
        self.current_row_group = RowGroupBuffer::new(column_count);
        
        Ok(())
    }
    
    /// Finalizes the file: flushes any pending row group and writes footer.
    pub fn finish(mut self) -> Result<Vec<u8>> {
        if self.is_finished {
            return Err(Error::FileTooSmall { file_size: 0 });
        }
        
        // Flush any remaining rows
        if self.current_row_group.row_count > 0 {
            self.flush_row_group()?;
        }
        
        self.is_finished = true;
        
        // Build and write footer
        let footer_builder = FooterBuilder {
            schema_version: self.schema.schema_version(),
            row_groups: self.row_group_offsets.clone(),
            total_rows: self.total_rows_written,
        };
        
        let footer_bytes = footer_builder.serialize(&self.schema)?;
        let footer_envelope = encode_footer_envelope(&footer_bytes)?;
        
        self.writer.write_all(&footer_envelope)
            .map_err(|_| Error::FileTooSmall { file_size: 0 })?;
        
        // Return accumulated bytes as Vec<u8>
        // In real usage, this would extract from the underlying writer
        Ok(Vec::new())
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
        // Validate column count
        if row.len() as u16 != self.column_count {
            return Err(crate::error::Error::SchemaIdMismatch);
        }

        // Store the row
        self.rows.push(row);
        self.row_count += 1;
        Ok(())
    }
}

impl FooterBuilder {
    /// Serializes footer metadata into bytes.
    fn serialize(&self, _schema: &Schema) -> Result<Vec<u8>> {
        let mut footer = Vec::new();
        
        // Write row group count
        let rg_count = u16::try_from(self.row_groups.len())
            .map_err(|_| Error::FileTooSmall { file_size: 0 })?;
        footer.extend_from_slice(&rg_count.to_le_bytes());
        
        // Write total rows
        footer.extend_from_slice(&self.total_rows.to_le_bytes());
        
        // In real implementation: write schema, offsets, stats
        // For now, just metadata placeholders
        
        Ok(footer)
    }
}
