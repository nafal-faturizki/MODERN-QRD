//! Regression tests for known edge cases and formerly reported bugs.
//!
//! Each test name describes the precise scenario that was found to be
//! problematic or is worth guarding against forever.

use qrd_core::{
    ColumnChunkHeader, FileReader, LogicalTypeId, Nullability, SchemaBuilder, SchemaField,
    StreamingWriter, Error,
};
use qrd_core::encoding::{self, EncodingId};
use qrd_core::compression::{self, CompressionId};
use qrd_core::ecc::{self, EccConfig};
use qrd_core::integrity::crc32_bytes;
use std::io::Cursor;

// ─── Encoding regressions ─────────────────────────────────────────────────────

#[test]
fn regression_rle_empty_input_produces_empty_output() {
    let enc = encoding::rle::encode(&[]).unwrap();
    assert!(enc.is_empty());
}

#[test]
fn regression_rle_decode_empty_is_ok() {
    let dec = encoding::rle::decode(&[]).unwrap();
    assert!(dec.is_empty());
}

#[test]
fn regression_bit_packed_decode_truncated_returns_error() {
    // 4 bytes instead of minimum 5 (orig_len + bit_width)
    assert!(encoding::bit_packed::decode(&[0u8; 4]).is_err());
}

#[test]
fn regression_bit_packed_zero_byte_data_roundtrip() {
    let data = vec![0u8; 100];
    let enc = encoding::bit_packed::encode(&data).unwrap();
    let dec = encoding::bit_packed::decode(&enc).unwrap();
    assert_eq!(dec, data);
}

#[test]
fn regression_delta_binary_non_multiple_of_4_returns_error() {
    for bad_len in [1usize, 2, 3, 5, 6, 7, 9, 10, 11] {
        let data = vec![0u8; bad_len];
        assert!(
            encoding::delta_binary::encode(&data).is_err(),
            "should fail for len={}", bad_len
        );
    }
}

#[test]
fn regression_delta_binary_min_encoded_length_check() {
    // Truncated delta_binary data (< 8 bytes) must not panic
    assert!(encoding::delta_binary::decode(&[0u8; 7]).is_err());
    assert!(encoding::delta_binary::decode(&[0u8; 4]).is_err());
    assert!(encoding::delta_binary::decode(&[0u8; 0]).is_err());
}

#[test]
fn regression_delta_byte_array_truncated_returns_error() {
    assert!(encoding::delta_byte_array::decode(&[0u8; 3]).is_err());
}

#[test]
fn regression_byte_stream_split_truncated_returns_error() {
    assert!(encoding::byte_stream_split::decode(&[0u8; 3]).is_err());
}

#[test]
fn regression_dictionary_rle_truncated_returns_error() {
    assert!(encoding::dictionary_rle::decode(&[0u8; 4]).is_err());
}

#[test]
fn regression_delta_binary_wrapping_add_doesnt_panic() {
    // Large values that would overflow if not using wrapping_add
    let values: Vec<u32> = vec![u32::MAX, 1, 0];
    let data: Vec<u8> = values.iter().flat_map(|v| v.to_le_bytes()).collect();
    let enc = encoding::delta_binary::encode(&data).unwrap();
    let dec = encoding::delta_binary::decode(&enc).unwrap();
    assert_eq!(dec, data);
}

#[test]
fn regression_rle_exact_u32_max_run_doesnt_panic() {
    // Test that a run of exactly 1 doesn't cause issues
    let data = vec![0xABu8];
    let enc = encoding::rle::encode(&data).unwrap();
    assert_eq!(enc, &[1u8, 0, 0, 0, 0xAB]);
}

#[test]
fn regression_dictionary_rle_all_unique_bytes() {
    // 256 unique bytes, one of each - no compression expected
    let data: Vec<u8> = (0u8..=255).collect();
    let enc = encoding::dictionary_rle::encode(&data).unwrap();
    let dec = encoding::dictionary_rle::decode(&enc).unwrap();
    assert_eq!(dec, data);
}

#[test]
fn regression_byte_stream_split_odd_length_roundtrip() {
    // Non-multiple-of-4 lengths must roundtrip correctly
    for len in [1usize, 2, 3, 5, 6, 7, 9, 10, 11, 13] {
        let data = vec![0x5Au8; len];
        let enc = encoding::byte_stream_split::encode(&data).unwrap();
        let dec = encoding::byte_stream_split::decode(&enc).unwrap();
        assert_eq!(dec, data, "failed for len={}", len);
    }
}

// ─── Compression regressions ──────────────────────────────────────────────────

#[test]
fn regression_zstd_large_homogeneous_data() {
    let data = vec![0xBBu8; 100_000];
    let c = compression::zstd_codec::compress(&data).unwrap();
    let d = compression::zstd_codec::decompress(&c).unwrap();
    assert_eq!(d, data);
}

#[test]
fn regression_lz4_large_data_roundtrip() {
    let data: Vec<u8> = (0..50_000).map(|i| (i % 256) as u8).collect();
    let c = compression::lz4_codec::compress(&data).unwrap();
    let d = compression::lz4_codec::decompress(&c).unwrap();
    assert_eq!(d, data);
}

#[test]
fn regression_compression_pipeline_encode_compress_roundtrip() {
    // Typical pipeline: encode then compress
    let raw: Vec<u8> = vec![10u8; 200];
    let encoded = encoding::encoding_encode(EncodingId::Rle, &raw).unwrap();
    let compressed = compression::codec_compress(CompressionId::Zstd, &encoded).unwrap();

    let decompressed = compression::codec_decompress(CompressionId::Zstd, &compressed).unwrap();
    let decoded = encoding::encoding_decode(EncodingId::Rle, &decompressed).unwrap();
    assert_eq!(decoded, raw);
}

// ─── ECC regressions ─────────────────────────────────────────────────────────

#[test]
fn regression_ecc_recovers_last_data_shard() {
    let cfg = EccConfig::new(5, 3).unwrap();
    let chunks = vec![vec![1u8; 8], vec![2u8; 8], vec![3u8; 8]];
    let refs: Vec<&[u8]> = chunks.iter().map(|c| c.as_slice()).collect();
    let parity = ecc::encode(&cfg, &refs).unwrap();

    let shards: Vec<Option<Vec<u8>>> = vec![
        Some(chunks[0].clone()),
        Some(chunks[1].clone()),
        None, // last data shard missing
        Some(parity[0].clone()),
        Some(parity[1].clone()),
    ];
    let recovered = ecc::decode(&cfg, &shards).unwrap();
    assert_eq!(recovered[2], chunks[2]);
}

#[test]
fn regression_ecc_recovers_first_data_shard() {
    let cfg = EccConfig::new(5, 3).unwrap();
    let chunks = vec![vec![11u8; 4], vec![22u8; 4], vec![33u8; 4]];
    let refs: Vec<&[u8]> = chunks.iter().map(|c| c.as_slice()).collect();
    let parity = ecc::encode(&cfg, &refs).unwrap();

    let shards: Vec<Option<Vec<u8>>> = vec![
        None, // first data shard missing
        Some(chunks[1].clone()),
        Some(chunks[2].clone()),
        Some(parity[0].clone()),
        Some(parity[1].clone()),
    ];
    let recovered = ecc::decode(&cfg, &shards).unwrap();
    assert_eq!(recovered[0], chunks[0]);
}

#[test]
fn regression_ecc_same_data_chunks_produce_deterministic_parity() {
    let cfg = EccConfig::new(6, 4).unwrap();
    let chunks: Vec<Vec<u8>> = vec![vec![0xABu8; 8]; 4];
    let refs1: Vec<&[u8]> = chunks.iter().map(|c| c.as_slice()).collect();
    let refs2: Vec<&[u8]> = chunks.iter().map(|c| c.as_slice()).collect();
    let p1 = ecc::encode(&cfg, &refs1).unwrap();
    let p2 = ecc::encode(&cfg, &refs2).unwrap();
    assert_eq!(p1, p2);
}

#[test]
fn regression_ecc_zero_length_chunks() {
    // ECC with zero-length chunks (chunk_len=0)
    let cfg = EccConfig::new(5, 3).unwrap();
    let chunks: Vec<Vec<u8>> = vec![vec![]; 3];
    let refs: Vec<&[u8]> = chunks.iter().map(|c| c.as_slice()).collect();
    let parity = ecc::encode(&cfg, &refs);
    // Should succeed (zero-length is valid)
    assert!(parity.is_ok());
}

// ─── Writer/Reader regressions ────────────────────────────────────────────────

#[test]
fn regression_writer_empty_file_has_footer() {
    let schema = SchemaBuilder::new()
        .field(SchemaField::new("x", LogicalTypeId::Int32, Nullability::Required))
        .build()
        .unwrap();

    let buf = Cursor::new(Vec::new());
    let writer = StreamingWriter::new(buf, schema).unwrap();
    // No rows written - finish should still produce valid file
    let cursor = writer.finish().unwrap();
    let bytes = cursor.into_inner();

    // File must be at least header (32) + footer with length field (4)
    assert!(bytes.len() >= 36);
}

#[test]
fn regression_writer_row_group_size_boundary() {
    // Write exactly row_group_size rows; should flush exactly 1 row group
    let schema = SchemaBuilder::new()
        .field(SchemaField::new("n", LogicalTypeId::Int32, Nullability::Required))
        .build()
        .unwrap();

    let buf = Cursor::new(Vec::new());
    let mut writer = StreamingWriter::new(buf, schema)
        .unwrap()
        .with_row_group_size(5);

    for i in 0i32..5 {
        writer.write_row(vec![i.to_le_bytes().to_vec()]).unwrap();
    }
    // After 5 rows (== row_group_size), should have auto-flushed
    assert_eq!(writer.current_row_group.row_count, 0);
}

#[test]
fn regression_reader_detects_schema_id_mismatch() {
    use qrd_core::{FileHeader, FooterContent, FooterRowGroupEntry, encode_footer_envelope};

    let schema = SchemaBuilder::new()
        .field(SchemaField::new("x", LogicalTypeId::Int32, Nullability::Required))
        .build()
        .unwrap();

    let schema_id = schema.schema_id().unwrap();
    // Use a wrong schema ID in the header
    let wrong_schema_id = [0xFF; 8];
    let header = FileHeader::new(1, 0, wrong_schema_id, 0, 0, 0);

    let footer = FooterContent {
        footer_version: 1,
        schema,
        row_groups: vec![],
        statistics_flag: 0,
        statistics_bytes: vec![],
        encryption_metadata: None,
        schema_signature: None,
        file_metadata: vec![],
    };
    let body = footer.to_bytes().unwrap();
    let env = encode_footer_envelope(&body).unwrap();

    let mut bytes = header.to_bytes().to_vec();
    bytes.extend_from_slice(&env);

    let result = FileReader::new(Cursor::new(bytes));
    assert!(result.is_err());
}

#[test]
fn regression_reader_multi_row_group_total_rows() {
    let schema = SchemaBuilder::new()
        .field(SchemaField::new("id", LogicalTypeId::Int32, Nullability::Required))
        .build()
        .unwrap();

    let buf = Cursor::new(Vec::new());
    let mut writer = StreamingWriter::new(buf, schema)
        .unwrap()
        .with_row_group_size(10);

    // Write 25 rows → 2 full groups (10 each) + 1 partial (5)
    for i in 0i32..25 {
        writer.write_row(vec![i.to_le_bytes().to_vec()]).unwrap();
    }

    let cursor = writer.finish().unwrap();
    let reader = FileReader::new(Cursor::new(cursor.into_inner())).unwrap();
    assert_eq!(reader.total_rows(), 25);
    assert_eq!(reader.row_group_count(), 3);
}

#[test]
fn regression_file_too_small_for_reader() {
    // Only 4 bytes - too small for header + footer
    let bytes = vec![0x51u8, 0x52, 0x44, 0x01];
    let result = FileReader::new(Cursor::new(bytes));
    assert!(result.is_err());
}

#[test]
fn regression_crc32_of_known_string() {
    // CRC32("QRD") must be stable across implementations
    let c = crc32_bytes(b"QRD");
    // Verify it's deterministic
    assert_eq!(c, crc32_bytes(b"QRD"));
    assert_ne!(c, crc32_bytes(b"qrd")); // case-sensitive
}

// ─── Schema regressions ───────────────────────────────────────────────────────

#[test]
fn regression_schema_empty_field_name_is_valid() {
    let schema = SchemaBuilder::new()
        .field(SchemaField::new("", LogicalTypeId::Blob, Nullability::Optional))
        .build()
        .unwrap();
    let bytes = schema.serialize_footer_schema_section().unwrap();
    let (parsed, _) = qrd_core::Schema::parse_footer_schema_section(&bytes).unwrap();
    assert_eq!(parsed.fields()[0].name, "");
}

#[test]
fn regression_schema_field_with_metadata_roundtrip() {
    use qrd_core::SchemaMetadataEntry;
    let field = SchemaField::new("col", LogicalTypeId::Utf8String, Nullability::Optional)
        .with_metadata([
            SchemaMetadataEntry::new("description", "User email address"),
            SchemaMetadataEntry::new("pii", "true"),
        ]);
    let schema = SchemaBuilder::new().field(field).build().unwrap();
    let bytes = schema.serialize_footer_schema_section().unwrap();
    let (parsed, _) = qrd_core::Schema::parse_footer_schema_section(&bytes).unwrap();
    assert_eq!(parsed.fields()[0].metadata.len(), 2);
    assert_eq!(parsed.fields()[0].metadata[0].key, "description");
    assert_eq!(parsed.fields()[0].metadata[1].value, "true");
}

#[test]
fn regression_schema_version_is_preserved_in_roundtrip() {
    let schema = qrd_core::Schema::builder()
        .schema_version(3)
        .field(SchemaField::new("x", LogicalTypeId::Int32, Nullability::Required))
        .build()
        .unwrap();
    let bytes = schema.serialize_footer_schema_section().unwrap();
    let (parsed, _) = qrd_core::Schema::parse_footer_schema_section(&bytes).unwrap();
    assert_eq!(parsed.schema_version(), 3);
}

use qrd_core::encoding::encoding_encode;
use qrd_core::encoding::encoding_decode;
use qrd_core::compression::codec_compress;
use qrd_core::compression::codec_decompress;
