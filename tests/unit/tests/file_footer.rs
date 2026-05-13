use qrd_core::{
    decode_footer_body,
    encode_footer_envelope,
    Error,
    FooterContent,
    FooterRowGroupEntry,
    LogicalTypeId,
    Nullability,
    Schema,
    SchemaField,
};

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

#[test]
fn serializes_and_parses_footer_content() {
    let schema = Schema::builder()
        .field(SchemaField::new("device_id", LogicalTypeId::Enum, Nullability::Required))
        .build()
        .unwrap();

    let footer = FooterContent {
        footer_version: 1,
        schema: schema.clone(),
        row_groups: vec![FooterRowGroupEntry { byte_offset: 32, row_count: 5 }],
        statistics_flag: 0,
        statistics_bytes: Vec::new(),
        encryption_metadata: None,
        schema_signature: None,
        file_metadata: Vec::new(),
    };

    let body = footer.to_bytes().unwrap();
    let parsed = FooterContent::parse(&body, 0).unwrap();

    assert_eq!(parsed.schema, schema);
    assert_eq!(parsed.row_group_count(), 1);
    assert_eq!(parsed.total_rows(), 5);
}
