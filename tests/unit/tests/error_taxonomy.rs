use qrd_core::Error;

#[test]
fn formats_schema_mismatch_error() {
    let error = Error::SchemaIdMismatch;
    assert_eq!(error.to_string(), "schema id mismatch");
}
