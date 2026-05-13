use qrd_core::integrity::{crc32_bytes, crc32_matches};

#[test]
fn computes_crc32_for_standard_vector() {
    assert_eq!(crc32_bytes(b"123456789"), 0xcbf4_3926);
}

#[test]
fn matches_expected_crc32() {
    let bytes = b"qrd";
    let checksum = crc32_bytes(bytes);

    assert!(crc32_matches(bytes, checksum));
    assert!(!crc32_matches(bytes, checksum ^ 0xFFFF_FFFF));
}
