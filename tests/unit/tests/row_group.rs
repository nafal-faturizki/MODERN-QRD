use qrd_core::{Error, RowGroupHeader};

#[test]
fn serializes_and_parses_row_group_header() {
    let header = RowGroupHeader::new(42, 3, 0x00F0);
    let bytes = header.to_bytes();

    assert_eq!(bytes.len(), 12);
    assert_eq!(RowGroupHeader::parse(&bytes).unwrap(), header);
}

#[test]
fn rejects_row_group_header_checksum_mismatch() {
    let header = RowGroupHeader::new(42, 3, 0x00F0);
    let mut bytes = header.to_bytes();
    bytes[0] ^= 0x01;

    assert!(matches!(RowGroupHeader::parse(&bytes), Err(Error::InvalidRowGroupHeader)));
}
