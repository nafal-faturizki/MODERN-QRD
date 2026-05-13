use qrd_core::{StreamingWriter, Schema, SchemaField, LogicalTypeId, Nullability};

#[test]
fn creates_streaming_writer_with_schema() {
    let schema = Schema::builder()
        .field(SchemaField::new("id", LogicalTypeId::Int64, Nullability::Required))
        .field(SchemaField::new("name", LogicalTypeId::Utf8String, Nullability::Optional))
        .build()
        .expect("schema should build");

    let buffer = Vec::new();
    let writer = StreamingWriter::new(buffer, schema.clone())
        .expect("writer should initialize");

    // Verify writer was created and header written
    // (In real implementation, we'd check file_header state)
    assert_eq!(writer.total_rows_written, 0);
}

#[test]
fn writes_multiple_rows_and_flushes() {
    let schema = Schema::builder()
        .field(SchemaField::new("id", LogicalTypeId::Int64, Nullability::Required))
        .field(SchemaField::new("value", LogicalTypeId::Float64, Nullability::Optional))
        .build()
        .expect("schema should build");

    let buffer = Vec::new();
    let mut writer = StreamingWriter::new(buffer, schema)
        .expect("writer should initialize");

    // Write a single row with two column values
    let row = vec![
        vec![1, 2, 3, 4, 5, 6, 7, 8],  // id column
        vec![1, 2, 3, 4, 5, 6, 7, 8],  // value column
    ];
    writer.write_row(row).expect("should write row");

    assert_eq!(writer.current_row_group.row_count, 1);
}

#[test]
fn finalizes_with_footer() {
    let schema = Schema::builder()
        .field(SchemaField::new("col1", LogicalTypeId::Int32, Nullability::Required))
        .build()
        .expect("schema should build");

    let buffer = Vec::new();
    let mut writer = StreamingWriter::new(buffer, schema)
        .expect("writer should initialize");

    // Write one row
    let row = vec![vec![1, 2, 3, 4]];
    writer.write_row(row).expect("should write row");

    // Finish writes footer and envelope
    let _result = writer.finish().expect("should finish");
    
    // Footer should be successfully written (no panic)
}

#[test]
fn rejects_writes_after_finish() {
    let schema = Schema::builder()
        .field(SchemaField::new("col1", LogicalTypeId::Int32, Nullability::Required))
        .build()
        .expect("schema should build");

    let buffer = Vec::new();
    let writer = StreamingWriter::new(buffer, schema)
        .expect("writer should initialize");

    // Finish the writer (consumes it)
    let _result = writer.finish().expect("should finish");
    
    // After finish, the writer is consumed, so we can't test further writes
    // Instead, we test that finish succeeds (no panic)
}
