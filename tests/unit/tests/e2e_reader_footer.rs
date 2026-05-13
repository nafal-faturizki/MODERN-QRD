use qrd_core::{
    encode_footer_envelope, FileHeader, FileReader, FooterContent, FooterRowGroupEntry,
    LogicalTypeId, Nullability, Schema, SchemaField,
};
use std::io::Cursor;

#[test]
fn e2e_reader_parses_footer_metadata() {
    let schema = Schema::builder()
        .field(SchemaField::new("device_id", LogicalTypeId::Enum, Nullability::Required))
        .build()
        .expect("schema should build");

    let schema_id = schema.schema_id().expect("schema id should compute");

    let header = FileHeader::new(1, 0, schema_id, 0, 2, 0);
    let header_bytes = header.to_bytes();

    let footer = FooterContent {
        footer_version: 1,
        schema,
        row_groups: vec![
            FooterRowGroupEntry { byte_offset: 32, row_count: 3 },
            FooterRowGroupEntry { byte_offset: 32, row_count: 4 },
        ],
        statistics_flag: 0,
        statistics_bytes: Vec::new(),
        encryption_metadata: None,
        schema_signature: None,
        file_metadata: Vec::new(),
    };

    let body = footer.to_bytes().expect("footer serialization should succeed");
    let footer_envelope = encode_footer_envelope(&body).expect("encode footer failed");

    let mut file_bytes = Vec::new();
    file_bytes.extend_from_slice(&header_bytes);
    file_bytes.extend_from_slice(b"PAYLOAD_PLACEHOLDER");
    file_bytes.extend_from_slice(&footer_envelope);

    let cursor = Cursor::new(file_bytes);
    let reader = FileReader::new(cursor).expect("FileReader::new failed");

    assert_eq!(reader.total_rows(), 7);
    assert_eq!(reader.footer_row_group_count(), 2);
}