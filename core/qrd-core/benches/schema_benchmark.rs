use qrd_core::schema::{LogicalTypeId, Nullability, Schema, SchemaField};

fn main() {
    let schema = Schema::builder()
        .field(SchemaField::new("device_id", LogicalTypeId::Enum, Nullability::Required))
        .build()
        .unwrap();

    assert_eq!(schema.fields().len(), 1);
}
