use qrd_core::{Error, FileHeader};

#[test]
fn serializes_and_parses_file_header() {
    let header = FileHeader::new(1, 0, [1, 2, 3, 4, 5, 6, 7, 8], 0x1122_3344, 3, 1_700_000_000);
    let bytes = header.to_bytes();

    assert_eq!(bytes.len(), 32);
    assert_eq!(&bytes[0..4], &[0x51, 0x52, 0x44, 0x01]);
    assert_eq!(FileHeader::parse(&bytes).unwrap(), header);
}

#[test]
fn rejects_invalid_magic_and_checksum() {
    let header = FileHeader::new(1, 0, [0; 8], 0, 0, 0);
    let mut bytes = header.to_bytes();

    bytes[0] = 0x00;
    assert!(matches!(FileHeader::parse(&bytes), Err(Error::InvalidMagic)));

    let mut bytes = header.to_bytes();
    bytes[28] ^= 0xFF;
    assert!(matches!(FileHeader::parse(&bytes), Err(Error::HeaderChecksumMismatch)));
}

#[test]
fn validates_supported_major_version() {
    let header = FileHeader::new(2, 0, [0; 8], 0, 0, 0);

    assert!(matches!(
        header.validate_major_version(1),
        Err(Error::UnsupportedMajorVersion { major_version: 2 })
    ));
}
