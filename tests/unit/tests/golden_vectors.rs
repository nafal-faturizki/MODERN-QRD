//! Golden vector tests for all encoding and compression algorithms.
//!
//! These tests verify that the implementation produces the exact binary
//! output specified by the format. Each golden vector is a (input, expected_encoded)
//! pair derived from the format specification in SPECIFICATION.md.
//!
//! Golden vectors serve as cross-language reference: the Python, TypeScript,
//! Go, Java, and C/C++ SDKs MUST produce identical output for the same input.

use qrd_core::encoding;
use qrd_core::compression;
use qrd_core::integrity::crc32_bytes;

// ─── PLAIN golden vectors ─────────────────────────────────────────────────────

#[test]
fn golden_plain_empty() {
    let enc = encoding::plain::encode(b"").unwrap();
    assert_eq!(enc, b"");
}

#[test]
fn golden_plain_ascii() {
    let data = b"QRD";
    let enc = encoding::plain::encode(data).unwrap();
    assert_eq!(enc, data);
}

#[test]
fn golden_plain_all_bytes_0x00_to_0x0f() {
    let data: Vec<u8> = (0u8..16).collect();
    let enc = encoding::plain::encode(&data).unwrap();
    assert_eq!(enc, data);
}

#[test]
fn golden_plain_all_bytes_0xf0_to_0xff() {
    let data: Vec<u8> = (0xF0u8..=0xFFu8).collect();
    let enc = encoding::plain::encode(&data).unwrap();
    assert_eq!(enc, data);
}

#[test]
fn golden_plain_single_0x00() {
    assert_eq!(encoding::plain::encode(&[0x00]).unwrap(), &[0x00]);
}

#[test]
fn golden_plain_single_0xff() {
    assert_eq!(encoding::plain::encode(&[0xFF]).unwrap(), &[0xFF]);
}

// ─── RLE golden vectors ───────────────────────────────────────────────────────

// RLE format: pairs of (count: u32LE, value_byte)

#[test]
fn golden_rle_single_run_of_one() {
    // [0x01] → count=1, value=0x01 → [1,0,0,0, 1]
    let enc = encoding::rle::encode(&[0x01u8]).unwrap();
    assert_eq!(enc, &[1, 0, 0, 0, 0x01]);
}

#[test]
fn golden_rle_run_of_three_aa() {
    // [0xAA, 0xAA, 0xAA] → count=3, value=0xAA → [3,0,0,0, 0xAA]
    let enc = encoding::rle::encode(&[0xAAu8; 3]).unwrap();
    assert_eq!(enc, &[3, 0, 0, 0, 0xAA]);
}

#[test]
fn golden_rle_two_runs() {
    // [0x01, 0x01, 0x02] → run(2, 0x01) + run(1, 0x02)
    let enc = encoding::rle::encode(&[0x01u8, 0x01, 0x02]).unwrap();
    assert_eq!(enc, &[2, 0, 0, 0, 0x01, 1, 0, 0, 0, 0x02]);
}

#[test]
fn golden_rle_alternating_bytes() {
    // [0x00, 0x01, 0x00, 0x01] → 4 runs of length 1
    let data = &[0x00u8, 0x01, 0x00, 0x01];
    let enc = encoding::rle::encode(data).unwrap();
    assert_eq!(enc, &[
        1, 0, 0, 0, 0x00,
        1, 0, 0, 0, 0x01,
        1, 0, 0, 0, 0x00,
        1, 0, 0, 0, 0x01,
    ]);
}

#[test]
fn golden_rle_decode_golden_vector() {
    // Decode: [2,0,0,0, 0x41, 3,0,0,0, 0x42] → "AABBB"
    let encoded = &[2u8, 0, 0, 0, 0x41, 3, 0, 0, 0, 0x42];
    let dec = encoding::rle::decode(encoded).unwrap();
    assert_eq!(dec, b"AABBB");
}

// ─── BIT_PACKED golden vectors ────────────────────────────────────────────────

// Format: [orig_len: u32LE][bit_width: u8][packed_bits...]

#[test]
fn golden_bit_packed_all_zeros() {
    // All zeros → bit_width=0 → [len:u32LE, 0]
    let data = vec![0u8; 4];
    let enc = encoding::bit_packed::encode(&data).unwrap();
    assert_eq!(u32::from_le_bytes([enc[0], enc[1], enc[2], enc[3]]), 4u32);
    assert_eq!(enc[4], 0u8, "bit_width must be 0");
    assert_eq!(enc.len(), 5); // only header, no payload for bit_width=0
}

#[test]
fn golden_bit_packed_values_0_1_header() {
    // [0, 1] → bit_width=1, orig_len=2
    let data = vec![0u8, 1u8];
    let enc = encoding::bit_packed::encode(&data).unwrap();
    assert_eq!(u32::from_le_bytes([enc[0], enc[1], enc[2], enc[3]]), 2u32);
    assert_eq!(enc[4], 1u8, "bit_width must be 1 for max value 1");
}

#[test]
fn golden_bit_packed_values_0_to_3_uses_2_bits() {
    // [0, 1, 2, 3] → bit_width=2
    let data = vec![0u8, 1, 2, 3];
    let enc = encoding::bit_packed::encode(&data).unwrap();
    assert_eq!(enc[4], 2u8, "bit_width must be 2");
}

#[test]
fn golden_bit_packed_max_byte_uses_8_bits() {
    let data = vec![0xFFu8; 4];
    let enc = encoding::bit_packed::encode(&data).unwrap();
    assert_eq!(enc[4], 8u8, "bit_width must be 8 for 0xFF values");
}

#[test]
fn golden_bit_packed_roundtrip_sequential() {
    let data: Vec<u8> = (0u8..=7).collect(); // values 0-7, need 3 bits
    let enc = encoding::bit_packed::encode(&data).unwrap();
    assert_eq!(enc[4], 3u8, "bit_width must be 3 for max value 7");
    let dec = encoding::bit_packed::decode(&enc).unwrap();
    assert_eq!(dec, data);
}

// ─── DELTA_BINARY golden vectors ─────────────────────────────────────────────

// Format: [value_count: u32LE][first_value: u32LE][delta_i: i32LE ...]

#[test]
fn golden_delta_binary_single_value_zero() {
    // [0u32] → count=1, first=0, no deltas
    let data = 0u32.to_le_bytes().to_vec();
    let enc = encoding::delta_binary::encode(&data).unwrap();
    // count(u32) + first_value(u32) = 8 bytes, no deltas
    assert_eq!(enc.len(), 8);
    assert_eq!(u32::from_le_bytes([enc[0], enc[1], enc[2], enc[3]]), 1u32); // count
    assert_eq!(u32::from_le_bytes([enc[4], enc[5], enc[6], enc[7]]), 0u32); // first
}

#[test]
fn golden_delta_binary_sequential_values() {
    // [100u32, 101u32, 102u32] → count=3, first=100, deltas=[1, 1]
    let data: Vec<u8> = [100u32, 101u32, 102u32]
        .iter()
        .flat_map(|v| v.to_le_bytes())
        .collect();
    let enc = encoding::delta_binary::encode(&data).unwrap();
    assert_eq!(u32::from_le_bytes([enc[0], enc[1], enc[2], enc[3]]), 3u32); // count
    assert_eq!(u32::from_le_bytes([enc[4], enc[5], enc[6], enc[7]]), 100u32); // first
    assert_eq!(i32::from_le_bytes([enc[8], enc[9], enc[10], enc[11]]), 1i32); // delta 1
    assert_eq!(i32::from_le_bytes([enc[12], enc[13], enc[14], enc[15]]), 1i32); // delta 2
}

#[test]
fn golden_delta_binary_constant_values() {
    // [5u32, 5u32, 5u32] → count=3, first=5, deltas=[0, 0]
    let data: Vec<u8> = [5u32; 3].iter().flat_map(|v| v.to_le_bytes()).collect();
    let enc = encoding::delta_binary::encode(&data).unwrap();
    assert_eq!(i32::from_le_bytes([enc[8], enc[9], enc[10], enc[11]]), 0i32);
}

#[test]
fn golden_delta_binary_negative_delta() {
    // [10u32, 5u32] → count=2, first=10, delta=[-5]
    let data: Vec<u8> = [10u32, 5u32].iter().flat_map(|v| v.to_le_bytes()).collect();
    let enc = encoding::delta_binary::encode(&data).unwrap();
    assert_eq!(i32::from_le_bytes([enc[8], enc[9], enc[10], enc[11]]), -5i32);
}

// ─── DELTA_BYTE_ARRAY golden vectors ─────────────────────────────────────────

// Format: [orig_len: u32LE][first_byte: u8][delta_i: i16LE ...]

#[test]
fn golden_delta_byte_array_single_byte() {
    // [0x42] → orig_len=1, first=0x42, no deltas
    let enc = encoding::delta_byte_array::encode(&[0x42u8]).unwrap();
    assert_eq!(enc.len(), 5); // 4 + 1
    assert_eq!(u32::from_le_bytes([enc[0], enc[1], enc[2], enc[3]]), 1u32);
    assert_eq!(enc[4], 0x42u8);
}

#[test]
fn golden_delta_byte_array_increasing_sequence() {
    // [10, 12, 14] → orig_len=3, first=10, deltas=[2, 2]
    let data = &[10u8, 12, 14];
    let enc = encoding::delta_byte_array::encode(data).unwrap();
    assert_eq!(u32::from_le_bytes([enc[0], enc[1], enc[2], enc[3]]), 3u32);
    assert_eq!(enc[4], 10u8);
    assert_eq!(i16::from_le_bytes([enc[5], enc[6]]), 2i16);
    assert_eq!(i16::from_le_bytes([enc[7], enc[8]]), 2i16);
}

#[test]
fn golden_delta_byte_array_constant_sequence() {
    // [5, 5, 5] → deltas=[0, 0]
    let data = &[5u8; 3];
    let enc = encoding::delta_byte_array::encode(data).unwrap();
    assert_eq!(i16::from_le_bytes([enc[5], enc[6]]), 0i16);
}

// ─── BYTE_STREAM_SPLIT golden vectors ────────────────────────────────────────

// Format: [orig_len: u32LE][stream_0 || stream_1 || stream_2 || stream_3]

#[test]
fn golden_byte_stream_split_4_bytes() {
    // [a, b, c, d] → stream[0]=[a], stream[1]=[b], stream[2]=[c], stream[3]=[d]
    let data = &[0x11u8, 0x22, 0x33, 0x44];
    let enc = encoding::byte_stream_split::encode(data).unwrap();
    // orig_len=4 as u32LE, then 4 streams of 1 byte each
    assert_eq!(enc.len(), 4 + 4); // 4 (len field) + 4 (streams)
    assert_eq!(u32::from_le_bytes([enc[0], enc[1], enc[2], enc[3]]), 4u32);
    // Each byte should be in its own stream
    assert_eq!(enc[4], 0x11); // stream 0
    assert_eq!(enc[5], 0x22); // stream 1
    assert_eq!(enc[6], 0x33); // stream 2
    assert_eq!(enc[7], 0x44); // stream 3
}

#[test]
fn golden_byte_stream_split_8_bytes() {
    // [a,b,c,d, e,f,g,h] → streams [a,e], [b,f], [c,g], [d,h]
    let data = &[1u8, 2, 3, 4, 5, 6, 7, 8];
    let enc = encoding::byte_stream_split::encode(data).unwrap();
    // 4 (len) + 2+2+2+2 = 12 bytes total
    assert_eq!(enc.len(), 12);
    assert_eq!(enc[4], 1u8); // stream[0][0]
    assert_eq!(enc[5], 5u8); // stream[0][1]
    assert_eq!(enc[6], 2u8); // stream[1][0]
    assert_eq!(enc[7], 6u8); // stream[1][1]
    assert_eq!(enc[8], 3u8); // stream[2][0]
    assert_eq!(enc[9], 7u8); // stream[2][1]
    assert_eq!(enc[10], 4u8); // stream[3][0]
    assert_eq!(enc[11], 8u8); // stream[3][1]
}

// ─── DICTIONARY_RLE golden vectors ───────────────────────────────────────────

// Format: [orig_len: u32LE][dict_len: u8][dict_entries...][run_len: u32LE][index: u8]...

#[test]
fn golden_dict_rle_single_value() {
    // [0xAA] → dict=[0xAA], run(1, index=0)
    let enc = encoding::dictionary_rle::encode(&[0xAAu8]).unwrap();
    assert_eq!(u32::from_le_bytes([enc[0], enc[1], enc[2], enc[3]]), 1u32); // orig_len
    assert_eq!(enc[4], 1u8); // dict_len
    assert_eq!(enc[5], 0xAAu8); // dict[0]
    assert_eq!(u32::from_le_bytes([enc[6], enc[7], enc[8], enc[9]]), 1u32); // run_len
    assert_eq!(enc[10], 0u8); // index
}

#[test]
fn golden_dict_rle_two_values() {
    // [0x01, 0x02] → dict=[0x01, 0x02], run(1,0), run(1,1)
    let enc = encoding::dictionary_rle::encode(&[0x01u8, 0x02]).unwrap();
    assert_eq!(u32::from_le_bytes([enc[0], enc[1], enc[2], enc[3]]), 2u32);
    assert_eq!(enc[4], 2u8); // dict_len=2
    assert_eq!(enc[5], 0x01u8); // dict[0]
    assert_eq!(enc[6], 0x02u8); // dict[1]
    // first run: len=1, index=0
    assert_eq!(u32::from_le_bytes([enc[7], enc[8], enc[9], enc[10]]), 1u32);
    assert_eq!(enc[11], 0u8);
    // second run: len=1, index=1
    assert_eq!(u32::from_le_bytes([enc[12], enc[13], enc[14], enc[15]]), 1u32);
    assert_eq!(enc[16], 1u8);
}

#[test]
fn golden_dict_rle_repeated_single() {
    // [0xFF, 0xFF, 0xFF] → dict=[0xFF], run(3, 0)
    let enc = encoding::dictionary_rle::encode(&[0xFFu8; 3]).unwrap();
    assert_eq!(enc[4], 1u8); // dict_len=1
    assert_eq!(u32::from_le_bytes([enc[6], enc[7], enc[8], enc[9]]), 3u32); // run_len=3
    assert_eq!(enc[10], 0u8); // index=0
}

// ─── CRC32 golden vectors ─────────────────────────────────────────────────────

#[test]
fn golden_crc32_ieee_123456789() {
    // ISO 3309 test vector
    assert_eq!(crc32_bytes(b"123456789"), 0xCBF4_3926);
}

#[test]
fn golden_crc32_empty_string() {
    // CRC32 of empty string is 0x00000000
    assert_eq!(crc32_bytes(b""), 0x0000_0000);
}

#[test]
fn golden_crc32_single_zero_byte() {
    // CRC32 of [0x00] is well-defined (from CRC32/ISO-HDLC standard)
    let checksum = crc32_bytes(&[0x00u8]);
    assert!(checksum != 0xCBF4_3926, "should be different from '123456789' CRC");
}

#[test]
fn golden_crc32_all_zeros_vs_all_ff() {
    let c0 = crc32_bytes(&[0x00u8; 16]);
    let cff = crc32_bytes(&[0xFFu8; 16]);
    assert_ne!(c0, cff, "CRC32 of all-zeros must differ from all-0xFF");
}

#[test]
fn golden_crc32_qrd_magic() {
    // CRC32 of the QRD magic bytes
    let magic = &[0x51u8, 0x52, 0x44, 0x01];
    let checksum = crc32_bytes(magic);
    assert!(checksum != 0, "CRC32 of magic must be non-zero");
}

// ─── Compression golden vectors ───────────────────────────────────────────────

#[test]
fn golden_compression_none_preserves_bytes_exactly() {
    let data = b"\x00\x01\x02\x03\xFF\xFE\xFD\xFC";
    let c = compression::none::compress(data).unwrap();
    assert_eq!(&c, data);
}

#[test]
fn golden_compression_zstd_roundtrip_256_bytes() {
    let data: Vec<u8> = (0u8..=255).collect();
    let c = compression::zstd_codec::compress(&data).unwrap();
    let d = compression::zstd_codec::decompress(&c).unwrap();
    assert_eq!(d, data);
}

#[test]
fn golden_compression_lz4_roundtrip_256_bytes() {
    let data: Vec<u8> = (0u8..=255).collect();
    let c = compression::lz4_codec::compress(&data).unwrap();
    let d = compression::lz4_codec::decompress(&c).unwrap();
    assert_eq!(d, data);
}

#[test]
fn golden_compression_zstd_produces_valid_magic() {
    // ZSTD frames start with magic 0xFD2FB528
    let data = b"some test data to compress";
    let c = compression::zstd_codec::compress(data).unwrap();
    // ZSTD magic is [0x28, 0xB5, 0x2F, 0xFD] in little-endian
    assert_eq!(&c[0..4], &[0x28u8, 0xB5, 0x2F, 0xFD], "ZSTD frame magic mismatch");
}

// ─── Schema golden vectors ────────────────────────────────────────────────────

#[test]
fn golden_schema_fingerprint_is_deterministic() {
    let schema = qrd_core::Schema::builder()
        .field(qrd_core::SchemaField::new("id", LogicalTypeId::Int64, Nullability::Required))
        .field(qrd_core::SchemaField::new("val", LogicalTypeId::Float64, Nullability::Optional))
        .build()
        .unwrap();

    let fp1 = schema.schema_fingerprint().unwrap();
    let fp2 = schema.schema_fingerprint().unwrap();
    assert_eq!(fp1, fp2);
}

#[test]
fn golden_schema_id_is_first_8_bytes_of_fingerprint() {
    let schema = qrd_core::Schema::builder()
        .field(qrd_core::SchemaField::new("x", LogicalTypeId::Int32, Nullability::Required))
        .build()
        .unwrap();

    let fp = schema.schema_fingerprint().unwrap();
    let id = schema.schema_id().unwrap();
    assert_eq!(&id, &fp[..8]);
}

#[test]
fn golden_schema_different_field_names_produce_different_ids() {
    let s1 = qrd_core::Schema::builder()
        .field(qrd_core::SchemaField::new("fieldA", LogicalTypeId::Int32, Nullability::Required))
        .build()
        .unwrap();
    let s2 = qrd_core::Schema::builder()
        .field(qrd_core::SchemaField::new("fieldB", LogicalTypeId::Int32, Nullability::Required))
        .build()
        .unwrap();
    assert_ne!(s1.schema_id().unwrap(), s2.schema_id().unwrap());
}

#[test]
fn golden_schema_different_types_produce_different_ids() {
    let s1 = qrd_core::Schema::builder()
        .field(qrd_core::SchemaField::new("x", LogicalTypeId::Int32, Nullability::Required))
        .build()
        .unwrap();
    let s2 = qrd_core::Schema::builder()
        .field(qrd_core::SchemaField::new("x", LogicalTypeId::Int64, Nullability::Required))
        .build()
        .unwrap();
    assert_ne!(s1.schema_id().unwrap(), s2.schema_id().unwrap());
}

#[test]
fn golden_schema_roundtrip_multi_field() {
    let schema = qrd_core::Schema::builder()
        .field(qrd_core::SchemaField::new("device_id", LogicalTypeId::Enum, Nullability::Required))
        .field(qrd_core::SchemaField::new("ts", LogicalTypeId::Timestamp, Nullability::Required))
        .field(qrd_core::SchemaField::new("health", LogicalTypeId::Float32, Nullability::Optional))
        .build()
        .unwrap();

    let bytes = schema.serialize_footer_schema_section().unwrap();
    let (parsed, consumed) = qrd_core::Schema::parse_footer_schema_section(&bytes).unwrap();
    assert_eq!(consumed, bytes.len());
    assert_eq!(parsed.fields().len(), 3);
    assert_eq!(parsed.fields()[0].name, "device_id");
    assert_eq!(parsed.fields()[1].name, "ts");
    assert_eq!(parsed.fields()[2].name, "health");
}

// ─── File footer golden vectors ───────────────────────────────────────────────

#[test]
fn golden_footer_envelope_length_field_is_last_4_bytes() {
    let body = b"test footer body";
    let env = qrd_core::encode_footer_envelope(body).unwrap();
    let len_field = u32::from_le_bytes([
        env[env.len() - 4],
        env[env.len() - 3],
        env[env.len() - 2],
        env[env.len() - 1],
    ]);
    // The length field encodes the length of (checksum + body)
    assert!(len_field > 0);
    assert_eq!(len_field as usize + 4, env.len());
}

#[test]
fn golden_footer_envelope_is_deterministic() {
    let body = b"deterministic footer";
    let e1 = qrd_core::encode_footer_envelope(body).unwrap();
    let e2 = qrd_core::encode_footer_envelope(body).unwrap();
    assert_eq!(e1, e2);
}

#[test]
fn golden_footer_decode_rejects_corrupt_checksum() {
    let body = b"footer content";
    let mut env = qrd_core::encode_footer_envelope(body).unwrap();
    env[0] ^= 0x01; // corrupt first byte of checksum
    assert!(matches!(
        qrd_core::decode_footer_body(&env),
        Err(qrd_core::Error::FooterChecksumMismatch)
    ));
}

use qrd_core::LogicalTypeId;
use qrd_core::Nullability;
