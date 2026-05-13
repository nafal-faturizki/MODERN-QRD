use qrd_core::reader::ColumnReader;

#[test]
fn column_reader_creation() {
    let reader = ColumnReader::new(vec![0, 2, 4]);
    assert_eq!(reader.columns(), &[0, 2, 4]);
}

#[test]
fn column_reader_with_filter() {
    let reader = ColumnReader::new(vec![0])
        .with_row_filter(Box::new(|row_id| row_id < 100));

    assert!(reader.has_row_filter());
}
