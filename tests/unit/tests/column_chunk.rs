use qrd_core::{ChunkEncryptionMetadata, ColumnChunkHeader, Error};

#[test]
fn serializes_and_parses_plain_column_chunk_header() {
    let header = ColumnChunkHeader::new_plain(0x00, 0x01, 0x00, 128, 256, 2, 64, 9);
    let bytes = header.to_bytes().unwrap();

    assert_eq!(bytes.len(), 28);
    let (parsed, consumed) = ColumnChunkHeader::parse(&bytes).unwrap();

    assert_eq!(consumed, 28);
    assert_eq!(parsed, header);
}

#[test]
fn serializes_and_parses_encrypted_column_chunk_header() {
    let header = ColumnChunkHeader::new_encrypted(
        0x00,
        0x01,
        0x00,
        128,
        256,
        2,
        64,
        9,
        ChunkEncryptionMetadata {
            nonce: [7; 12],
            auth_tag: [9; 16],
            key_id: b"sensor-key".to_vec(),
        },
    );

    let bytes = header.to_bytes().unwrap();
    let (parsed, consumed) = ColumnChunkHeader::parse(&bytes).unwrap();

    assert_eq!(parsed, header);
    assert_eq!(consumed, bytes.len());
}

#[test]
fn rejects_unknown_encryption_id() {
    let mut bytes = vec![0u8; 28];
    bytes[2] = 0x7f;

    assert!(matches!(
        ColumnChunkHeader::parse(&bytes),
        Err(Error::UnknownEncryption { id: 0x7f })
    ));
}
