use qrd_core::schema::{LogicalTypeId, Nullability, Schema, SchemaField};
use sha2::{Digest, Sha256};

#[test]
fn serializes_minimal_footer_schema_section() {
    let schema = Schema::builder()
        .field(SchemaField::new("device_id", LogicalTypeId::Enum, Nullability::Required))
        .build()
        .unwrap();

    let bytes = schema.serialize_footer_schema_section().unwrap();

    let expected = vec![
        0x16, 0x00, 0x00, 0x00,
        0x01, 0x00,
        0x01, 0x00,
        0x09, 0x00,
        b'd', b'e', b'v', b'i', b'c', b'e', b'_', b'i', b'd',
        0x21,
        0x00,
        0x00,
        0x00,
        0x00,
        0x00, 0x00,
    ];

    assert_eq!(bytes, expected);
}

#[test]
fn schema_id_is_first_eight_bytes_of_sha256() {
    let schema = Schema::builder()
        .field(SchemaField::new("device_id", LogicalTypeId::Enum, Nullability::Required))
        .build()
        .unwrap();

    let bytes = schema.serialize_footer_schema_section().unwrap();
    let expected_fingerprint: [u8; 32] = Sha256::digest(&bytes[4..]).into();
    let expected_schema_id: [u8; 8] = expected_fingerprint[..8].try_into().unwrap();

    assert_eq!(schema.schema_fingerprint().unwrap(), expected_fingerprint);
    assert_eq!(schema.schema_id().unwrap(), expected_schema_id);
}
