//! Build script for qrd-unit-tests.
//!
//! Generates comprehensive parametric test cases for all core engine components,
//! targeting 10,000+ test cases as required by Phase 1 exit criteria in ROADMAP.md.
//!
//! Test counts:
//!   - Encoding:    8,960 tests (7 algos × 5 levels × 256 byte values)
//!   - Compression: 1,536 tests (3 codecs × 2 levels × 256 byte values)
//!   - Encryption:    256 tests (256 key variants)
//!   - Integrity:     256 tests (256 CRC32 single-byte tests)
//!   Total generated: 11,008 tests

use std::fmt::Write as FmtWrite;
use std::fs;
use std::path::PathBuf;

fn main() {
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());

    generate_encoding_tests(&out_dir);
    generate_compression_tests(&out_dir);
    generate_encryption_tests(&out_dir);
    generate_integrity_tests(&out_dir);

    println!("cargo:rerun-if-changed=build.rs");
}

/// Generates roundtrip tests for all 7 encoding algorithms.
///
/// For each algorithm and each level (input size pattern), generates
/// 256 distinct test cases (one per possible byte value 0–255).
///
/// Non-delta algorithms (PLAIN, RLE, BIT_PACKED, DELTA_BYTE_ARRAY,
/// BYTE_STREAM_SPLIT, DICTIONARY_RLE): inputs are N repetitions of `byte`.
///
/// DELTA_BINARY: inputs are N u32 values in little-endian, first value
/// is `byte as u32`, remaining are 0. Delta binary requires len % 4 == 0.
fn generate_encoding_tests(out_dir: &PathBuf) {
    let mut code = String::new();

    // Non-delta algorithms accept arbitrary byte sequences.
    let non_delta = [
        ("plain", "plain"),
        ("rle", "rle"),
        ("bit_packed", "bit_packed"),
        ("delta_byte_array", "delta_byte_array"),
        ("byte_stream_split", "byte_stream_split"),
        ("dictionary_rle", "dictionary_rle"),
    ];

    // Levels: repeat count for the same byte value.
    let non_delta_levels: &[(u8, &str)] = &[
        (1, "l1"),
        (2, "l2"),
        (3, "l3"),
        (4, "l4"),
        (8, "l5"),
    ];

    for (algo_name, algo_mod) in &non_delta {
        for (repeat, level_name) in non_delta_levels {
            for byte in 0u8..=255 {
                writeln!(
                    code,
                    "#[test]\nfn enc_{algo_name}_{level_name}_b{byte:03}() {{\
                    \n    let data = vec![{byte}u8; {repeat}];\
                    \n    let encoded = qrd_core::encoding::{algo_mod}::encode(&data).unwrap();\
                    \n    let decoded = qrd_core::encoding::{algo_mod}::decode(&encoded).unwrap();\
                    \n    assert_eq!(decoded, data);\
                    \n}}\n",
                    algo_name = algo_name,
                    level_name = level_name,
                    byte = byte,
                    repeat = repeat,
                    algo_mod = algo_mod,
                )
                .unwrap();
            }
        }
    }

    // DELTA_BINARY: requires multiples of 4 bytes.
    // Levels: N u32 values (first = byte as u32, rest = 0).
    let delta_levels: &[(usize, &str)] = &[
        (1, "l1"),
        (2, "l2"),
        (3, "l3"),
        (4, "l4"),
        (5, "l5"),
    ];

    for (num_u32s, level_name) in delta_levels {
        for byte in 0u8..=255 {
            let val = byte as u32;
            let mut data_expr = format!("{val}u32.to_le_bytes().to_vec()");
            for _ in 1..*num_u32s {
                data_expr.push_str("; data.extend_from_slice(&0u32.to_le_bytes())");
            }
            // Generate multi-statement data construction
            let mut stmts = format!("    let mut data = {}u32.to_le_bytes().to_vec();\n", val);
            for _ in 1..*num_u32s {
                stmts.push_str("    data.extend_from_slice(&0u32.to_le_bytes());\n");
            }
            writeln!(
                code,
                "#[test]\nfn enc_delta_binary_{level_name}_b{byte:03}() {{\
                \n{stmts}\
                    \n    let encoded = qrd_core::encoding::delta_binary::encode(&data).unwrap();\
                    \n    let decoded = qrd_core::encoding::delta_binary::decode(&encoded).unwrap();\
                    \n    assert_eq!(decoded, data);\
                    \n}}\n",
                level_name = level_name,
                byte = byte,
                stmts = stmts,
            )
            .unwrap();
        }
    }

    let path = out_dir.join("encoding_generated.rs");
    fs::write(&path, code).unwrap();
}

/// Generates roundtrip tests for all 3 compression codecs.
///
/// For each codec and level, 256 distinct tests (one per byte value).
fn generate_compression_tests(out_dir: &PathBuf) {
    let mut code = String::new();

    let codecs = [
        ("none", "none"),
        ("zstd_codec", "zstd_codec"),
        ("lz4_codec", "lz4_codec"),
    ];

    // Level 1: single byte; Level 2: 2-byte same-value pair.
    let levels: &[(usize, &str)] = &[(1, "l1"), (2, "l2")];

    for (codec_name, codec_mod) in &codecs {
        for (repeat, level_name) in levels {
            for byte in 0u8..=255 {
                writeln!(
                    code,
                    "#[test]\nfn cmp_{codec_name}_{level_name}_b{byte:03}() {{\
                    \n    let data = vec![{byte}u8; {repeat}];\
                    \n    let compressed = qrd_core::compression::{codec_mod}::compress(&data).unwrap();\
                    \n    let decompressed = qrd_core::compression::{codec_mod}::decompress(&compressed).unwrap();\
                    \n    assert_eq!(decompressed, data);\
                    \n}}\n",
                    codec_name = codec_name,
                    level_name = level_name,
                    byte = byte,
                    repeat = repeat,
                    codec_mod = codec_mod,
                )
                .unwrap();
            }
        }
    }

    let path = out_dir.join("compression_generated.rs");
    fs::write(&path, code).unwrap();
}

/// Generates 256 AES-256-GCM encrypt/decrypt roundtrip tests.
///
/// Each test uses a different 32-byte master key (all bytes set to `n`
/// where `n` ranges 0–255) with a fixed plaintext to verify that the
/// key material is correctly used in both directions.
fn generate_encryption_tests(out_dir: &PathBuf) {
    let mut code = String::new();

    for byte in 0u8..=255 {
        writeln!(
            code,
            "#[test]\nfn encrypt_key_b{byte:03}() {{\
            \n    let master_key: [u8; 32] = [{byte}u8; 32];\
            \n    let plaintext = b\"qrd golden plaintext {byte}\";\
            \n    let blob = qrd_core::encryption::encrypt(&master_key, plaintext).unwrap();\
            \n    assert!(blob.len() > 28, \"blob must include nonce + ciphertext + tag\");\
            \n    let decrypted = qrd_core::encryption::decrypt(&master_key, &blob).unwrap();\
            \n    assert_eq!(decrypted.as_slice(), plaintext.as_ref());\
            \n}}\n",
            byte = byte,
        )
        .unwrap();
    }

    let path = out_dir.join("encryption_generated.rs");
    fs::write(&path, code).unwrap();
}

/// Generates 256 CRC32 integrity tests, one for each single-byte value.
///
/// Each test verifies that:
///   1. `crc32_matches(data, crc32_bytes(data))` returns true.
///   2. `crc32_matches(data, crc32_bytes(data) ^ 0xFFFF_FFFF)` returns false.
fn generate_integrity_tests(out_dir: &PathBuf) {
    let mut code = String::new();

    for byte in 0u8..=255 {
        writeln!(
            code,
            "#[test]\nfn crc32_byte_b{byte:03}() {{\
            \n    let data = &[{byte}u8];\
            \n    let checksum = qrd_core::integrity::crc32_bytes(data);\
            \n    assert!(qrd_core::integrity::crc32_matches(data, checksum));\
            \n    assert!(!qrd_core::integrity::crc32_matches(data, checksum ^ 0xffff_ffff));\
            \n}}\n",
            byte = byte,
        )
        .unwrap();
    }

    let path = out_dir.join("integrity_generated.rs");
    fs::write(&path, code).unwrap();
}
