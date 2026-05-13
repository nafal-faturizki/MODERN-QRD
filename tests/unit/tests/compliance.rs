//! Compliance tests verifying SPECIFICATION.md normative requirements.
//!
//! Each test corresponds to a MUST / MUST NOT clause in the specification.
//! Test names reference the specification section where possible.

use qrd_core::{
    encoding_encode, encoding_decode, codec_compress, codec_decompress,
    aes_encrypt, aes_decrypt,
    ChunkEncryptionMetadata, ColumnChunkHeader, FileHeader, FileReader,
    FooterContent, FooterRowGroupEntry, LogicalTypeId, Nullability, Schema,
    SchemaBuilder, SchemaField, StreamingWriter, Error,
};
use qrd_core::encoding::EncodingId;
use qrd_core::compression::CompressionId;
use qrd_core::integrity::{crc32_bytes, crc32_matches};
use qrd_core::ecc::{self, EccConfig};
use std::io::Cursor;

// ─── §3: File Header compliance ─────────────────────────────────────────────

#[test]
fn spec_3_magic_bytes_are_fixed() {
    // SPECIFICATION §3: MAGIC must be [0x51, 0x52, 0x44, 0x01]
    let header = FileHeader::new(1, 0, [0; 8], 0, 0, 0);
    let bytes = header.to_bytes();
    assert_eq!(&bytes[0..4], &[0x51, 0x52, 0x44, 0x01]);
}

#[test]
fn spec_3_header_is_exactly_32_bytes() {
    let header = FileHeader::new(1, 0, [0; 8], 0, 0, 0);
    assert_eq!(header.to_bytes().len(), 32);
}

#[test]
fn spec_3_rejects_wrong_magic_byte0() {
    let header = FileHeader::new(1, 0, [0; 8], 0, 0, 0);
    let mut bytes = header.to_bytes();
    bytes[0] = 0x00;
    assert!(matches!(FileHeader::parse(&bytes), Err(Error::InvalidMagic)));
}

#[test]
fn spec_3_rejects_wrong_magic_byte1() {
    let header = FileHeader::new(1, 0, [0; 8], 0, 0, 0);
    let mut bytes = header.to_bytes();
    bytes[1] = 0x00;
    assert!(matches!(FileHeader::parse(&bytes), Err(Error::InvalidMagic)));
}

#[test]
fn spec_3_rejects_wrong_magic_byte2() {
    let header = FileHeader::new(1, 0, [0; 8], 0, 0, 0);
    let mut bytes = header.to_bytes();
    bytes[2] = 0x00;
    assert!(matches!(FileHeader::parse(&bytes), Err(Error::InvalidMagic)));
}

#[test]
fn spec_3_rejects_wrong_magic_byte3() {
    let header = FileHeader::new(1, 0, [0; 8], 0, 0, 0);
    let mut bytes = header.to_bytes();
    bytes[3] = 0x00;
    assert!(matches!(FileHeader::parse(&bytes), Err(Error::InvalidMagic)));
}

#[test]
fn spec_3_header_checksum_validated_on_parse() {
    let header = FileHeader::new(1, 0, [0; 8], 0, 0, 0);
    let mut bytes = header.to_bytes();
    bytes[28] ^= 0xFF; // corrupt checksum
    assert!(matches!(FileHeader::parse(&bytes), Err(Error::HeaderChecksumMismatch)));
}

#[test]
fn spec_3_header_checksum_covers_bytes_0_to_27() {
    let header = FileHeader::new(1, 0, [1u8; 8], 0xABCD, 5, 1_700_000_000);
    let bytes = header.to_bytes();
    let expected_crc = crc32_bytes(&bytes[..28]);
    let stored_crc = u32::from_le_bytes([bytes[28], bytes[29], bytes[30], bytes[31]]);
    assert_eq!(stored_crc, expected_crc);
}

#[test]
fn spec_3_major_version_stored_as_u16le_at_offset_4() {
    let header = FileHeader::new(1, 0, [0; 8], 0, 0, 0);
    let bytes = header.to_bytes();
    assert_eq!(u16::from_le_bytes([bytes[4], bytes[5]]), 1u16);
}

#[test]
fn spec_3_minor_version_stored_as_u16le_at_offset_6() {
    let header = FileHeader::new(1, 3, [0; 8], 0, 0, 0);
    let bytes = header.to_bytes();
    assert_eq!(u16::from_le_bytes([bytes[6], bytes[7]]), 3u16);
}

#[test]
fn spec_3_schema_id_stored_at_offset_8() {
    let schema_id = [1u8, 2, 3, 4, 5, 6, 7, 8];
    let header = FileHeader::new(1, 0, schema_id, 0, 0, 0);
    let bytes = header.to_bytes();
    assert_eq!(&bytes[8..16], &schema_id);
}

#[test]
fn spec_3_flags_stored_as_u32le_at_offset_16() {
    let header = FileHeader::new(1, 0, [0; 8], 0xDEAD_BEEF, 0, 0);
    let bytes = header.to_bytes();
    assert_eq!(u32::from_le_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]), 0xDEAD_BEEF);
}

#[test]
fn spec_3_row_group_count_stored_as_u32le_at_offset_20() {
    let header = FileHeader::new(1, 0, [0; 8], 0, 42, 0);
    let bytes = header.to_bytes();
    assert_eq!(u32::from_le_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]), 42);
}

#[test]
fn spec_3_created_at_stored_as_u32le_at_offset_24() {
    let header = FileHeader::new(1, 0, [0; 8], 0, 0, 1_700_000_000);
    let bytes = header.to_bytes();
    assert_eq!(u32::from_le_bytes([bytes[24], bytes[25], bytes[26], bytes[27]]), 1_700_000_000);
}

#[test]
fn spec_3_file_too_small_for_header() {
    let bytes = vec![0x51u8, 0x52, 0x44, 0x01]; // only magic bytes
    assert!(matches!(FileHeader::parse(&bytes), Err(Error::FileTooSmall { .. })));
}

#[test]
fn spec_3_reject_unsupported_major_version_2() {
    let header = FileHeader::new(2, 0, [0; 8], 0, 0, 0);
    assert!(matches!(
        header.validate_major_version(1),
        Err(Error::UnsupportedMajorVersion { major_version: 2 })
    ));
}

#[test]
fn spec_3_accept_supported_major_version_1() {
    let header = FileHeader::new(1, 0, [0; 8], 0, 0, 0);
    assert!(header.validate_major_version(1).is_ok());
}

#[test]
fn spec_3_roundtrip_header_preserves_all_fields() {
    let schema_id = [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF, 0x11, 0x22];
    let header = FileHeader::new(1, 5, schema_id, 0x0000_0001, 100, 1_700_000_000);
    let bytes = header.to_bytes();
    let parsed = FileHeader::parse(&bytes).unwrap();
    assert_eq!(parsed, header);
}

// ─── §7: Encoding compliance ─────────────────────────────────────────────────

#[test]
fn spec_7_plain_empty_roundtrip() {
    let data: &[u8] = &[];
    let enc = encoding_encode(EncodingId::Plain, data).unwrap();
    let dec = encoding_decode(EncodingId::Plain, &enc).unwrap();
    assert_eq!(dec, data);
}

#[test]
fn spec_7_rle_empty_roundtrip() {
    let data: &[u8] = &[];
    let enc = encoding_encode(EncodingId::Rle, data).unwrap();
    let dec = encoding_decode(EncodingId::Rle, &enc).unwrap();
    assert_eq!(dec, data);
}

#[test]
fn spec_7_bit_packed_empty_roundtrip() {
    let data: &[u8] = &[];
    let enc = encoding_encode(EncodingId::BitPacked, data).unwrap();
    let dec = encoding_decode(EncodingId::BitPacked, &enc).unwrap();
    assert_eq!(dec, data);
}

#[test]
fn spec_7_delta_binary_empty_roundtrip() {
    // delta_binary empty: encode produces [0,0,0,0], decode returns []
    let data: &[u8] = &[];
    let enc = encoding_encode(EncodingId::DeltaBinary, data).unwrap();
    let dec = encoding_decode(EncodingId::DeltaBinary, &enc).unwrap();
    assert_eq!(dec, data);
}

#[test]
fn spec_7_delta_byte_array_empty_roundtrip() {
    let data: &[u8] = &[];
    let enc = encoding_encode(EncodingId::DeltaByteArray, data).unwrap();
    let dec = encoding_decode(EncodingId::DeltaByteArray, &enc).unwrap();
    assert_eq!(dec, data);
}

#[test]
fn spec_7_byte_stream_split_empty_roundtrip() {
    let data: &[u8] = &[];
    let enc = encoding_encode(EncodingId::ByteStreamSplit, data).unwrap();
    let dec = encoding_decode(EncodingId::ByteStreamSplit, &enc).unwrap();
    assert_eq!(dec, data);
}

#[test]
fn spec_7_dictionary_rle_empty_roundtrip() {
    let data: &[u8] = &[];
    let enc = encoding_encode(EncodingId::DictionaryRle, data).unwrap();
    let dec = encoding_decode(EncodingId::DictionaryRle, &enc).unwrap();
    assert_eq!(dec, data);
}

#[test]
fn spec_7_delta_binary_rejects_non_multiple_of_4() {
    // SPECIFICATION: DELTA_BINARY operates on 32-bit integers
    let data = &[1u8, 2, 3]; // 3 bytes, not divisible by 4
    let result = encoding_encode(EncodingId::DeltaBinary, data);
    assert!(result.is_err());
}

#[test]
fn spec_7_plain_is_identity() {
    let data = b"arbitrary bytes 12345!@#$%";
    let enc = encoding_encode(EncodingId::Plain, data).unwrap();
    assert_eq!(enc, data);
    let dec = encoding_decode(EncodingId::Plain, &enc).unwrap();
    assert_eq!(dec, data);
}

#[test]
fn spec_7_rle_compresses_repeated_bytes() {
    let data = vec![0xAAu8; 1000];
    let enc = encoding_encode(EncodingId::Rle, &data).unwrap();
    assert!(enc.len() < data.len(), "RLE should compress repeated bytes");
    let dec = encoding_decode(EncodingId::Rle, &enc).unwrap();
    assert_eq!(dec, data);
}

#[test]
fn spec_7_rle_single_run() {
    let data = vec![42u8; 100];
    let enc = qrd_core::encoding::rle::encode(&data).unwrap();
    // Should produce exactly 5 bytes: [100, 0, 0, 0, 42]
    assert_eq!(enc.len(), 5);
    assert_eq!(u32::from_le_bytes([enc[0], enc[1], enc[2], enc[3]]), 100);
    assert_eq!(enc[4], 42);
}

#[test]
fn spec_7_rle_truncated_input_returns_error() {
    let bad = vec![1u8, 0, 0]; // too short for a run record (needs 5 bytes per run)
    let result = qrd_core::encoding::rle::decode(&bad);
    assert!(result.is_err());
}

#[test]
fn spec_7_bit_packed_all_zeros_produces_zero_width() {
    let data = vec![0u8; 16];
    let enc = qrd_core::encoding::bit_packed::encode(&data).unwrap();
    // bit_width should be 0 for all-zero input
    assert_eq!(enc[4], 0u8, "bit_width should be 0 for all-zero data");
    let dec = qrd_core::encoding::bit_packed::decode(&enc).unwrap();
    assert_eq!(dec, data);
}

#[test]
fn spec_7_bit_packed_max_byte_value_uses_8_bits() {
    let data = vec![0xFFu8; 4];
    let enc = qrd_core::encoding::bit_packed::encode(&data).unwrap();
    assert_eq!(enc[4], 8u8, "bit_width should be 8 for 0xFF values");
}

#[test]
fn spec_7_delta_binary_monotone_increasing() {
    let values: Vec<u32> = (0..100).collect();
    let data: Vec<u8> = values.iter().flat_map(|v| v.to_le_bytes()).collect();
    let enc = encoding_encode(EncodingId::DeltaBinary, &data).unwrap();
    let dec = encoding_decode(EncodingId::DeltaBinary, &enc).unwrap();
    assert_eq!(dec, data);
}

#[test]
fn spec_7_delta_binary_monotone_decreasing() {
    let values: Vec<u32> = (0..100).rev().collect();
    let data: Vec<u8> = values.iter().flat_map(|v| v.to_le_bytes()).collect();
    let enc = encoding_encode(EncodingId::DeltaBinary, &data).unwrap();
    let dec = encoding_decode(EncodingId::DeltaBinary, &enc).unwrap();
    assert_eq!(dec, data);
}

#[test]
fn spec_7_byte_stream_split_float32_roundtrip() {
    // Typical use: 4-byte float32 values
    let floats: Vec<f32> = vec![1.0, 2.5, -0.5, 100.0];
    let data: Vec<u8> = floats.iter().flat_map(|f| f.to_le_bytes()).collect();
    let enc = encoding_encode(EncodingId::ByteStreamSplit, &data).unwrap();
    let dec = encoding_decode(EncodingId::ByteStreamSplit, &enc).unwrap();
    assert_eq!(dec, data);
}

#[test]
fn spec_7_dictionary_rle_low_cardinality_compresses() {
    // 3 unique values repeated many times
    let data: Vec<u8> = [1u8, 2, 3].iter().cycle().take(300).copied().collect();
    let enc = encoding_encode(EncodingId::DictionaryRle, &data).unwrap();
    let dec = encoding_decode(EncodingId::DictionaryRle, &enc).unwrap();
    assert_eq!(dec, data);
}

#[test]
fn spec_7_unknown_encoding_id_returns_error() {
    use std::convert::TryFrom;
    assert!(qrd_core::encoding::EncodingId::try_from(0xFFu8).is_err());
    assert!(qrd_core::encoding::EncodingId::try_from(0x07u8).is_err());
}

// ─── §8: Compression compliance ──────────────────────────────────────────────

#[test]
fn spec_8_none_codec_is_passthrough() {
    let data = b"arbitrary data";
    let compressed = codec_compress(CompressionId::None, data).unwrap();
    assert_eq!(compressed, data);
    let decompressed = codec_decompress(CompressionId::None, &compressed).unwrap();
    assert_eq!(decompressed, data);
}

#[test]
fn spec_8_zstd_empty_roundtrip() {
    let data: &[u8] = &[];
    let c = codec_compress(CompressionId::Zstd, data).unwrap();
    let d = codec_decompress(CompressionId::Zstd, &c).unwrap();
    assert_eq!(d, data);
}

#[test]
fn spec_8_lz4_empty_roundtrip() {
    let data: &[u8] = &[];
    let c = codec_compress(CompressionId::Lz4, data).unwrap();
    let d = codec_decompress(CompressionId::Lz4, &c).unwrap();
    assert_eq!(d, data);
}

#[test]
fn spec_8_zstd_compresses_repetitive_data() {
    let data = vec![0xABu8; 10_000];
    let c = codec_compress(CompressionId::Zstd, &data).unwrap();
    assert!(c.len() < data.len(), "ZSTD should compress repetitive data");
    let d = codec_decompress(CompressionId::Zstd, &c).unwrap();
    assert_eq!(d, data);
}

#[test]
fn spec_8_lz4_compresses_repetitive_data() {
    let data = vec![0x55u8; 10_000];
    let c = codec_compress(CompressionId::Lz4, &data).unwrap();
    let d = codec_decompress(CompressionId::Lz4, &c).unwrap();
    assert_eq!(d, data);
}

#[test]
fn spec_8_adaptive_selects_none_for_small_data() {
    let data = vec![0u8; 50]; // < 100 bytes threshold
    let codec = qrd_core::compression::adaptive_select(&data);
    assert_eq!(codec, CompressionId::None);
}

#[test]
fn spec_8_adaptive_selects_zstd_for_low_entropy() {
    let data = vec![0u8; 500]; // highly repetitive
    let codec = qrd_core::compression::adaptive_select(&data);
    assert_eq!(codec, CompressionId::Zstd);
}

#[test]
fn spec_8_unknown_compression_id_returns_error() {
    use std::convert::TryFrom;
    assert!(qrd_core::compression::CompressionId::try_from(0xFFu8).is_err());
    assert!(qrd_core::compression::CompressionId::try_from(0x03u8).is_err());
}

// ─── §10: Encryption compliance ──────────────────────────────────────────────

#[test]
fn spec_10_wrong_key_fails_decryption() {
    let key1: [u8; 32] = [1u8; 32];
    let key2: [u8; 32] = [2u8; 32];
    let plaintext = b"secret data";
    let blob = aes_encrypt(&key1, plaintext).unwrap();
    assert!(aes_decrypt(&key2, &blob).is_err());
}

#[test]
fn spec_10_nonce_is_prepended_to_ciphertext() {
    let key: [u8; 32] = [42u8; 32];
    let plaintext = b"test";
    let blob = aes_encrypt(&key, plaintext).unwrap();
    // blob = nonce (12 bytes) + ciphertext + auth_tag (16 bytes)
    assert!(blob.len() >= 12 + plaintext.len() + 16);
}

#[test]
fn spec_10_auth_tag_is_appended() {
    let key: [u8; 32] = [7u8; 32];
    let plaintext = b"authentication test";
    let blob = aes_encrypt(&key, plaintext).unwrap();
    // Min size: 12 (nonce) + 16 (auth_tag) + plaintext
    assert_eq!(blob.len(), 12 + plaintext.len() + 16);
}

#[test]
fn spec_10_corrupted_ciphertext_fails_authentication() {
    let key: [u8; 32] = [0xAAu8; 32];
    let plaintext = b"integrity check";
    let mut blob = aes_encrypt(&key, plaintext).unwrap();
    // Corrupt 1 byte in the ciphertext region (after nonce)
    blob[15] ^= 0xFF;
    assert!(aes_decrypt(&key, &blob).is_err());
}

#[test]
fn spec_10_truncated_blob_fails() {
    let key: [u8; 32] = [0u8; 32];
    let too_short = &[0u8; 11]; // less than nonce size (12)
    assert!(aes_decrypt(&key, too_short).is_err());
}

#[test]
fn spec_10_hkdf_is_deterministic_with_same_inputs() {
    use qrd_core::encryption::Cipher;
    let master_key = [3u8; 32];
    let salt = [5u8; 32];
    let info = b"col:device_id";
    let k1 = Cipher::derive_key(&master_key, Some(&salt), info).unwrap();
    let k2 = Cipher::derive_key(&master_key, Some(&salt), info).unwrap();
    assert_eq!(k1, k2);
}

#[test]
fn spec_10_hkdf_different_info_produces_different_keys() {
    use qrd_core::encryption::Cipher;
    let master_key = [3u8; 32];
    let salt = [5u8; 32];
    let k1 = Cipher::derive_key(&master_key, Some(&salt), b"col:device_id").unwrap();
    let k2 = Cipher::derive_key(&master_key, Some(&salt), b"col:health_val").unwrap();
    assert_ne!(k1, k2);
}

#[test]
fn spec_10_hkdf_different_master_key_produces_different_keys() {
    use qrd_core::encryption::Cipher;
    let salt = [0u8; 32];
    let info = b"col:sensor";
    let k1 = Cipher::derive_key(&[1u8; 32], Some(&salt), info).unwrap();
    let k2 = Cipher::derive_key(&[2u8; 32], Some(&salt), info).unwrap();
    assert_ne!(k1, k2);
}

#[test]
fn spec_10_hkdf_without_salt_still_works() {
    use qrd_core::encryption::Cipher;
    let master_key = [7u8; 32];
    let k = Cipher::derive_key(&master_key, None, b"info");
    assert!(k.is_ok());
}

// ─── §11: ECC compliance ─────────────────────────────────────────────────────

#[test]
fn spec_11_ecc_config_n_must_be_greater_than_k() {
    assert!(EccConfig::new(5, 5).is_err());
    assert!(EccConfig::new(4, 5).is_err());
    assert!(EccConfig::new(5, 4).is_ok());
}

#[test]
fn spec_11_ecc_config_k_must_be_nonzero() {
    assert!(EccConfig::new(5, 0).is_err());
}

#[test]
fn spec_11_ecc_config_n_must_be_nonzero() {
    assert!(EccConfig::new(0, 0).is_err());
}

#[test]
fn spec_11_ecc_parity_count_is_n_minus_k() {
    let cfg = EccConfig::new(10, 7).unwrap();
    assert_eq!(cfg.parity_count(), 3);
}

#[test]
fn spec_11_ecc_encode_produces_correct_parity_count() {
    let cfg = EccConfig::new(5, 3).unwrap();
    let chunks = vec![vec![1u8, 2, 3]; 3];
    let refs: Vec<&[u8]> = chunks.iter().map(|c| c.as_slice()).collect();
    let parity = ecc::encode(&cfg, &refs).unwrap();
    assert_eq!(parity.len(), 2);
}

#[test]
fn spec_11_ecc_recovers_single_missing_data_shard() {
    let cfg = EccConfig::new(5, 3).unwrap();
    let chunks = vec![vec![10u8; 8], vec![20u8; 8], vec![30u8; 8]];
    let refs: Vec<&[u8]> = chunks.iter().map(|c| c.as_slice()).collect();
    let parity = ecc::encode(&cfg, &refs).unwrap();

    let shards: Vec<Option<Vec<u8>>> = vec![
        None, // missing
        Some(chunks[1].clone()),
        Some(chunks[2].clone()),
        Some(parity[0].clone()),
        Some(parity[1].clone()),
    ];
    let recovered = ecc::decode(&cfg, &shards).unwrap();
    assert_eq!(recovered[0], chunks[0]);
}

#[test]
fn spec_11_ecc_rejects_mismatched_shard_count() {
    let cfg = EccConfig::new(5, 3).unwrap();
    let shards: Vec<Option<Vec<u8>>> = vec![
        Some(vec![1u8; 4]),
        Some(vec![2u8; 4]),
        // only 2 shards, expected 5
    ];
    assert!(ecc::decode(&cfg, &shards).is_err());
}

// ─── §12: Integrity compliance ────────────────────────────────────────────────

#[test]
fn spec_12_crc32_ieee_test_vector() {
    // CRC32 of "123456789" is 0xCBF43926 per ISO 3309
    assert_eq!(crc32_bytes(b"123456789"), 0xCBF4_3926);
}

#[test]
fn spec_12_crc32_empty_input() {
    let checksum = crc32_bytes(b"");
    assert!(crc32_matches(b"", checksum));
}

#[test]
fn spec_12_crc32_mismatch_detected() {
    let data = b"test data";
    let checksum = crc32_bytes(data);
    assert!(!crc32_matches(data, checksum.wrapping_add(1)));
}

#[test]
fn spec_12_crc32_is_deterministic() {
    let data = b"deterministic test";
    assert_eq!(crc32_bytes(data), crc32_bytes(data));
}

// ─── §13: Writer compliance ───────────────────────────────────────────────────

#[test]
fn spec_13_writer_rejects_wrong_column_count() {
    let schema = SchemaBuilder::new()
        .field(SchemaField::new("a", LogicalTypeId::Int32, Nullability::Required))
        .field(SchemaField::new("b", LogicalTypeId::Int32, Nullability::Required))
        .build()
        .unwrap();

    let mut writer = StreamingWriter::new(Cursor::new(Vec::new()), schema).unwrap();
    // Provide only 1 column instead of 2
    let result = writer.write_row(vec![vec![1, 2, 3, 4]]);
    assert!(result.is_err());
}

#[test]
fn spec_13_file_header_written_at_byte_zero() {
    let schema = SchemaBuilder::new()
        .field(SchemaField::new("x", LogicalTypeId::Int32, Nullability::Required))
        .build()
        .unwrap();

    let buf = Cursor::new(Vec::new());
    let writer = StreamingWriter::new(buf, schema).unwrap();
    let cursor = writer.finish().unwrap();
    let bytes = cursor.into_inner();
    // First 4 bytes must be the QRD magic
    assert_eq!(&bytes[0..4], &[0x51, 0x52, 0x44, 0x01]);
}

#[test]
fn spec_13_footer_is_last_data_in_file() {
    let schema = SchemaBuilder::new()
        .field(SchemaField::new("val", LogicalTypeId::Int64, Nullability::Required))
        .build()
        .unwrap();

    let buf = Cursor::new(Vec::new());
    let mut writer = StreamingWriter::new(buf, schema).unwrap();
    writer.write_row(vec![100i64.to_le_bytes().to_vec()]).unwrap();
    let cursor = writer.finish().unwrap();
    let bytes = cursor.into_inner();

    // Last 4 bytes must be the footer length field (as per SPECIFICATION §6)
    let footer_len = u32::from_le_bytes([
        bytes[bytes.len() - 4],
        bytes[bytes.len() - 3],
        bytes[bytes.len() - 2],
        bytes[bytes.len() - 1],
    ]);
    assert!(footer_len > 0, "footer length must be > 0");
    assert!((footer_len as usize + 4) <= bytes.len(), "footer must fit within file");
}

#[test]
fn spec_13_writer_reader_row_count_matches() {
    let schema = SchemaBuilder::new()
        .field(SchemaField::new("id", LogicalTypeId::Int32, Nullability::Required))
        .build()
        .unwrap();

    let buf = Cursor::new(Vec::new());
    let mut writer = StreamingWriter::new(buf, schema).unwrap();
    for i in 0i32..50 {
        writer.write_row(vec![i.to_le_bytes().to_vec()]).unwrap();
    }
    let cursor = writer.finish().unwrap();
    let reader = FileReader::new(Cursor::new(cursor.into_inner())).unwrap();
    assert_eq!(reader.total_rows(), 50);
}

#[test]
fn spec_13_schema_id_in_header_matches_footer() {
    let schema = SchemaBuilder::new()
        .field(SchemaField::new("ts", LogicalTypeId::Timestamp, Nullability::Required))
        .field(SchemaField::new("val", LogicalTypeId::Float64, Nullability::Optional))
        .build()
        .unwrap();

    let buf = Cursor::new(Vec::new());
    let mut writer = StreamingWriter::new(buf, schema.clone()).unwrap();
    writer.write_row(vec![
        100i64.to_le_bytes().to_vec(),
        3.14f64.to_le_bytes().to_vec(),
    ]).unwrap();
    let cursor = writer.finish().unwrap();
    let reader = FileReader::new(Cursor::new(cursor.into_inner())).unwrap();

    let expected_id = schema.schema_id().unwrap();
    assert_eq!(reader.file_header().schema_id, expected_id);
}

// ─── §9: Type system compliance ──────────────────────────────────────────────

#[test]
fn spec_9_all_logical_types_serialize_in_schema() {
    use std::convert::TryFrom;
    use qrd_core::LogicalTypeId;

    let types = [
        LogicalTypeId::Boolean,
        LogicalTypeId::Int8,
        LogicalTypeId::Int16,
        LogicalTypeId::Int32,
        LogicalTypeId::Int64,
        LogicalTypeId::UInt8,
        LogicalTypeId::UInt16,
        LogicalTypeId::UInt32,
        LogicalTypeId::UInt64,
        LogicalTypeId::Float32,
        LogicalTypeId::Float64,
        LogicalTypeId::Timestamp,
        LogicalTypeId::Date,
        LogicalTypeId::Time,
        LogicalTypeId::Duration,
        LogicalTypeId::Utf8String,
        LogicalTypeId::Enum,
        LogicalTypeId::Uuid,
        LogicalTypeId::Blob,
        LogicalTypeId::Decimal,
        LogicalTypeId::Struct,
        LogicalTypeId::Array,
        LogicalTypeId::Map,
        LogicalTypeId::Any,
    ];

    for lt in types {
        let schema = SchemaBuilder::new()
            .field(SchemaField::new("col", lt, Nullability::Required))
            .build()
            .unwrap();

        let bytes = schema.serialize_footer_schema_section().unwrap();
        let (parsed, _) = Schema::parse_footer_schema_section(&bytes).unwrap();
        assert_eq!(parsed.fields()[0].logical_type_id, lt);
    }
}

#[test]
fn spec_9_unknown_logical_type_returns_error() {
    use std::convert::TryFrom;
    assert!(qrd_core::schema::LogicalTypeId::try_from(0x00u8).is_err());
    assert!(qrd_core::schema::LogicalTypeId::try_from(0xFEu8).is_err());
}

#[test]
fn spec_9_all_nullability_options_serialize() {
    use qrd_core::Nullability;
    for null in [Nullability::Required, Nullability::Optional, Nullability::Repeated] {
        let schema = SchemaBuilder::new()
            .field(SchemaField::new("col", LogicalTypeId::Int32, null))
            .build()
            .unwrap();
        let bytes = schema.serialize_footer_schema_section().unwrap();
        let (parsed, _) = Schema::parse_footer_schema_section(&bytes).unwrap();
        assert_eq!(parsed.fields()[0].nullability, null);
    }
}

// ─── Column Chunk compliance ──────────────────────────────────────────────────

#[test]
fn spec_5_plain_column_chunk_header_is_28_bytes() {
    let header = ColumnChunkHeader::new_plain(0x00, 0x00, 0x00, 100, 100, 0, 50, 0);
    let bytes = header.to_bytes().unwrap();
    assert_eq!(bytes.len(), 28);
}

#[test]
fn spec_5_encrypted_column_chunk_header_has_nonce_and_tag() {
    let meta = ChunkEncryptionMetadata {
        nonce: [1u8; 12],
        auth_tag: [2u8; 16],
        key_id: b"sensor".to_vec(),
    };
    let header = ColumnChunkHeader::new_encrypted(0x00, 0x00, 0x00, 100, 100, 0, 50, 0, meta);
    let bytes = header.to_bytes().unwrap();
    // must be > 28 (base) + 12 (nonce) + 16 (tag) + 2 (key_id_len) + 6 (key_id)
    assert!(bytes.len() > 28 + 12 + 16 + 2 + 6);
}

#[test]
fn spec_5_column_chunk_header_roundtrip_plain() {
    let header = ColumnChunkHeader::new_plain(0x00, 0x01, 0x02, 256, 512, 3, 100, 42);
    let bytes = header.to_bytes().unwrap();
    let (parsed, consumed) = ColumnChunkHeader::parse(&bytes).unwrap();
    assert_eq!(consumed, bytes.len());
    assert_eq!(parsed.encoding_id, header.encoding_id);
    assert_eq!(parsed.compression_id, header.compression_id);
    assert_eq!(parsed.compressed_size, header.compressed_size);
    assert_eq!(parsed.uncompressed_size, header.uncompressed_size);
    assert_eq!(parsed.row_count_chunk, header.row_count_chunk);
}

#[test]
fn spec_5_column_chunk_header_roundtrip_encrypted() {
    let meta = ChunkEncryptionMetadata {
        nonce: [0xABu8; 12],
        auth_tag: [0xCDu8; 16],
        key_id: b"health_val_key".to_vec(),
    };
    let header = ColumnChunkHeader::new_encrypted(0x00, 0x01, 0x00, 100, 200, 0, 50, 0, meta);
    let bytes = header.to_bytes().unwrap();
    let (parsed, _) = ColumnChunkHeader::parse(&bytes).unwrap();
    let enc = parsed.encryption.unwrap();
    assert_eq!(enc.nonce, [0xABu8; 12]);
    assert_eq!(enc.auth_tag, [0xCDu8; 16]);
    assert_eq!(enc.key_id, b"health_val_key".to_vec());
}

#[test]
fn spec_5_column_chunk_rejects_unknown_encryption_id() {
    let mut bytes = vec![0u8; 28];
    bytes[2] = 0xFE; // unknown encryption id
    assert!(ColumnChunkHeader::parse(&bytes).is_err());
}

// ─── Row group compliance ─────────────────────────────────────────────────────

#[test]
fn spec_4_row_group_header_is_12_bytes() {
    use qrd_core::RowGroupHeader;
    let rg = RowGroupHeader::new(100, 3, 0);
    assert_eq!(rg.to_bytes().len(), 12);
}

#[test]
fn spec_4_row_group_header_checksum_is_validated() {
    use qrd_core::RowGroupHeader;
    let rg = RowGroupHeader::new(100, 3, 0);
    let mut bytes = rg.to_bytes();
    bytes[8] ^= 0xFF; // corrupt checksum
    assert!(RowGroupHeader::parse(&bytes).is_err());
}

#[test]
fn spec_4_row_group_header_roundtrip() {
    use qrd_core::RowGroupHeader;
    let rg = RowGroupHeader::new(512, 8, 0x0001);
    let bytes = rg.to_bytes();
    let parsed = RowGroupHeader::parse(&bytes).unwrap();
    assert_eq!(parsed.row_count, 512);
    assert_eq!(parsed.column_count, 8);
    assert_eq!(parsed.rg_flags, 0x0001);
}
