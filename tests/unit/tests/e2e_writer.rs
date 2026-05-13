use qrd_core::{
    ColumnChunkHeader, FileReader, LogicalTypeId, Nullability, SchemaBuilder, SchemaField,
    StreamingWriter,
};
use qrd_core::row_group::ROW_GROUP_HEADER_SIZE;
use std::io::Cursor;

#[test]
fn e2e_write_schema_creation() {
    // Create a simple schema with 3 fields
    let schema = SchemaBuilder::new()
        .field(SchemaField::new("id", LogicalTypeId::Int32, Nullability::Required))
        .field(SchemaField::new("name", LogicalTypeId::Utf8String, Nullability::Optional))
        .field(SchemaField::new("score", LogicalTypeId::Float32, Nullability::Optional))
        .build()
        .expect("Schema build failed");

    assert_eq!(schema.fields().len(), 3);
    assert_eq!(schema.schema_version(), 1);
}

#[test]
fn e2e_writer_creation() {
    let schema = SchemaBuilder::new()
        .field(SchemaField::new("id", LogicalTypeId::Int32, Nullability::Required))
        .build()
        .expect("Schema build failed");

    let buffer = Cursor::new(Vec::new());
    let writer = StreamingWriter::new(buffer, schema);
    
    assert!(writer.is_ok(), "Writer creation should succeed");
}

#[test]
fn e2e_writer_basic_row_write() {
    let schema = SchemaBuilder::new()
        .field(SchemaField::new("id", LogicalTypeId::Int32, Nullability::Required))
        .build()
        .expect("Schema build failed");

    let buffer = Cursor::new(Vec::new());
    let mut writer = StreamingWriter::new(buffer, schema)
        .expect("Writer creation failed");

    // Prepare a row: single i32 value encoded as bytes
    // Row is Vec of column values, where each column is Vec<u8>
    let id_value: i32 = 42;
    let row = vec![id_value.to_le_bytes().to_vec()];

    let result = writer.write_row(row);
    assert!(result.is_ok(), "Write row should succeed");
}

#[test]
fn e2e_writer_multiple_rows() {
    let schema = SchemaBuilder::new()
        .field(SchemaField::new("id", LogicalTypeId::Int32, Nullability::Required))
        .field(SchemaField::new("value", LogicalTypeId::Int64, Nullability::Required))
        .build()
        .expect("Schema build failed");

    let buffer = Cursor::new(Vec::new());
    let mut writer = StreamingWriter::new(buffer, schema)
        .expect("Writer creation failed")
        .with_row_group_size(1000);

    // Write 10 rows
    for i in 0..10 {
        let id = (i as i32).to_le_bytes().to_vec();
        let value = (i as i64).to_le_bytes().to_vec();
        let row = vec![id, value];
        
        let result = writer.write_row(row);
        assert!(result.is_ok(), "Write row {} should succeed", i);
    }

    assert_eq!(writer.total_rows_written, 10);
}

#[test]
fn e2e_writer_finish() {
    let schema = SchemaBuilder::new()
        .field(SchemaField::new("id", LogicalTypeId::Int32, Nullability::Required))
        .build()
        .expect("Schema build failed");

    let buffer = Cursor::new(Vec::new());
    let mut writer = StreamingWriter::new(buffer, schema)
        .expect("Writer creation failed");

    // Write one row
    let row = vec![42i32.to_le_bytes().to_vec()];
    writer.write_row(row).expect("Write row failed");

    // Finish writing
    let result = writer.finish();
    assert!(result.is_ok(), "Finish should succeed");
}

#[test]
fn e2e_writer_reject_wrong_column_count() {
    let schema = SchemaBuilder::new()
        .field(SchemaField::new("id", LogicalTypeId::Int32, Nullability::Required))
        .field(SchemaField::new("name", LogicalTypeId::Utf8String, Nullability::Required))
        .build()
        .expect("Schema build failed");

    let buffer = Cursor::new(Vec::new());
    let mut writer = StreamingWriter::new(buffer, schema)
        .expect("Writer creation failed");

    // Wrong: only 1 column instead of 2
    let row = vec![42i32.to_le_bytes().to_vec()];
    
    let result = writer.write_row(row);
    assert!(result.is_err(), "Write row with wrong column count should fail");
}

#[test]
fn e2e_writer_prevents_write_after_finish() {
    let schema = SchemaBuilder::new()
        .field(SchemaField::new("id", LogicalTypeId::Int32, Nullability::Required))
        .build()
        .expect("Schema build failed");

    let buffer = Cursor::new(Vec::new());
    let mut writer = StreamingWriter::new(buffer, schema)
        .expect("Writer creation failed");

    // Write and finish
    let row = vec![42i32.to_le_bytes().to_vec()];
    writer.write_row(row).expect("First write should succeed");
    
    // finish() takes ownership, so this will consume the writer
    let _result = writer.finish().expect("Finish should succeed");
    
    // After finish, writer cannot be used anymore (it's moved)
    // So this test just verifies that finish() succeeds
}

#[test]
fn e2e_writer_reader_roundtrip_decoded_first_chunk() {
    let schema = SchemaBuilder::new()
        .field(SchemaField::new("id", LogicalTypeId::Int32, Nullability::Required))
        .build()
        .expect("Schema build failed");

    let buffer = Cursor::new(Vec::new());
    let mut writer = StreamingWriter::new(buffer, schema).expect("Writer creation failed");

    for value in [10i32, 20i32, 30i32] {
        writer
            .write_row(vec![value.to_le_bytes().to_vec()])
            .expect("write row should succeed");
    }

    let cursor = writer.finish().expect("finish should succeed");
    let mut reader = FileReader::new(Cursor::new(cursor.into_inner())).expect("reader should open");

    let row_group_offsets = reader.row_group_offsets();
    assert_eq!(row_group_offsets.len(), 1);

    let chunk_offset = row_group_offsets[0] + ROW_GROUP_HEADER_SIZE as u64;
    let decoded = reader
        .read_decoded_column_chunk_at(chunk_offset, None)
        .expect("decoded read should succeed");

    let expected = [10i32, 20i32, 30i32]
        .iter()
        .flat_map(|v| v.to_le_bytes())
        .collect::<Vec<u8>>();
    assert_eq!(decoded, expected);
}

#[test]
fn e2e_reader_detects_chunk_checksum_mismatch() {
    let schema = SchemaBuilder::new()
        .field(SchemaField::new("id", LogicalTypeId::Int32, Nullability::Required))
        .build()
        .expect("Schema build failed");

    let buffer = Cursor::new(Vec::new());
    let mut writer = StreamingWriter::new(buffer, schema).expect("Writer creation failed");
    writer
        .write_row(vec![1i32.to_le_bytes().to_vec()])
        .expect("write should succeed");

    let cursor = writer.finish().expect("finish should succeed");
    let mut bytes = cursor.into_inner();

    let row_group_offset = qrd_core::file_header::FILE_HEADER_SIZE as u64;
    let chunk_offset = row_group_offset + ROW_GROUP_HEADER_SIZE as u64;
    let header_start = chunk_offset as usize;

    let (header, consumed) = ColumnChunkHeader::parse(&bytes[header_start..]).expect("chunk header parse");
    let checksum_offset = header_start + consumed + header.compressed_size as usize;
    bytes[checksum_offset] ^= 0xFF;

    let mut reader = FileReader::new(Cursor::new(bytes)).expect("reader should open");
    let err = reader
        .read_decoded_column_chunk_at(chunk_offset, None)
        .expect_err("checksum mismatch should fail");

    assert!(matches!(err, qrd_core::Error::ChunkChecksumMismatch));
}
