use qrd_core::columnar;

#[test]
fn transpose_simple() {
    let rows = vec![
        vec![vec![1u8], vec![2u8]],
        vec![vec![3u8], vec![4u8]],
    ];

    let columns = columnar::transpose(&rows, 2).unwrap();
    assert_eq!(columns.len(), 2);
    assert_eq!(columns[0], vec![1u8, 3]);
    assert_eq!(columns[1], vec![2u8, 4]);
}

#[test]
fn transpose_empty() {
    let rows: Vec<Vec<Vec<u8>>> = vec![];
    let columns = columnar::transpose(&rows, 3).unwrap();
    assert_eq!(columns.len(), 3);
    for col in columns {
        assert!(col.is_empty());
    }
}

#[test]
fn transpose_validate_column_count() {
    let rows = vec![
        vec![vec![1u8], vec![2u8], vec![3u8]],
        vec![vec![4u8], vec![5u8]], // Wrong count!
    ];

    let result = columnar::transpose(&rows, 3);
    assert!(result.is_err());
}
