use qrd_core::{FileHeader, ColumnChunkHeader, encode_footer_envelope, FileReader};
use std::io::Cursor;

#[test]
fn e2e_reader_reads_column_chunk_at_offset() {
    // Build a fake column chunk header and payload
    let mut chunk_header = ColumnChunkHeader::new_plain(
        0x00, // encoding plain
        0x00, // compression none
        0x00, // flags
        11,   // compressed_size
        11,   // uncompressed_size
        0,    // null_count
        3,    // row_count_chunk
        100,  // row_offset placeholder
    );

    let header_bytes = chunk_header.to_bytes().expect("to_bytes");
    let payload = b"hello world".to_vec();

    // File layout: header(32) + some payload + column_chunk_header + payload + footer
    let header = FileHeader::new(1, 0, [0u8;8], 0, 0, 0);
    let header_bytes_32 = header.to_bytes();

    let mut file_bytes = Vec::new();
    file_bytes.extend_from_slice(&header_bytes_32);
    file_bytes.extend_from_slice(b"SOME_PAYLOAD");

    let chunk_offset = file_bytes.len() as u64;
    file_bytes.extend_from_slice(&header_bytes);
    file_bytes.extend_from_slice(&payload);

    // Simple footer body minimal
    let mut body = Vec::new();
    body.extend_from_slice(&0u16.to_le_bytes());
    body.extend_from_slice(&0u64.to_le_bytes());
    body.extend_from_slice(&1u16.to_le_bytes());
    let footer = encode_footer_envelope(&body).unwrap();
    file_bytes.extend_from_slice(&footer);

    let cursor = Cursor::new(file_bytes);
    let mut reader = FileReader::new(cursor).expect("FileReader::new");

    let (read_header, read_payload) = reader.read_column_chunk_at(chunk_offset).expect("read column chunk");
    assert_eq!(read_header.encoding_id, chunk_header.encoding_id);
    assert_eq!(read_payload, payload);
}
