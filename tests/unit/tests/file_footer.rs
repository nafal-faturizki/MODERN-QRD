use qrd_core::{decode_footer_body, encode_footer_envelope, Error};

#[test]
fn encodes_and_decodes_footer_envelope() {
    let body = b"footer-body-bytes";
    let envelope = encode_footer_envelope(body).unwrap();

    assert_eq!(decode_footer_body(&envelope).unwrap(), body);
}

#[test]
fn rejects_corrupt_footer_checksum() {
    let body = b"footer-body-bytes";
    let mut envelope = encode_footer_envelope(body).unwrap();
    envelope[0] ^= 0x01;

    assert!(matches!(decode_footer_body(&envelope), Err(Error::FooterChecksumMismatch)));
}
