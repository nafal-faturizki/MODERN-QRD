use qrd_core::compression;

#[test]
fn none_roundtrip() {
    let data = b"hello world";
    let compressed = compression::none::compress(data).unwrap();
    let decompressed = compression::none::decompress(&compressed).unwrap();
    assert_eq!(&decompressed[..], data);
}

#[test]
fn zstd_roundtrip() {
    let data = b"hello world! This is a test. ".repeat(10);
    let data_flat = data.as_slice();
    let compressed = compression::zstd_codec::compress(data_flat).unwrap();
    assert!(compressed.len() < data_flat.len());
    let decompressed = compression::zstd_codec::decompress(&compressed).unwrap();
    assert_eq!(&decompressed[..], data_flat);
}

#[test]
fn lz4_roundtrip() {
    let data = b"hello world! This is a test. ".repeat(10);
    let data_flat = data.as_slice();
    let compressed = compression::lz4_codec::compress(data_flat).unwrap();
    let decompressed = compression::lz4_codec::decompress(&compressed).unwrap();
    assert_eq!(&decompressed[..], data_flat);
}

#[test]
fn adaptive_select_high_entropy() {
    let random_data: Vec<u8> = (0..100).map(|i| (i * 7) as u8).collect();
    let codec = compression::adaptive_select(&random_data);
    // High entropy data should not use strong compression
    assert!(!matches!(codec, compression::CompressionId::Zstd));
}

#[test]
fn adaptive_select_low_entropy() {
    let repetitive_data = vec![1u8; 200];
    let codec = compression::adaptive_select(&repetitive_data);
    assert_eq!(codec, compression::CompressionId::Zstd);
}
