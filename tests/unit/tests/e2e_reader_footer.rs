use qrd_core::{FileHeader, encode_footer_envelope, FileReader};
use std::io::Cursor;

#[test]
fn e2e_reader_parses_footer_metadata() {
    // Build a fake file: header + payload + footer_envelope
    let header = FileHeader::new(1, 0, [0u8; 8], 0, 0, 0);
    let header_bytes = header.to_bytes();

    // Footer body: rg_count (u16), total_rows (u64), schema_version (u16), then per-rg: offset(u64), row_count(u32)
    let rg_count: u16 = 3;
    let total_rows: u64 = 7;
    let schema_version: u16 = 1;

    let mut body = Vec::new();
    body.extend_from_slice(&rg_count.to_le_bytes());
    body.extend_from_slice(&total_rows.to_le_bytes());
    body.extend_from_slice(&schema_version.to_le_bytes());

    // Add three row groups (offset placeholders + row counts)
    body.extend_from_slice(&0u64.to_le_bytes());
    body.extend_from_slice(&3u32.to_le_bytes());
    body.extend_from_slice(&0u64.to_le_bytes());
    body.extend_from_slice(&3u32.to_le_bytes());
    body.extend_from_slice(&0u64.to_le_bytes());
    body.extend_from_slice(&1u32.to_le_bytes());

    let footer_envelope = encode_footer_envelope(&body).expect("encode footer failed");

    // Create full file bytes
    let mut file_bytes = Vec::new();
    file_bytes.extend_from_slice(&header_bytes);
    file_bytes.extend_from_slice(b"PAYLOAD_PLACEHOLDER");
    file_bytes.extend_from_slice(&footer_envelope);

    let cursor = Cursor::new(file_bytes);
    let reader = FileReader::new(cursor).expect("FileReader::new failed");

    assert_eq!(reader.total_rows(), total_rows);
    assert_eq!(reader.footer_row_group_count(), rg_count);
}
