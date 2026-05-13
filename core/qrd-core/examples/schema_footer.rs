use qrd_core::schema::{LogicalTypeId, Nullability, Schema, SchemaField};

fn main() {
    let schema = Schema::builder()
        .field(SchemaField::new("device_id", LogicalTypeId::Enum, Nullability::Required))
        .build()
        .unwrap();

    let footer_schema = schema.serialize_footer_schema_section().unwrap();
    println!("schema bytes: {}", footer_schema.len());
}
