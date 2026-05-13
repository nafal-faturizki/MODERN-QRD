<div align="center">

<img src="https://drive.google.com/uc?export=view&id=1Q_-_J8JKuPwO8t3e6HGfW26rB_ZTkAkH" alt="QRD-SDK Logo" width="180"/>

<br/>

# QRD-SDK

### Privacy-Native Streaming Analytical Binary Container Format

**Edge-native · Zero-Knowledge · WASM-capable · Multi-language · Deterministic**

<br/>

[![CI](https://github.com/zenipara/QRD-SDK/actions/workflows/ci.yml/badge.svg)](https://github.com/zenipara/QRD-SDK/actions/workflows/ci.yml)
[![License: BSL-1.1](https://img.shields.io/badge/License-BSL--1.1-blue.svg)](LICENSE)
[![Rust Edition](https://img.shields.io/badge/Rust-2021_Edition-orange.svg)](https://www.rust-lang.org/)
[![Version](https://img.shields.io/badge/Version-1.0.0-blue.svg)](CHANGELOG.md)
[![Docs](https://img.shields.io/badge/Docs-docs.qrd.dev-brightgreen.svg)](https://docs.qrd.dev)
[![Crates.io](https://img.shields.io/badge/crates.io-qrd--core-red.svg)](https://crates.io/crates/qrd-core)
[![Security Audit](https://img.shields.io/badge/Security-Audited-darkgreen.svg)](docs/security/SECURITY_AUDIT.md)
[![FIPS-140-3 Aligned](https://img.shields.io/badge/Crypto-FIPS--140--3_Aligned-navy.svg)](docs/security/CRYPTOGRAPHY.md)

<br/>

[Overview](#-overview) · [Positioning](#-positioning) · [Why QRD](#-why-qrd) · [Design Principles](#-design-principles) · [Architecture](#-architecture) · [Binary Format](#-binary-format-specification) · [Encryption & ZK Model](#-encryption--zero-knowledge-model) · [Type System](#-type-system) · [Encoding](#-encoding-algorithms) · [Compression](#-compression) · [Security & Trust](#-security--trust) · [Threat Model](#-threat-model) · [Quick Start](#-quick-start) · [SDKs](#-multi-language-sdk) · [Test Suite](#-test-suite) · [Benchmarks](#-benchmarks) · [Use Cases](#-use-cases) · [Evolution](#-evolution-roadmap) · [Contributing](#-contributing)

</div>

---

## 📌 Overview

**QRD** (Columnar Row Descriptor) adalah **format binary container kolumnar yang dirancang dengan privacy sebagai properti desain inti**, bukan fitur tambahan. QRD dibangun untuk analytical workloads di lingkungan **edge, browser, dan offline** — di mana data sensitif bergerak melintasi batas kepercayaan yang tidak dapat diasumsikan aman.

QRD dibangun dengan prinsip **streaming-first** — data ditulis secara inkremental dalam row group tanpa perlu membuffer seluruh dataset di memori. Di balik semua binding multi-bahasa terdapat satu **Rust core engine** yang menjadi sumber kebenaran tunggal. Setiap bahasa hanya menyediakan lapisan tipis di atas FFI atau WASM, memastikan fidelitas format yang identik di semua platform dan runtime.

```
QRD bukan database. QRD bukan pengganti Parquet.
QRD bukan pengganti format universal semua usecase.

QRD adalah encrypted columnar container layer untuk sistem yang membutuhkan:
  ✓ Enkripsi end-to-end sebagai properti format, bukan infrastruktur
  ✓ Zero-knowledge storage: server tidak dapat membaca konten tanpa kunci
  ✓ Streaming ingestion dari edge ke cloud dengan bounded memory
  ✓ Analytical columnar reads di browser via WASM tanpa dekripsi server-side
  ✓ Deterministic binary output lintas bahasa dan platform
  ✓ Kepercayaan yang dapat diverifikasi secara kriptografis, bukan dijalankan atas kepercayaan
```

> **Catatan Penting**: QRD mengisi niche spesifik — privacy-native encrypted columnar streaming. QRD **bukan** solusi universal yang menggantikan Parquet untuk warehouse analytics, SQLite untuk OLTP, atau Arrow IPC untuk in-process data sharing. Untuk usecase tersebut, gunakan format yang tepat.

---

## 🎯 Positioning

### Universal Privacy-Native Binary Container Layer

QRD berdiri di persimpangan dua domain yang sebelumnya tidak dapat dipersatukan dengan elegan:

```
                    TANPA QRD
                    
[Edge Device]                    [Cloud Storage]
Plaintext data ──────────────►  Plaintext at rest
                    ↑
           server harus bisa baca data
           untuk deduplikasi, indexing, analytics

                    DENGAN QRD

[Edge Device]                    [Cloud Storage]
Encrypted QRD ──────────────►  Encrypted at rest
                    ↑
           server menyimpan ciphertext
           tidak pernah melihat plaintext
           reader butuh kunci untuk dekripsi
```

**QRD bukan Universal Container pengganti semua format.** QRD adalah **encrypted columnar transport dan storage layer** yang optimal untuk:

| ✅ Cocok untuk QRD | ❌ Bukan target QRD |
|---|---|
| Sensor telemetry dengan data sensitif (health, location, biometrics) | Data warehouse analytics tanpa persyaratan privasi (gunakan Parquet) |
| Cross-boundary data transfer dengan zero-trust | In-process Arrow IPC antara services di trust boundary sama |
| Browser-native analytics tanpa data meninggalkan perangkat | General-purpose database (gunakan SQLite/DuckDB) |
| Audit logs dengan integritas kriptografis terverifikasi | Bulk ETL tanpa enkripsi requirement |
| Edge AI inference di perangkat terbatas | Real-time OLTP workloads |

### Differensiasi dari Format Lain

| Properti | **QRD** | Parquet | Arrow IPC | CSV | SQLite |
|---|---|---|---|---|---|
| Format type | Encrypted columnar container | Columnar binary file | In-memory / IPC | Text table | Embedded relational DB |
| Privacy sebagai properti format | **Native** | Ekstensi eksternal | Ekstensi eksternal | Tidak ada | Optional plugin |
| Zero-knowledge server | **Ya, by design** | Tidak | Tidak | Tidak | Tidak |
| Streaming write | **Native row-group stream** | Requires buffering | Not designed | Yes (no schema) | Limited |
| Offline-first | **Ya** | Ecosystem-heavy | Tidak | Ya | Ya |
| Partial column read | **Ya** | Ya | Ya | Tidak | Query-bound |
| Schema embedded | **Ya** | Ya | Ya | Tidak | Ya |
| Chunk-level independent compression | **Ya** | Ya | Partial | Tidak | Optional |
| Enkripsi granular per-kolom | **Ya** | Tidak | Tidak | Tidak | Database-level |
| Error correction (Reed-Solomon) | **Ya** | Tidak | Tidak | Tidak | Tidak |
| Browser / WASM | **First-class** | Limited | Arrow JS | Ya | Tidak |
| Cross-language fidelity | **Single engine** | Multiple impls | Reference impl | Trivial | Single engine |
| Bounded-memory streaming | **By design** | Bukan tujuan primer | Bukan tujuan primer | Ya (no schema) | Tidak |

---

## 🔍 Why QRD

### Masalah yang Dipecahkan

Format yang ada hari ini memiliki trade-off yang tidak cocok untuk privacy-native edge pipelines:

| Masalah | Format Lama | Solusi QRD |
|---|---|---|
| Parquet butuh buffer dataset penuh | Tidak cocok untuk streaming | Row-group streaming dengan bounded memory |
| Arrow IPC bukan file format persisten | In-memory only | Persistent binary container dengan schema footer |
| CSV tidak ada schema, encoding, atau kompresi | Terlalu primitif | Self-describing dengan encoding, kompresi, dan enkripsi |
| SQLite bukan columnar analytics | Row-oriented | Columnar chunks dengan partial reads |
| Format lain tidak support WASM/browser | Server-only | First-class WASM dan browser support |
| Enkripsi sebagai afterthought | Bukan properti format | Zero-knowledge per-column encryption native |
| Multiple implementasi menyebabkan drift | Inkonsistensi cross-language | Satu Rust engine, semua bahasa via FFI |
| Tidak ada error correction untuk storage degraded | Data loss permanen | Reed-Solomon ECC parity chunks |

---

## 🧱 Design Principles

Prinsip desain QRD adalah **kontrak teknis**, bukan aspirasi. Setiap rilis harus dapat membuktikan kepatuhan terhadap seluruh prinsip ini.

```
 1. PRIVACY-NATIVE
    Enkripsi adalah properti format, bukan infrastruktur.
    Kolom dienkripsi sebelum meninggalkan encoder.
    Server yang menyimpan file QRD tidak pernah melihat plaintext.
    Tidak ada "encrypted mode" vs "plaintext mode" sebagai konfigurasi runtime.

 2. ZERO-KNOWLEDGE BY DEFAULT
    Format tidak mengekspos informasi tentang nilai plaintext tanpa kunci.
    Statistik (min/max/distinct_count) dienkripsi bersama payload bila kolom dienkripsi.
    Footer metadata untuk kolom terenkripsi tidak membocorkan distribusi data.

 3. DETERMINISTIC
    Input identik selalu menghasilkan binary identik di semua bahasa dan platform.
    Tidak ada randomness dalam format kecuali kriptografis (nonce, IV).
    Cross-language golden vector tests membuktikan determinisme ini.

 4. STREAMING-FIRST
    Ingestion tak terbatas tanpa materialisasi dataset penuh.
    Memory Writer: proporsional terhadap satu row group, bukan total file.
    Footer ditulis last — tidak ada backtrack ke header saat streaming.

 5. COLUMNAR
    Row-to-column transposisi per row group.
    Selective reads: hanya column chunks yang dibutuhkan dibaca dari disk.
    Compression dan encoding dioptimasi per-kolom berdasarkan karakteristik data.

 6. BOUNDED MEMORY
    Writer memory: O(row_group_size × avg_row_width)
    Reader memory: O(selected_columns × active_row_groups)
    Tidak bergantung pada ukuran total file — cocok untuk device dengan RAM terbatas.

 7. SELF-DESCRIBING
    Schema embedded di footer — tidak perlu schema registry eksternal.
    Schema fingerprint (SHA-256 truncated) di header untuk validasi lintas file.
    Format versi embedded untuk backward compatibility yang deterministic.

 8. CRYPTOGRAPHIC TRUST
    Setiap klaim integritas (checksum, auth tag) dapat diverifikasi tanpa kepercayaan.
    CRC32 per column chunk dan footer untuk deteksi korupsi non-adversarial.
    AES-256-GCM auth tag membuktikan integritas dan authenticity payload terenkripsi.
    Reed-Solomon ECC untuk recovery dari media degraded.

 9. LITTLE-ENDIAN CANONICAL
    Semua integer multi-byte dalam format little-endian.
    Platform big-endian melakukan byte-swap saat baca/tulis.
    Canonical encoding memastikan binary identik di semua arsitektur.

10. PARSER HARDENING
    Setiap field eksternal divalidasi sebelum digunakan.
    Tolak header/footer malformed, terpotong, atau magic byte salah.
    Fail-fast pada encoding/compression ID yang tidak dikenal.
    Zero panic policy di core engine pada input adversarial.

11. AUDIT-READY
    Semua unsafe Rust didokumentasi dengan safety invariant eksplisit.
    Setiap klaim kriptografis mengacu pada primitive yang telah diaudit.
    Test suite 10.000+ kasus mencakup golden vectors, property tests, dan fuzz corpus.
```

---

## 🏗 Architecture

### Layered Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        Application Layer                        │
│     Analytics pipeline · ML inference · Telemetry · Audit log  │
└──────────────────────────────┬──────────────────────────────────┘
                               │
┌──────────────────────────────▼──────────────────────────────────┐
│                      Language SDK Layer                         │
│                                                                 │
│  ┌──────────┐  ┌────────────┐  ┌──────┐  ┌──────────────────┐  │
│  │  Python  │  │ TypeScript │  │  Go  │  │  Java  /  C/C++  │  │
│  │  (PyO3)  │  │   (WASM)   │  │(CGO) │  │  (JNI  /  FFI)  │  │
│  └────┬─────┘  └─────┬──────┘  └──┬───┘  └────────┬─────────┘  │
└───────│───────────────│────────────│────────────────│────────────┘
        │               │            │                │
┌───────▼───────────────▼────────────▼────────────────▼────────────┐
│                   FFI / WASM Interface Layer                      │
│     core/qrd-ffi/   (C-compatible ABI, stable)                   │
│     core/qrd-wasm/  (WebAssembly target, WASI + browser)         │
└──────────────────────────────┬──────────────────────────────────-┘
                               │
┌──────────────────────────────▼──────────────────────────────────┐
│                       Rust Core Engine                           │
│                        core/qrd-core/                           │
│                                                                 │
│  ┌─────────┐ ┌──────────┐ ┌──────────┐ ┌──────────────────┐   │
│  │ Schema  │ │  Writer  │ │  Reader  │ │    Encoding      │   │
│  │ Builder │ │Streaming │ │  Partial │ │  PLAIN/RLE/DELTA │   │
│  └─────────┘ └──────────┘ └──────────┘ └──────────────────┘   │
│                                                                 │
│  ┌─────────────┐ ┌──────────────┐ ┌────────────────────────┐  │
│  │ Compression │ │  Encryption  │ │    ECC / Integrity     │  │
│  │  ZSTD/LZ4   │ │AES-256-GCM  │ │  Reed-Solomon / CRC32  │  │
│  │  + Adaptive │ │  + HKDF     │ │  + BLAKE3 aux digest   │  │
│  └─────────────┘ └──────────────┘ └────────────────────────┘  │
│                                                                 │
│  ┌─────────────┐ ┌──────────────┐ ┌────────────────────────┐  │
│  │  Columnar   │ │   Metadata   │ │      Fuzz Targets      │  │
│  │ Transpose   │ │ Footer I/O   │ │ header/footer/rowgroup  │  │
│  └─────────────┘ └──────────────┘ └────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
```

### Streaming Write Pipeline

```
Input Row
    │
    ▼
[Row Buffer per Row Group]         ← bounded memory O(row_group_size)
    │  [buffer full → flush]
    ▼
[Columnar Transpose]               ← row → column layout per group
    │
    ▼
[Per-Column Encoding]              ← PLAIN / RLE / DELTA / DICT_RLE / etc.
    │
    ▼
[Per-Chunk Compression]            ← ZSTD / LZ4 / adaptive selection
    │                              ← selalu sebelum enkripsi
    ▼
[AES-256-GCM Encryption]          ← opsional per-kolom, nonce unik per chunk
    │
    ▼
[CRC32 / Auth Tag Append]          ← integritas chunk sebelum flush
    │
    ▼
[Reed-Solomon ECC]                 ← opsional, parity chunks per row group
    │
    ▼
[Row Group Flush → File Stream]    ← append-only, tidak ada backtrack
    │
    (setelah semua row group)
    ▼
[File Footer Write]                ← schema, offsets, statistik, CRC32 footer
    │
    ▼
[FOOTER_LENGTH (4 bytes U32LE)]    ← 4 bytes terakhir file
```

> **Urutan pipeline ini adalah kontrak, bukan implementasi detail.** Kompresi harus selalu terjadi sebelum enkripsi — mengenkripsi data terlebih dahulu lalu mengompresi menghasilkan overhead tanpa manfaat rasio kompresi.

### Read Modes

```
File
 │
 ├── [1] Footer Parse (wajib pertama)
 │         FOOTER_LENGTH ← 4 bytes terakhir
 │         Footer content ← seek ke file_size - 4 - FOOTER_LENGTH
 │         CRC32 validation ← tolak jika mismatch
 │         Schema + Row group offsets + Statistics
 │
 ├── [2] Full Scan
 │         Iterate semua row group secara sekuensial
 │
 ├── [3] Partial Column Read
 │         Seek langsung ke column chunks yang diminta
 │         Skip semua column chunks yang tidak diminta
 │
 ├── [4] Row Group Projection
 │         Pilih row groups berdasarkan range atau predicate statistik
 │         Min/max statistics untuk skip row groups yang tidak relevan
 │
 └── [5] Footer-Only Inspection
           Schema + statistik tanpa membaca payload data apapun
           Cocok untuk discovery, cataloging, dan browser metadata display
```

### Memory Bounds (Formal)

```
Writer:
  peak_memory = row_group_size × avg_row_width_bytes
              + column_dict_overhead (untuk DICT_RLE kolom)
              + ecc_parity_overhead (bila ECC aktif)

Reader (partial column read):
  peak_memory = Σ(selected_column_chunk_size) × active_parallel_row_groups
              + footer_size (selalu dimuat)

Memory tidak pernah bergantung pada ukuran total file.
Constraint ini diverifikasi oleh memory regression tests di suite.
```

---

## 📄 Binary Format Specification

### File Layout

```
┌──────────────────────────────────────────┐
│            FILE HEADER (32 bytes)        │
│   MAGIC · VERSION · SCHEMA_ID · FLAGS    │
├──────────────────────────────────────────┤
│              ROW GROUP 0                 │
│  ┌────────────────────────────────────┐  │
│  │       Row Group Header             │  │
│  ├────────────────────────────────────┤  │
│  │  Col Chunk 0  [enc │ comp │ crc32] │  │
│  │  Col Chunk 1  [enc │ comp │ crc32] │  │
│  │  ...                               │  │
│  │  Col Chunk N  [enc │ comp │ crc32] │  │
│  ├────────────────────────────────────┤  │
│  │  [ECC Parity Chunks — optional]    │  │
│  ├────────────────────────────────────┤  │
│  │  Row Group Footer (mini)           │  │
│  └────────────────────────────────────┘  │
├──────────────────────────────────────────┤
│              ROW GROUP 1 ... N           │
├──────────────────────────────────────────┤
│              FILE FOOTER                 │
│  Schema · Offsets · Stats · CRC32        │
├──────────────────────────────────────────┤
│         FOOTER_LENGTH (4 bytes U32LE)    │
└──────────────────────────────────────────┘
```

### File Header (32 bytes) — Fixed, Tidak Pernah Berubah

```
Offset  Sz  Type    Field              Keterangan
──────  ──  ──────  ─────────────────  ──────────────────────────────────────────
0       4   U32LE   MAGIC              0x51 0x52 0x44 0x01  ("QRD\x01")
4       2   U16LE   VERSION_MAJOR      Perubahan breaking pada format binary
6       2   U16LE   VERSION_MINOR      Penambahan opsional yang backward-compatible
8       8   [U8;8]  SCHEMA_ID          8 bytes pertama SHA-256 dari schema serialized
16      4   U32LE   CREATED_AT         Unix timestamp seconds (creation time)
20      4   U32LE   TOTAL_ROW_COUNT    Total logical rows (0 jika streaming belum finish)
24      2   U16LE   COLUMN_COUNT       Jumlah kolom dalam schema
26      2   U16LE   ROW_GROUP_SIZE_K   Target rows per row group ÷ 1024
28      4   U32LE   FLAGS              Bit flags — lihat tabel FLAGS di bawah
```

**Tabel FLAGS:**

```
Bit 0   : ENCRYPTED          — File mengandung minimal satu kolom terenkripsi
Bit 1   : ECC_ENABLED        — File mengandung ECC parity chunks
Bit 2   : STATS_ENCRYPTED    — Statistik kolom terenkripsi (untuk ZK compliance)
Bit 3   : SCHEMA_SIGNED      — Schema footer ditandatangani (Ed25519)
Bit 4–31: RESERVED           — Reader harus ignore; writer harus set 0
```

### Column Chunk Layout

```
Offset   Sz  Type    Field                Keterangan
───────  ──  ──────  ───────────────────  ──────────────────────────────────────
0        1   U8      ENCODING_ID          ID algoritma encoding (lihat tabel)
1        1   U8      COMPRESSION_ID       ID codec kompresi (lihat tabel)
2        1   U8      ENCRYPTION_ID        0x00 = none, 0x01 = AES-256-GCM
3        1   U8      RESERVED             Harus 0x00; tolak jika non-zero
4        4   U32LE   UNCOMPRESSED_LEN     Ukuran payload sebelum kompresi
8        4   U32LE   COMPRESSED_LEN       Ukuran payload setelah kompresi
12       4   U32LE   NULL_COUNT           Jumlah nilai null dalam chunk
16       4   U32LE   DISTINCT_COUNT       Jumlah nilai unik (0 jika dienkripsi)
20       8   U64LE   ROW_OFFSET           Offset row pertama chunk ini dalam row group
                     [bila ENCRYPTION_ID != 0x00]:
28       12  [U8;12] NONCE               AES-GCM nonce (random, per-chunk)
40       16  [U8;16] AUTH_TAG            AES-GCM authentication tag
56       2   U16LE   KEY_ID_LEN          Panjang KEY_ID (0 jika tidak ada)
58       V   BYTES   KEY_ID              Identifier kunci (opsional)
                     [payload]:
?        B   BYTES   PAYLOAD             Encoded + compressed (+ encrypted) data
?+B      4   U32LE   CRC32               CRC32 dari uncompressed payload (sebelum enc)
```

### File Footer Structure

```
Footer Content (variable length)
─────────────────────────────────────────────────────
[footer_version: U16LE]                    ← versi struktur footer

[schema_section]
  [schema_length: U32LE]
  [schema_version: U16LE]
  [field_count: U16LE]
  For each field:
    [name_len: U16LE]
    [name: UTF-8 bytes]
    [logical_type_id: U8]
    [nullability_id: U8]
    [encoding_hint: U8]                    ← preferred encoding untuk kolom ini
    [compression_hint: U8]                 ← preferred codec untuk kolom ini
    [encryption_id: U8]                    ← 0x00=none, 0x01=AES-256-GCM
    [metadata_count: U16LE]
    For each metadata entry:
      [key_len: U16LE] [key: UTF-8]
      [value_len: U16LE] [value: UTF-8]

[row_group_section]
  [row_group_count: U32LE]
  For each row group:
    [byte_offset: U64LE]                   ← offset dari awal file
    [row_count: U32LE]                     ← jumlah rows dalam row group ini

[statistics_section]
  [statistics_flag: U8]                    ← 0x00=absent, 0x01=plaintext, 0x02=encrypted
  [statistics_length: U32LE]
  [statistics_bytes]                       ← per-kolom: min/max/null_count/distinct_count

[encryption_metadata]                      ← hanya bila FLAGS.ENCRYPTED = 1
  [key_derivation_algo: U8]               ← 0x01 = HKDF-SHA256
  [kdf_params_length: U16LE]
  [kdf_params_bytes]                       ← salt, info, output_len

[schema_signature]                         ← hanya bila FLAGS.SCHEMA_SIGNED = 1
  [sig_algo: U8]                          ← 0x01 = Ed25519
  [signature: 64 bytes]
  [public_key: 32 bytes]

[file_metadata_length: U32LE]
[file_metadata_bytes]                      ← key-value pairs opsional

[footer_checksum: U32LE]                  ← CRC32 seluruh footer content di atas
─────────────────────────────────────────────────────
[FOOTER_LENGTH: U32LE]                    ← 4 bytes terakhir file (big-endian bukan LE)
```

**Footer Parsing Protocol (wajib diikuti semua reader):**
1. Seek ke `file_size - 4`; baca `FOOTER_LENGTH` sebagai U32LE
2. Validasi `FOOTER_LENGTH < file_size - 32` (header size) — tolak jika tidak
3. Seek ke `file_size - 4 - FOOTER_LENGTH`
4. Baca `FOOTER_LENGTH` bytes sebagai footer content
5. Validasi CRC32 footer — tolak jika mismatch
6. Parse schema, row group offsets, statistik
7. Baca row groups menggunakan offsets dari footer

---

## 🔐 Encryption & Zero-Knowledge Model

### Definisi Zero-Knowledge dalam Konteks QRD

QRD menggunakan istilah "zero-knowledge" dalam arti storage-level: **server atau storage layer yang menyimpan file QRD tidak dapat memperoleh informasi tentang nilai plaintext kolom terenkripsi tanpa kunci dekripsi yang valid.**

Ini bukan zero-knowledge proof kriptografis (ZKP) dalam arti formal. Namun properti ini lebih kuat dari sekedar "encryption at rest" karena:

1. **Per-column key granularity** — kolom berbeda bisa dienkripsi dengan kunci berbeda
2. **Statistics encryption** — bila `FLAGS.STATS_ENCRYPTED = 1`, min/max/distinct_count tidak tersedia di footer tanpa dekripsi
3. **No server-side key** — HKDF key derivation dirancang sehingga server tidak perlu memegang kunci master

### Mengapa Cloud Deduplication Tidak Kompatibel

Format binary QRD menggunakan **nonce unik per chunk** yang di-generate secara random. Ini berarti:

```
File identik ditulis dua kali akan menghasilkan binary yang berbeda.
Karena nonce berbeda → ciphertext berbeda → hash file berbeda.

Konsekuensi terhadap cloud deduplication:
  Content-addressable deduplication TIDAK BEKERJA pada file QRD terenkripsi.
  Block-level deduplication TIDAK EFEKTIF pada ciphertext.
  
Ini adalah trade-off yang disengaja: keamanan semantik (IND-CPA)
mensyaratkan probabilistic encryption. Deduplication mensyaratkan
deterministic content — keduanya secara definisi tidak kompatibel.
```

**Implikasi praktis untuk operator storage:**
- Jangan mengandalkan deduplication untuk efisiensi storage QRD terenkripsi
- Gunakan compression pada level storage layer (bukan deduplication)
- Pertimbangkan chunking QRD berdasarkan row group boundary untuk storage efficiency

### Enkripsi Per-Kolom (AES-256-GCM)

```
Skema kunci per-kolom:

master_key (dipegang oleh client, tidak pernah ke server)
     │
     ▼  HKDF-SHA256(master_key, salt, info="qrd:col:{col_name}:{schema_id}")
column_key_N
     │
     ▼  AES-256-GCM(column_key_N, nonce_random_12bytes)
encrypted_payload
     │
     ▼
[NONCE (12 bytes)] [AUTH_TAG (16 bytes)] [CIPHERTEXT (B bytes)]

Auth tag membuktikan:
  1. Payload tidak dimodifikasi (integritas)
  2. Payload dienkripsi oleh pemegang column_key_N (authenticity)
```

**Struktur kolom terenkripsi dalam file:**
- Kolom yang tidak dienkripsi tetap plaintext dan dapat dibaca tanpa kunci
- Kolom terenkripsi hanya readable oleh pemegang kunci yang tepat
- Mix kolom terenkripsi dan plaintext dalam satu file adalah valid dan umum

### HKDF Key Derivation

```rust
// Derivasi kunci per-kolom dari master key
fn derive_column_key(
    master_key: &[u8; 32],
    salt: &[u8; 32],         // random per-file, disimpan di footer
    column_name: &str,
    schema_id: &[u8; 8],
) -> [u8; 32] {
    let info = format!("qrd:col:{}:{}", column_name, hex::encode(schema_id));
    hkdf::Hkdf::<sha2::Sha256>::new(Some(salt), master_key)
        .expand(info.as_bytes(), &mut output)
}
```

---

## 🛡 Security & Trust

### Model Kepercayaan

QRD dirancang dengan asumsi bahwa **tidak ada komponen di luar client yang dapat dipercaya sepenuhnya**. Ini termasuk:

- Storage server (cloud, on-premise)
- Transport layer (meskipun TLS digunakan)
- Intermediary processors
- Shared storage systems

Kepercayaan diberikan hanya kepada:
- **Kunci kriptografis** yang dipegang oleh client
- **Rust core engine** yang dapat diaudit dan di-reproduce
- **Format specification** yang publik dan deterministik

### CRC32 Integrity Verification

QRD memvalidasi integritas pada tiga level:

```
Level 1: Per column chunk
  CRC32(uncompressed_payload) disimpan di column chunk header
  Reader memverifikasi setelah dekompresi, sebelum dekoding
  Mendeteksi: storage corruption, transmission errors, partial writes

Level 2: AES-GCM Authentication Tag (untuk kolom terenkripsi)
  AUTH_TAG memverifikasi integritas DAN keaslian ciphertext
  Gagal jika ciphertext dimodifikasi (adversarial atau accidental)
  Lebih kuat dari CRC32 — unforgeable tanpa kunci

Level 3: Per file footer
  CRC32(footer_content) disimpan sebagai field terakhir footer
  Diverifikasi sebelum metadata apapun diparse
  Reader HARUS menolak file dengan footer CRC mismatch
```

### Reed-Solomon ECC

```
Konfigurasi ECC per Row Group:
  DATA_CHUNKS   : N column chunks (data aktual)
  PARITY_CHUNKS : K chunks tambahan (derived dari data)
  
  Recovery: hingga K chunk yang hilang atau korup dapat di-reconstruct

Cocok untuk:
  ✓ Cold storage jangka panjang (bit rot)
  ✓ Transmisi via kanal unreliable (lossy networks)
  ✓ Media degraded (HDD dengan bad sectors)
  ✓ Archival storage dengan SLA durability

Parameter tipikal:
  RS(32,8)   → toleransi 8 chunk korup dari 32 total
  RS(16,4)   → toleransi 4 chunk korup dari 16 total
```

### Parser Hardening (Zero-Panic Policy)

Core engine memiliki komitmen **zero-panic pada input adversarial**:

- Strict bounds check pada semua input eksternal sebelum digunakan
- Tolak header/footer dengan magic byte salah, size fields overflow, atau terpotong
- Fail-fast dengan error eksplisit pada encoding/compression ID yang tidak dikenal
- Semua `unsafe` Rust didokumentasi dengan safety invariant dalam komentar
- Integer overflow menggunakan `checked_*` arithmetic, bukan wrapping
- Fuzz targets aktif untuk semua parsing entrypoints

```rust
// Contoh parser hardening:
fn parse_footer_length(file: &mut impl Read + Seek) -> Result<u32> {
    let file_size = file.seek(SeekFrom::End(0))?;
    ensure!(file_size >= HEADER_SIZE + 4, Error::FileTooSmall { file_size });
    
    file.seek(SeekFrom::End(-4))?;
    let footer_len = file.read_u32::<LittleEndian>()?;
    
    ensure!(
        footer_len > 0 && footer_len <= file_size.saturating_sub(HEADER_SIZE + 4),
        Error::InvalidFooterLength { footer_len, file_size }
    );
    Ok(footer_len)
}
```

---

## 🚨 Threat Model

### Aset yang Dilindungi

| Aset | Nilai | Mekanisme Perlindungan |
|---|---|---|
| Plaintext payload kolom terenkripsi | Tinggi | AES-256-GCM per-column key |
| Schema dan field names kolom sensitif | Menengah | Optional schema signing + metadata omission |
| Statistik distribusi data terenkripsi | Menengah | `STATS_ENCRYPTED` flag, enkripsi bersama payload |
| Integritas format (non-adversarial) | Tinggi | CRC32 per-chunk dan per-footer |
| Integritas payload terenkripsi (adversarial) | Tinggi | AES-GCM authentication tag |
| Ketersediaan data di storage degraded | Menengah | Reed-Solomon ECC |

### Threat Actors dan Mitigasi

```
THREAT 1: Curious Storage Provider
  Deskripsi : Cloud storage membaca file QRD yang disimpan
  Mitigasi  : Kolom terenkripsi tidak dapat dibaca tanpa kunci
              Statistik terenkripsi tidak membocorkan distribusi
              Nama kolom dapat di-obfuscate via metadata
  Status    : ✅ Dimitigasi oleh desain format

THREAT 2: Passive Network Eavesdropper
  Deskripsi : Membaca file QRD dalam transit
  Mitigasi  : Di luar scope QRD — gunakan TLS untuk transport
              AES-GCM memberikan lapisan enkripsi redundan
  Status    : ⚠️ Partial — transport security di luar scope format

THREAT 3: Malicious File (Parser Attack)
  Deskripsi : File QRD crafted untuk menyebabkan panic/overflow/OOB
  Mitigasi  : Zero-panic policy, strict bounds checks
              Fuzz testing terhadap semua parse entrypoints
              Semua size fields divalidasi sebelum alokasi
  Status    : ✅ Dimitigasi oleh parser hardening

THREAT 4: Storage Corruption (Non-Adversarial)
  Deskripsi : Bit rot, media failure, partial write
  Mitigasi  : CRC32 per-chunk, AES-GCM auth tag, Reed-Solomon ECC
              Footer CRC32 untuk early-exit corruption detection
  Status    : ✅ Dimitigasi dengan ECC aktif

THREAT 5: Schema Tampering
  Deskripsi : Penyerang memodifikasi schema footer untuk mengubah tipe data
  Mitigasi  : Footer CRC32, opsional Ed25519 schema signature
              SCHEMA_ID di header untuk cross-validation
  Status    : ✅ Dimitigasi (signature opsional namun direkomendasikan)

THREAT 6: Key Exhaustion / Nonce Reuse
  Deskripsi : Nonce AES-GCM yang sama digunakan dua kali dengan kunci sama
  Mitigasi  : Nonce 12-byte di-generate secara cryptographic random per chunk
              Probabilitas collision: 1/(2^96) — dapat diabaikan
  Status    : ✅ Dimitigasi secara probabilistik
```

### Batasan Eksplisit Threat Model

QRD **tidak** melindungi terhadap:

- **Kunci yang dikompromikan** — jika master key bocor, semua kolom terenkripsi dapat dibaca
- **Timing side-channel** — library AES-GCM mungkin memiliki timing variance; untuk use case high-security, gunakan constant-time implementation yang telah diaudit
- **Metadata inference** — ukuran file, jumlah row group, dan nama kolom yang tidak diobfuscate dapat membocorkan informasi struktural
- **Attacker dengan akses memori runtime** — enkripsi at-rest tidak melindungi data yang sudah didekripsi di memori
- **Implementasi SDK yang tidak diaudit** — hanya Rust core engine yang memiliki jaminan keamanan penuh

---

## 🗃 Type System

### Numeric Types

| Type | Bytes | Range | Physical Representation |
|---|---|---|---|
| `BOOLEAN` | 1/8 | true / false | Bit-packed 8 per byte |
| `INT8` | 1 | -128 … 127 | Signed byte |
| `INT16` | 2 | -32,768 … 32,767 | Signed LE |
| `INT32` | 4 | -2³¹ … 2³¹-1 | Signed LE |
| `INT64` | 8 | -2⁶³ … 2⁶³-1 | Signed LE |
| `UINT8` | 1 | 0 … 255 | Unsigned byte |
| `UINT16` | 2 | 0 … 65,535 | Unsigned LE |
| `UINT32` | 4 | 0 … 2³²-1 | Unsigned LE |
| `UINT64` | 8 | 0 … 2⁶⁴-1 | Unsigned LE |
| `FLOAT32` | 4 | IEEE 754 single | 4-byte LE |
| `FLOAT64` | 8 | IEEE 754 double | 8-byte LE |

### Temporal Types

| Type | Bytes | Format | Contoh |
|---|---|---|---|
| `TIMESTAMP` | 8 | Unix microseconds UTC (INT64) | `1609459200000000` |
| `DATE` | 4 | Days since 1970-01-01 (INT32) | `18628` (2021-01-01) |
| `TIME` | 8 | Microseconds since 00:00:00 (INT64) | `43200000000` (12:00) |
| `DURATION` | 8 | Microseconds signed (INT64) | `3600000000` (1 jam) |

### Text & Binary Types

| Type | Format | Max Size | Catatan |
|---|---|---|---|
| `UTF8_STRING` | Variable length, length-prefixed U32LE | 4 GB per value | Encoding UTF-8 wajib |
| `ENUM` | Dictionary index U16LE + dict table | 65,535 nilai unik | Dictionary di footer |
| `UUID` | 16 bytes raw | 128-bit | RFC 4122, big-endian byte order |
| `BLOB` | Variable length, length-prefixed U32LE | 4 GB per value | Opaque binary |
| `DECIMAL` | Sign(1) + scale(1) + magnitude(variable) | Arbitrary precision | Exact numeric, no float error |

### Composite Types (Planned)

| Type | Deskripsi | Status |
|---|---|---|
| `STRUCT` | Named nested field set | Phase 3 |
| `ARRAY` | Homogeneous variable-length list | Phase 3 |
| `MAP` | Key-value pairs dengan typed key | Phase 4 |
| `ANY` | Escape hatch, schema validation disabled | Phase 4 |

### Nullability

| Value | Semantik | Null Bitmap | Storage Overhead |
|---|---|---|---|
| `REQUIRED` | Nilai null tidak diizinkan | Tidak ada | 0 bytes overhead |
| `OPTIONAL` | Bisa mengandung null | Present, bit-packed | ⌈N/8⌉ bytes per N rows |
| `REPEATED` | 0 atau lebih elemen per row | Present + offset array | Variable |

---

## ⚙️ Encoding Algorithms

Encoding diterapkan **sebelum kompresi dan sebelum enkripsi**. Tujuannya: transformasi nilai menjadi representasi yang lebih compressible. Setiap column chunk memiliki `ENCODING_ID` tersimpan di header.

### PLAIN (0x00)

Nilai disimpan dalam bentuk serialized mentah. Baseline, selalu valid.

```
[value_0][value_1]...[value_N]
```

Digunakan: data dengan entropy tinggi (hash, UUID, random float), atau data yang akan dikompres dengan ZSTD yang sudah optimal tanpa pre-encoding.

### RLE — Run-Length Encoding (0x01)

Pasangan `(run_length: U32LE, value: T)`. Efektif untuk low-cardinality atau sorted sequences.

```
(5, "active") → "active" berulang 5 kali
(3, 42)       → 42 berulang 3 kali
```

Digunakan: kolom status enum, boolean kolom sorted, sparse non-null indicators.

### BIT_PACKED (0x02)

Integer dan boolean dikemas rapat sesuai bit-width minimum yang dibutuhkan.

```
8 boolean values  → 1 byte
4-bit integers    → 2 nilai per byte (bit width dihitung dari max value)
Header: [bit_width: U8][packed_bits...]
```

Digunakan: boolean, small integer enums, 4-bit category codes.

### DELTA_BINARY (0x03)

Menyimpan selisih antar nilai integer berurutan. First value disimpan literal.

```
[100, 102, 105, 109] → [100, delta_min=-1000, bitwidth=4, deltas=[+2, +3, +4]]
Implementasi: Parquet DELTA_BINARY_PACKED compatible
```

Digunakan: timestamp monoton, auto-increment ID, counter sequences. **Rasio kompresi tipikal 4–8× vs PLAIN untuk timestamp monoton.**

### DELTA_BYTE_ARRAY (0x04)

Prefix sharing untuk byte array berurutan. Menyimpan `(shared_prefix_len, suffix)`.

```
["https://api.example.com/v1/users", "https://api.example.com/v1/orders"]
→ prefix_len=30, suffixes=["users", "orders"]
```

Digunakan: URL dengan shared prefix, file path, log prefix, kolom string kategorikal panjang.

### BYTE_STREAM_SPLIT (0x05)

Menyusun ulang bytes floating-point ke dalam stream terpisah per byte position. Meningkatkan compressibility float secara dramatis.

```
Original float32 stream:
  [f0_b0, f0_b1, f0_b2, f0_b3, f1_b0, f1_b1, f1_b2, f1_b3, ...]

Setelah split:
  stream-0: [f0_b0, f1_b0, f2_b0, ...]   ← exponent high bytes (sangat berulang)
  stream-1: [f0_b1, f1_b1, f2_b1, ...]
  stream-2: [f0_b2, f1_b2, f2_b2, ...]
  stream-3: [f0_b3, f1_b3, f2_b3, ...]   ← mantissa low bytes (lebih random)
```

Digunakan: sensor float readings, koordinat GPS, temperature series. **Kombinasi dengan ZSTD menghasilkan rasio tipikal 3–6× vs PLAIN float.**

### DICTIONARY_RLE (0x06)

Dictionary nilai unik disimpan di header encoding, lalu nilai di-encode sebagai index RLE.

```
Header: [dict_size: U16LE][dict_entries: value_0, value_1, ..., value_K]
Data:   [(run_len: U32LE, dict_index: U16LE), ...]

Dictionary: {0: "active", 1: "inactive", 2: "pending"}
Data:       [(3, 0), (2, 1), (1, 2)]   ← 6 rows dari 3 byte index
```

Digunakan: kolom kategorikal dengan cardinality rendah (status, region, device_type). **Optimal untuk cardinality < 1000 dengan repetition tinggi.**

### Encoding Selection Guide (Per Karakteristik Data)

| Karakteristik Data | Encoding Rekomendasi | Alasan |
|---|---|---|
| Timestamp monoton naik | `DELTA_BINARY` | Delta kecil → bit width rendah |
| Integer auto-increment | `DELTA_BINARY` | Delta = 1 → hampir zero overhead |
| Status / kategori (< 1000 unik) | `DICTIONARY_RLE` | Dictionary kecil, index rendah |
| Boolean | `BIT_PACKED` | 8× space reduction |
| Small integer (0–255) | `BIT_PACKED` (8-bit) | Tidak ada overhead |
| Sensor float readings | `BYTE_STREAM_SPLIT` + ZSTD | Byte stream compressible |
| URL / path dengan prefix | `DELTA_BYTE_ARRAY` | Prefix sharing eliminasi redundansi |
| Hash / UUID / random | `PLAIN` | High entropy, tidak bisa dikompres |
| Run panjang nilai sama | `RLE` | Space proportional ke unique values |
| Data acak / BLOB | `PLAIN` | Tidak ada transformasi bermanfaat |

---

## 🗜 Compression

### Filosofi Kompresi QRD

Kompresi di QRD memiliki tiga properti desain yang tidak dapat dikompromikan:

1. **Chunk-level independence** — setiap column chunk dapat didekompresi secara independen, memungkinkan parallel decompression dan partial reads
2. **Setelah encoding, sebelum enkripsi** — urutan ini kritis; mengompresi ciphertext tidak efektif
3. **No information leakage** — untuk kolom terenkripsi, ukuran compressed chunk tidak boleh menjadi oracle distribusi data (length padding dapat dipertimbangkan untuk use case high-security)

### Codec yang Didukung

| Codec | ID | Rasio Tipikal | Kompresi | Dekompresi | Use Case Optimal |
|---|---|---|---|---|---|
| `NONE` | 0x00 | 1.0× | — | — | Data pre-compressed (JPEG, audio) |
| `LZ4` | 0x01 | 1.5–2.5× | ~500 MB/s | ~3 GB/s | Streaming, low-latency, write-heavy |
| `ZSTD` | 0x02 | 2.0–6.0× | ~200 MB/s | ~1.5 GB/s | Archive, analytics, storage efficiency |
| `GZIP` | 0x03 | 2.0–4.0× | ~80 MB/s | ~400 MB/s | Reserved, kompatibilitas legacy |

> Angka di atas adalah tipikal pada workload analytical columnar setelah encoding. Hardware, data characteristics, dan compression level ZSTD mempengaruhi hasil aktual secara signifikan.

### Adaptive Codec Selection

QRD menyediakan adaptive mode yang memilih codec berdasarkan analisis entropy chunk:

```rust
fn select_codec(encoded_chunk: &[u8], config: &WriterConfig) -> Codec {
    match config.workload_profile {
        WorkloadProfile::LowLatencyStream => Codec::Lz4,
        WorkloadProfile::Archive         => Codec::Zstd { level: 6 },
        WorkloadProfile::Adaptive        => {
            // Estimasi entropy dari sample 4KB pertama
            let entropy = estimate_entropy(&encoded_chunk[..4096.min(encoded_chunk.len())]);
            if entropy > ENTROPY_THRESHOLD_HIGH {
                Codec::None   // Data mendekati random — kompresi tidak efektif
            } else if config.latency_sensitive {
                Codec::Lz4
            } else {
                Codec::Zstd { level: 3 }  // Level 3 = sweet spot rasio/speed
            }
        }
    }
}
```

### Chunk Independence Architecture

```
Row Group
├── Column 0 chunk  [DELTA_BINARY + ZSTD]    ← didekompresi sendiri
├── Column 1 chunk  [DICT_RLE + LZ4]         ← didekompresi sendiri
├── Column 2 chunk  [PLAIN + NONE]           ← tidak perlu dekompresi
└── Column N chunk  [BSS + ZSTD]             ← didekompresi sendiri
```

Setiap chunk memiliki `UNCOMPRESSED_LEN` dan `COMPRESSED_LEN` di header, sehingga reader dapat alokasi buffer tepat sebelum dekompresi tanpa buffering berlebih.

### Maximizing Compression Ratio

Strategi berlapis untuk rasio kompresi maksimal tanpa mengorbankan performance:

```
Tier 1: Pre-encoding (sebelum kompresi)
  → DELTA_BINARY untuk timestamp/integer monoton: eliminasi nilai besar
  → DICT_RLE untuk low-cardinality: string → index kecil
  → BIT_PACKED untuk boolean/small int: pack 8 values per byte
  → BYTE_STREAM_SPLIT untuk float: pisahkan byte streams

Tier 2: Compression (setelah encoding)
  → ZSTD level 3–6 untuk archive workloads
  → LZ4 untuk streaming workloads (dekompresi ~3 GB/s)

Tier 3: Row Group Size Tuning
  → Larger row groups = more data per encoding pass = better dict/delta
  → Target: 50K–500K rows untuk archive, 5K–50K untuk streaming

Tier 4: Column Ordering (opsional)
  → Urutkan kolom dari highest repetition ke lowest
  → Berpengaruh pada dict size dan RLE run length
```

**Contoh rasio nyata (benchmark dataset sensor IoT):**

| Kolom | Tipe | Encoding | Codec | Rasio |
|---|---|---|---|---|
| `device_id` | ENUM | DICT_RLE | ZSTD-3 | 18× |
| `timestamp` | TIMESTAMP | DELTA_BINARY | LZ4 | 12× |
| `temperature` | FLOAT32 | BYTE_STREAM_SPLIT | ZSTD-3 | 4.8× |
| `status` | ENUM (5 values) | DICT_RLE | LZ4 | 32× |
| `raw_payload` | BLOB | PLAIN | NONE | 1× |

---

## 🚀 Quick Start

### Prerequisites

- [Rust](https://rustup.rs/) toolchain 1.75+ (2021 Edition)
- `cargo` tersedia di PATH
- Untuk Python SDK: Python 3.9+
- Untuk TypeScript SDK: Node.js 18+

### Clone & Build

```bash
git clone https://github.com/zenipara/QRD-SDK.git
cd QRD-SDK
cargo build --workspace --release
```

### Run Tests

```bash
# Core engine unit tests
cargo test --package qrd-core

# Full workspace
cargo test --workspace

# Property-based tests (proptest)
cargo test --package qrd-core -- proptest

# Cross-language golden vector validation
cargo test --package qrd-core -- golden
```

### Run Benchmarks

```bash
# Semua benchmark
cargo bench --package qrd-core

# Benchmark spesifik
cargo bench --package qrd-core -- encode
cargo bench --package qrd-core -- streaming
cargo bench --package qrd-core -- compression
cargo bench --package qrd-core -- encryption
```

### Validate & Lint

```bash
cargo clippy --workspace -- -D warnings
cargo fmt --all -- --check
```

---

## 💻 Code Examples

### Rust — Streaming Write dengan Enkripsi

```rust
use qrd_core::{
    Schema, SchemaField, LogicalType, Nullability,
    StreamingWriter, WriterConfig,
    Compression, Encryption, MasterKey,
};
use std::fs::File;
use std::io::BufWriter;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let schema = Schema::builder()
        .field(SchemaField::new("device_id",  LogicalType::ENUM,      Nullability::Required))
        .field(SchemaField::new("timestamp",  LogicalType::TIMESTAMP, Nullability::Required))
        .field(SchemaField::new("latitude",   LogicalType::FLOAT64,   Nullability::Optional))
        .field(SchemaField::new("longitude",  LogicalType::FLOAT64,   Nullability::Optional))
        .field(SchemaField::new("health_val", LogicalType::FLOAT32,   Nullability::Optional))
        .build()?;

    // Kunci master dipegang oleh client — tidak pernah dikirim ke server
    let master_key = MasterKey::from_env("QRD_MASTER_KEY")?;

    let config = WriterConfig::builder()
        .row_group_size(50_000)
        .compression(Compression::Zstd { level: 3 })
        // Hanya kolom sensitif yang dienkripsi; device_id dan timestamp tetap plaintext
        // untuk efisiensi query dan deduplication metadata
        .encrypt_columns(&["latitude", "longitude", "health_val"], &master_key)
        .ecc(true)
        .build()?;

    let file = BufWriter::new(File::create("telemetry.qrd")?);
    let mut writer = StreamingWriter::new(file, schema, config)?;

    for record in sensor_stream() {
        writer.write_row(vec![
            Value::Enum(record.device_id),
            Value::Timestamp(record.ts_micros),
            Value::Float64(record.lat),
            Value::Float64(record.lon),
            Value::Float32(record.health),
        ])?;
    }

    // Wajib — menulis footer dan memfinalisasi file
    writer.finish()?;
    Ok(())
}
```

### Rust — Partial Column Read dengan Dekripsi

```rust
use qrd_core::{FileReader, MasterKey};
use std::fs::File;
use std::io::BufReader;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let master_key = MasterKey::from_env("QRD_MASTER_KEY")?;
    let reader = FileReader::builder()
        .decryption_key(&master_key)
        .open(BufReader::new(File::open("telemetry.qrd")?))?;

    println!("Schema: {:?}", reader.schema());
    println!("Total rows: {}", reader.row_count());

    // Hanya baca device_id dan health_val (skip latitude, longitude)
    // health_val akan otomatis didekripsi menggunakan master_key
    let columns = reader.read_columns(&["device_id", "health_val"])?;

    for (device, health) in columns[0].iter().zip(columns[1].iter()) {
        println!("{}: {:.2}", device, health);
    }

    Ok(())
}
```

### Rust — Integrity Verification

```rust
let mut reader = FileReader::open(BufReader::new(File::open("telemetry.qrd")?))?;

match reader.verify_integrity() {
    Ok(report) => {
        println!("CRC32: {}", if report.crc_ok { "OK" } else { "FAIL" });
        println!("Auth tags: {}/{} valid", report.auth_tags_valid, report.auth_tags_total);
        println!("ECC: {}", if report.ecc_ok { "OK" } else { "ECC correction applied" });
    }
    Err(e) => eprintln!("Corruption detected: {}", e),
}
```

### Python

```python
import qrd
import os

schema = (qrd.SchemaBuilder()
    .add_field("device_id",  qrd.FieldType.ENUM,      qrd.Nullability.REQUIRED)
    .add_field("timestamp",  qrd.FieldType.TIMESTAMP, qrd.Nullability.REQUIRED)
    .add_field("health_val", qrd.FieldType.FLOAT32,   qrd.Nullability.OPTIONAL)
    .build())

master_key = qrd.MasterKey.from_env("QRD_MASTER_KEY")

# Write dengan enkripsi per-kolom
writer = qrd.FileWriter("telemetry.qrd", schema,
    compression=qrd.Compression.ZSTD,
    encrypt_columns=["health_val"],
    master_key=master_key)

for record in sensor_stream():
    writer.write_row({
        "device_id": record["device_id"],
        "timestamp": record["ts_micros"],
        "health_val": record["health"],
    })
writer.finish()

# Read dengan partial column selection
reader = qrd.FileReader("telemetry.qrd", master_key=master_key)
columns = reader.read_columns(["device_id", "health_val"])
```

### TypeScript / WASM (Browser)

```typescript
import { initWasm, SchemaBuilder, FileWriter, FileReader, MasterKey } from 'qrd-sdk/browser';

await initWasm();

// Inspeksi metadata tanpa load payload — cocok untuk browser offline-first
const buffer = await fetch('/data/telemetry.qrd').then(r => r.arrayBuffer());
const meta = qrd.inspectFooter(new Uint8Array(buffer));

console.log(`${meta.rowCount} rows, ${meta.schema.fields.length} columns`);
console.log('Encrypted columns:', meta.schema.fields
    .filter(f => f.isEncrypted)
    .map(f => f.name));

// Baca kolom plaintext tanpa kunci
const reader = new FileReader(new Uint8Array(buffer));
const deviceIds = reader.readColumn("device_id");

// Baca kolom terenkripsi dengan kunci dari user (tidak pernah ke server)
const masterKey = MasterKey.fromUserInput(await promptUserForKey());
const healthData = reader.readColumn("health_val", { masterKey });
```

### Go

```go
package main

import (
    "fmt"
    "os"
    qrd "github.com/zenipara/QRD-SDK/sdk/go"
)

func main() {
    schema := qrd.NewSchemaBuilder().
        AddField("device_id",  qrd.FieldTypeEnum,    qrd.NullabilityRequired).
        AddField("timestamp",  qrd.FieldTypeI64,     qrd.NullabilityRequired).
        AddField("health_val", qrd.FieldTypeFloat32, qrd.NullabilityOptional).
        Build()

    masterKey, _ := qrd.MasterKeyFromEnv("QRD_MASTER_KEY")

    writer, _ := qrd.NewFileWriter("telemetry.qrd", schema,
        qrd.WithCompression(qrd.CompressionZstd),
        qrd.WithEncryptedColumns([]string{"health_val"}, masterKey))
    defer writer.Close()

    writer.WriteRow(map[string]interface{}{
        "device_id": "sensor-001",
        "timestamp": int64(1700000000000000),
        "health_val": float32(98.6),
    })
    writer.Finish()

    reader, _ := qrd.NewFileReader("telemetry.qrd", qrd.WithMasterKey(masterKey))
    defer reader.Close()
    fmt.Printf("Rows: %d\n", reader.RowCount())
}
```

### Best Practices

```rust
// 1. SELALU panggil finish() — footer tidak ditulis tanpa ini
writer.finish()?;

// 2. Tuning row_group_size sesuai workload:
//    Constraint memory: 5_000–10_000 rows
//    Streaming edge:    20_000–50_000 rows (default)
//    Archive batch:    200_000–500_000 rows

// 3. Enkripsi hanya kolom sensitif — bukan semua kolom
//    Kolom plaintext tetap bisa diquery, diindex, dan dideduplikasi
//    Kolom terenkripsi memerlukan kunci — rencanakan distribusi kunci

// 4. Gunakan batch writes untuk performa optimal
let mut batch = Vec::with_capacity(50_000);
for row in data_source {
    batch.push(row);
    if batch.len() == 50_000 {
        writer.write_rows(&batch)?;
        batch.clear();
    }
}

// 5. Partial reads untuk analytical queries
let cols = reader.read_columns(&["timestamp", "device_id"])?;
// Kolom terenkripsi yang tidak diminta tidak akan didekripsi

// 6. Verify setelah write untuk critical data
let mut verify_reader = FileReader::open(file)?;
verify_reader.verify_integrity()?;
```

---

## 🌐 Multi-Language SDK

### Status SDK

| Language | Path | Mekanisme | Package | Status |
|---|---|---|---|---|
| **Rust** | `core/qrd-core/` | Native | `qrd-core` (crates.io) | Stable / Reference |
| **Python** | `sdk/python/` | PyO3 | `qrd-sdk` (PyPI) | Stable |
| **TypeScript** | `sdk/typescript/` | WASM | `qrd-sdk` (npm) | Stable |
| **Go** | `sdk/go/` | CGO | `github.com/zenipara/QRD-SDK/sdk/go` | Stable |
| **Java** | `sdk/java/` | JNI | Maven `io.qrd:qrd-core` | Stable |
| **C/C++** | `core/qrd-ffi/` | C FFI | Header + static lib | Stable |

Semua SDK menggunakan Rust core engine yang sama via FFI/WASM. Tidak ada implementasi mandiri dalam bahasa lain — ini adalah jaminan fidelitas format.

### Instalasi

**Rust**
```bash
cargo add qrd-core
# atau dalam Cargo.toml: qrd-core = "1.0"
```

**Python**
```bash
pip install qrd-sdk
```

**TypeScript / WASM**
```bash
npm install qrd-sdk
```

**Go**
```bash
go get github.com/zenipara/QRD-SDK/sdk/go@v1
```

**Java**
```xml
<dependency>
  <groupId>io.qrd</groupId>
  <artifactId>qrd-core</artifactId>
  <version>1.0.0</version>
</dependency>
```

**C/C++**
```bash
cargo build --package qrd-ffi --release
# Header: core/qrd-ffi/include/qrd.h
# Library: target/release/libqrd_ffi.a
```

---

## 🧪 Test Suite

### Target: 10.000+ Test Cases

QRD menargetkan **minimal 10.000 test cases** yang mencakup seluruh surface area format. Angka ini bukan vanity metric — setiap kategori di bawah memiliki justifikasi mengapa densitas pengujian tersebut diperlukan untuk format binary kriptografis yang digunakan di lingkungan produksi.

### Struktur Test Suite

```
tests/
├── unit/                          # ~2.500 tests
│   ├── schema/                    # Schema build, serialize, fingerprint
│   ├── encoding/                  # Per-algoritma: PLAIN, RLE, DELTA, BIT_PACKED, etc.
│   ├── compression/               # ZSTD, LZ4, adaptive selection, empty chunk
│   ├── encryption/                # AES-GCM correctness, auth tag, key derivation
│   ├── ecc/                       # Reed-Solomon encode/decode/recover
│   ├── parser/                    # Header, footer, column chunk parsing
│   └── integrity/                 # CRC32 per-chunk, footer checksum
│
├── property/                      # ~2.000 tests (proptest)
│   ├── roundtrip/                 # write → read → same data, semua tipe
│   ├── streaming/                 # Arbitrary row counts, arbitrary row group sizes
│   ├── partial_read/              # Arbitrary column selection always consistent
│   ├── encoding_correctness/      # Per-encoding: arbitrary input → decode(encode(x)) == x
│   ├── compression_roundtrip/     # Arbitrary bytes → decompress(compress(x)) == x
│   └── schema_compatibility/      # Compatible schema changes preserve readability
│
├── golden/                        # ~1.500 tests
│   ├── vectors/                   # Binary vector files per format version
│   │   ├── v1.0/                  # Canonical .qrd files dengan expected output JSON
│   │   └── cross-lang/            # File dibuat oleh satu SDK, dibaca oleh semua
│   ├── encoding_vectors/          # Per-encoding: input bytes → expected encoded bytes
│   └── encryption_vectors/        # NIST AES-GCM test vectors + custom QRD vectors
│
├── integration/                   # ~1.500 tests
│   ├── cross_language/            # Rust write → Python read → Go read → TS read
│   ├── streaming_scenarios/       # 1 row, 1M rows, empty file, single column
│   ├── partial_column_reads/      # Correctness + memory bound verification
│   ├── encryption_e2e/            # Write terenkripsi → baca dengan kunci → verify
│   ├── ecc_recovery/              # Simulasi chunk corruption → ECC recovery
│   └── schema_evolution/          # Backward/forward compatibility scenarios
│
├── fuzz/                          # Continuous (libfuzzer + honggfuzz)
│   ├── parse_header/              # Arbitrary bytes sebagai file header
│   ├── parse_footer/              # Arbitrary bytes sebagai file footer
│   ├── parse_column_chunk/        # Arbitrary bytes sebagai column chunk
│   ├── decode_rle/                # Arbitrary bytes melalui RLE decoder
│   ├── decode_delta/              # Arbitrary bytes melalui DELTA decoder
│   └── decrypt_chunk/             # Arbitrary ciphertext melalui AES-GCM (no panic)
│
├── regression/                    # ~500 tests
│   ├── memory_bounds/             # Writer/reader tidak melebihi target memory bound
│   ├── performance/               # Throughput tidak turun > 10% dari baseline
│   └── bug_corpus/                # Test case dari setiap bug yang ditemukan
│
└── compliance/                    # ~500 tests
    ├── nist_aes_gcm/              # NIST AES-GCM Known Answer Tests
    ├── crc32_vectors/             # CRC32 dengan known polynomial vectors
    ├── utf8_validation/           # Reject invalid UTF-8 dalam field names
    └── little_endian/             # Verifikasi canonical byte order semua integer
```

### Coverage Requirements

| Kategori | Minimum Coverage | Metrik |
|---|---|---|
| Core parser paths | 100% | Branch coverage di `parser/` |
| Encoding roundtrip | 100% | Semua tipe × semua encoding valid |
| Crypto primitives | 100% | Function coverage, NIST vectors |
| Error paths | 95% | Semua error variant harus dapat di-trigger |
| FFI bindings | 90% | Line coverage per bahasa |
| WASM module | 85% | Statement coverage (via WASM test runner) |

### Menjalankan Test Suite

```bash
# Unit tests
cargo test --package qrd-core

# Property tests dengan lebih banyak cases (CI default: 1000)
PROPTEST_CASES=10000 cargo test --package qrd-core -- proptest

# Golden vector tests
cargo test --package qrd-core -- golden

# Cross-language integration tests
./scripts/run_cross_lang_tests.sh

# Fuzz targets (membutuhkan nightly + cargo-fuzz)
cargo +nightly fuzz run parse_header -- -max_total_time=300
cargo +nightly fuzz run parse_footer -- -max_total_time=300

# Memory regression tests
cargo test --package qrd-core -- memory_bounds -- --nocapture

# Full suite dengan coverage report
cargo llvm-cov test --workspace --html
```

### Golden Vector Protocol

Golden vectors adalah file `.qrd` binary canonical yang disimpan di repository:

```
tests/golden/vectors/v1.0/
├── minimal_schema.qrd             # 1 kolom INT32, 10 rows, no compression
├── all_types_plaintext.qrd        # Semua tipe, ZSTD, no encryption
├── encrypted_columns.qrd          # Mix plain + encrypted, known key
├── ecc_enabled.qrd                # Dengan parity chunks RS(16,4)
├── large_row_groups.qrd           # 500K rows per group
└── expected/
    ├── minimal_schema.json        # Expected decoded content
    ├── all_types_plaintext.json
    └── ...
```

Setiap PR yang mengubah format binary harus menyertakan golden vector baru. Reader dari versi sebelumnya harus tetap dapat membaca golden vector lama.

---

## 📊 Benchmarks

### Design Targets (Diukur pada Modern Server Hardware)

| Operasi | Dataset | Target |
|---|---|---|
| Write throughput (no encryption) | 1 KB row, LZ4 | 1–5 GB/s |
| Write throughput (AES-256-GCM) | 1 KB row, LZ4 | 500 MB–2 GB/s |
| Full scan read | 100 MB dense | 2–10 GB/s |
| Partial column read (10% columns) | 1 GB dataset | 5–20 GB/s |
| ZSTD compression ratio (integer + timestamp) | Sensor dataset | 4–12× |
| ZSTD compression ratio (float BSS) | Sensor float | 3–6× |
| LZ4 compression overhead | Streaming | < 10% vs NONE |
| Footer parse latency | 1 GB file | < 1 ms |
| WASM write (browser, no encryption) | 10K rows | < 100 ms |

> Target ini adalah referensi desain. Selalu jalankan benchmark pada hardware target Anda. Output benchmark Criterion menyimpan baseline di `.criterion/` untuk perbandingan regresi otomatis.

### Menjalankan Benchmark

```bash
# Semua benchmark dengan Criterion
cargo bench --package qrd-core

# Benchmark spesifik
cargo bench --package qrd-core -- encode
cargo bench --package qrd-core -- streaming
cargo bench --package qrd-core -- compression
cargo bench --package qrd-core -- encryption
cargo bench --package qrd-core -- partial_read

# Dengan output verbose (tampilkan semua iterations)
cargo bench --package qrd-core -- --nocapture

# Bandingkan dengan baseline sebelumnya
cargo bench --package qrd-core -- --baseline main
```

**Setiap PR yang mengklaim perubahan performa harus menyertakan:**
- Spesifikasi hardware (CPU, RAM, storage type)
- Versi Rust toolchain
- Output Criterion sebelum dan sesudah
- Methodology sampling (warmup, iterations, statistical significance)

---

## 🧭 Use Cases

### Edge & IoT Telemetry (Privacy-Sensitive)

```
Health Sensor → [QRD Writer, bounded memory]
    Kolom terenkripsi: heart_rate, spo2, location
    Kolom plaintext: device_id, timestamp
    LZ4 compression, ECC enabled
         │
         ▼ upload via TLS
    [Cloud Storage]
    Server menyimpan ciphertext — tidak dapat membaca health_rate
         │
         ▼ dengan kunci
    [Authorized Client / Analytics Pipeline]
    Dekripsi dan analitik hanya oleh pemegang kunci
```

### Browser Analytics (Zero-Server-Trust)

```
Browser → [WASM QRD Writer]
    Data tidak pernah meninggalkan browser dalam plaintext
    File .qrd diunduh atau disimpan di IndexedDB
         │
         ▼ optional upload
    [Server]
    Server menerima ciphertext — tidak ada akses plaintext
         │
         ▼ WASM QRD Reader
    [Browser dengan kunci pengguna]
    Analitik terjadi entirely di browser
```

### Edge AI / ML Inference

```
Feature Store (.qrd) — kolom terenkripsi per feature group
         │
         ▼ partial column read
    Selected features (sesuai model)
         │
         ▼ ML inference pipeline
    Prediksi lokal — model tidak membutuhkan server
         ▼ optional
    Hasil dikembalikan ke cloud
```

### Audit & Compliance Logging

```
Audit Event → [QRD Writer]
    Schema deterministik, CRC32 per-event-chunk
    Schema signature (Ed25519) untuk non-repudiation
    Immutable row groups — tidak ada in-place edit
         │
         ▼
    [Audit Storage]
    Format self-describing: schema audit trail tanpa registry
    Kriptografis verifiable: setiap record dapat divalidasi
```

### Cross-Language Data Exchange (No Drift Guarantee)

```
Rust producer   → output.qrd →   Python ML consumer
                              →   Go API consumer
                              →   TypeScript (browser dashboard)

Satu format. Satu engine. Binary identik di semua consumer.
Tidak ada serialization drift antar bahasa.
```

---

## 🔒 Security & Trust

### Cryptography Audit

QRD berkomitmen untuk audit kriptografis independen sebelum setiap major release. Audit mencakup:

**Scope Audit:**
- AES-256-GCM implementation: correctness, nonce generation, auth tag verification
- HKDF key derivation: parameter selection, domain separation, output size
- CRC32 implementation: polynomial, endianness, collision resistance awareness
- Reed-Solomon: galois field arithmetic, encoding/decoding correctness
- Parser hardening: all external input paths, integer overflow, OOB access

**Standar yang Direferensikan:**
- NIST SP 800-38D (AES-GCM)
- RFC 5869 (HKDF)
- IEEE 802.3 (CRC32 Ethernet polynomial 0xEDB88320)
- RFC 6330 (Raptor codes, informational reference)
- FIPS 140-3 Level 1 alignment (operasional, bukan sertifikasi)

**Primitive yang Digunakan:**

| Fungsi | Library Rust | Justifikasi |
|---|---|---|
| AES-256-GCM | `aes-gcm` (RustCrypto) | Constant-time, NIST validated algorithm |
| HKDF-SHA256 | `hkdf` + `sha2` (RustCrypto) | RFC 5869 conformant |
| SHA-256 (schema fingerprint) | `sha2` (RustCrypto) | Industry standard, collision resistant |
| CRC32 (integrity) | `crc32fast` | Hardware-accelerated, non-cryptographic |
| CSPRNG (nonce) | `rand::rngs::OsRng` | OS entropy source, platform-appropriate |
| Ed25519 (optional signature) | `ed25519-dalek` | RFC 8032, fast and secure |

**Audit Reports:** Lihat [`docs/security/SECURITY_AUDIT.md`](docs/security/SECURITY_AUDIT.md)

### Responsible Disclosure

Vulnerability dilaporkan melalui: `security@qrd.dev`

PGP key tersedia di [`SECURITY.md`](SECURITY.md). Response target: **48 jam acknowledgment**, **7 hari** untuk severity tinggi. Lihat [`SECURITY.md`](SECURITY.md) untuk kebijakan lengkap.

---

## 🔄 Compatibility & Versioning

### Semantic Versioning

```
MAJOR.MINOR.PATCH

MAJOR → Perubahan format binary atau API yang tidak backward-compatible
MINOR → Fitur baru yang backward-compatible (field opsional, codec baru)
PATCH → Bug fix tanpa perubahan format atau API publik
```

### Format Version Compatibility Matrix

| Skenario | Behavior |
|---|---|
| Reader versi sama dengan writer | Fully compatible |
| Reader MAJOR < writer MAJOR | Reject: return `Error::UnsupportedMajorVersion` |
| Reader MINOR < writer MINOR | Ignore unknown optional fields; partial support |
| Unknown ENCODING_ID | Fail-fast dengan `Error::UnknownEncoding { id }` |
| Unknown COMPRESSION_ID | Fail-fast dengan `Error::UnknownCompression { id }` |
| Unknown FLAGS bit | Ignore bila di atas bit 3; warn bila bit 0–3 |
| Korup CRC32 (chunk) | Reject chunk: `Error::ChunkChecksumMismatch` |
| Korup CRC32 (footer) | Reject file: `Error::FooterChecksumMismatch` |
| AES-GCM auth tag fail | Reject: `Error::AuthenticationFailed` (tidak expose detail) |

### Schema Compatibility

| Perubahan Schema | Kompatibel? | Efek pada SCHEMA_ID |
|---|---|---|
| Tambah kolom OPTIONAL di akhir | Ya, backward-compatible | Berubah |
| Tambah optional metadata field | Ya | Tidak berubah |
| Rename field | Tidak — Breaking | Berubah |
| Ubah tipe field | Tidak — Breaking | Berubah |
| Ubah REQUIRED → OPTIONAL | Tidak — Breaking | Berubah |
| Ubah OPTIONAL → REQUIRED | Tidak — Breaking | Berubah |
| Ubah urutan kolom | Tidak — Breaking | Berubah |

---

## 📁 Repository Structure

```
QRD-SDK/
│
├── core/
│   ├── qrd-core/                  # Rust core engine — implementasi referensi
│   │   ├── src/
│   │   │   ├── schema/            # Schema builder, serialization, SHA-256 fingerprint
│   │   │   ├── writer/            # StreamingWriter, row group flush, footer write
│   │   │   ├── reader/            # FileReader, partial reads, footer parse
│   │   │   ├── encoding/          # PLAIN, RLE, BIT_PACKED, DELTA_*, DICT_RLE, BSS
│   │   │   ├── compression/       # ZSTD, LZ4, adaptive selection, entropy estimation
│   │   │   ├── encryption/        # AES-256-GCM, HKDF, nonce management, key derivation
│   │   │   ├── ecc/               # Reed-Solomon encode, decode, recovery
│   │   │   ├── columnar/          # Row-to-column transposition
│   │   │   ├── integrity/         # CRC32 computation dan verification
│   │   │   └── error/             # Error types, structured error taxonomy
│   │   ├── benches/               # Criterion benchmark suite
│   │   └── examples/              # Usage examples per feature
│   │
│   ├── qrd-ffi/                   # C-compatible FFI layer (stable ABI)
│   │   ├── src/                   # Thin wrapper, opaque pointer management
│   │   └── include/qrd.h          # C header file (canonical ABI contract)
│   │
│   └── qrd-wasm/                  # WebAssembly target
│       ├── src/                   # wasm-bindgen bindings
│       └── pkg/                   # Generated WASM + JS glue
│
├── sdk/
│   ├── python/                    # PyO3 Python binding
│   │   ├── src/                   # Rust PyO3 code
│   │   ├── qrd/                   # Python package
│   │   └── tests/                 # Python-specific tests
│   ├── typescript/                # WASM + TypeScript packaging
│   │   ├── src/                   # TypeScript wrapper + types
│   │   └── tests/                 # TS-specific tests
│   ├── go/                        # CGO Go binding
│   │   ├── qrd.go                 # Go package
│   │   └── qrd_test.go
│   └── java/                      # JNI Java binding
│       ├── src/main/              # Java package
│       └── src/test/
│
├── tests/
│   ├── unit/                      # Unit tests per komponen
│   ├── property/                  # Proptest property-based tests
│   ├── golden/                    # Golden vector files + expected output
│   ├── integration/               # Cross-language + E2E tests
│   ├── fuzz/                      # Fuzzing targets (libfuzzer)
│   ├── regression/                # Memory bounds + perf regression
│   └── compliance/                # NIST vectors + compliance checks
│
├── docs/
│   ├── FORMAT_SPEC.md             # Binary format specification (canonical)
│   ├── architecture/
│   │   └── ARCHITECTURE.md        # System design & component overview
│   ├── security/
│   │   ├── SECURITY_AUDIT.md      # Audit scope, results, dan remediation
│   │   ├── THREAT_MODEL.md        # Threat analysis & mitigation
│   │   ├── CRYPTOGRAPHY.md        # Crypto primitive choices & justification
│   │   └── FUZZING.md             # Fuzz coverage targets & corpus
│   ├── sdk/
│   │   └── SDKS.md                # Language binding status & install guide
│   ├── benchmarks/
│   │   └── BENCHMARKS.md          # Methodology, hardware specs, results
│   ├── STREAMING_MODEL.md         # Streaming write/read semantics
│   ├── MEMORY_MODEL.md            # Bounded-memory guarantees & proofs
│   ├── COMPRESSION.md             # Kompresi: filosofi, codec guide, tuning
│   ├── ENCRYPTION.md              # Enkripsi: model, key management, ZK
│   ├── EDGE_AI.md                 # Edge AI & telemetry guidance
│   ├── WASM.md                    # WASM & browser runtime docs
│   ├── STABILITY.md               # Compatibility & deprecation policy
│   ├── VERSIONING.md              # Semantic versioning policy
│   ├── PERFORMANCE.md             # Performance philosophy & profiling guide
│   ├── COMPATIBILITY.md           # Cross-version compatibility rules
│   ├── COMPETITOR_COMPARISON.md   # Format comparison & positioning
│   ├── DEPLOYMENT.md              # Deployment patterns & operational guidance
│   └── USE_CASES.md               # Extended use case documentation
│
├── examples/                      # Top-level usage examples per SDK + use case
├── benches/                       # Top-level benchmark aggregation
├── specs/                         # Format spec supplements & extension proposals
├── tools/
│   ├── qrd-inspect/               # CLI: inspect footer, schema, stats tanpa full read
│   ├── qrd-verify/                # CLI: verify integrity semua chunk + ECC check
│   ├── qrd-convert/               # CLI: konversi CSV/Parquet → QRD (satu arah)
│   └── qrd-keygen/                # CLI: generate master key dengan entropy yang tepat
│
├── Cargo.toml                     # Workspace manifest
├── Makefile                       # Common dev commands
├── CHANGELOG.md                   # Version history
├── CONTRIBUTING.md                # Contribution guide
├── SECURITY.md                    # Vulnerability reporting & PGP key
└── LICENSE                        # Business Source License 1.1
```

---

## 🗺 Evolution Roadmap

Roadmap QRD terorganisir per phase berdasarkan kematangan, bukan tanggal. Setiap phase memiliki exit criteria yang harus terpenuhi sebelum phase berikutnya dimulai.

### Phase 1 — Foundation (Current)

**Exit criteria:**
- [ ] Rust core engine dengan semua 7 encoding algorithms
- [ ] ZSTD dan LZ4 compression, adaptive selection
- [ ] AES-256-GCM per-column encryption dengan HKDF
- [ ] Reed-Solomon ECC
- [ ] CRC32 per-chunk dan per-footer
- [ ] C FFI layer (stable ABI)
- [ ] WASM target (browser + Node.js)
- [ ] Python, TypeScript, Go, Java SDKs
- [ ] Criterion benchmark suite
- [ ] Test suite mencapai 10.000 test cases
- [ ] Fuzzing corpus: 100K+ corpus entries per target
- [ ] Audit kriptografis oleh firma independen

### Phase 2 — Hardening & Compliance

**Focus:** Production readiness untuk regulated industries

**Deliverables:**
- FIPS 140-3 Level 1 alignment verification (operasional, bukan sertifikasi penuh)
- Constant-time AES-GCM verification path (mitigasi timing side-channel)
- Formal spec dalam format RFC-style untuk third-party implementors
- Schema signing via Ed25519 sebagai fitur stabil
- `qrd-inspect`, `qrd-verify`, `qrd-convert` tools menjadi production-ready
- Panduan deployment untuk healthcare (HIPAA), keuangan (SOC 2), dan edge telemetry
- Bahasa tambahan: Swift (iOS edge), Kotlin/Android, .NET/C#

### Phase 3 — Composite Types & Query Layer

**Focus:** Expressiveness format dan analytical capability

**Deliverables:**
- `STRUCT` dan `ARRAY` composite types dalam format binary
- Predicate pushdown di reader: filter row groups berdasarkan statistik footer
- Bloom filter per column chunk untuk point lookup
- Predicate-aware partial reads: skip row groups yang tidak memenuhi filter
- `qrd-query`: minimal SQL-like query engine di atas partial reads (single file)
- Schema evolution tooling: detect dan migrate compatible schema changes

### Phase 4 — Extended Ecosystem

**Focus:** Interoperabilitas dan adopsi ekosistem yang lebih luas

**Deliverables:**
- Konversi bidireksional Parquet ↔ QRD (dengan caveats enkripsi)
- Arrow IPC integration: QRD sebagai persistent layer, Arrow sebagai in-memory layer
- Streaming protocol: QRD over TCP/QUIC untuk real-time telemetry pipelines
- `MAP` tipe untuk key-value arbitrary
- Multi-file dataset abstraction dengan shared schema registry (opsional, tidak wajib)
- Formal ZK proof system integration (post-quantum cryptography exploration)

### Phase 5 — Formal Verification & Post-Quantum

**Focus:** Jaminan keamanan jangka panjang

**Deliverables:**
- Formal verification parser Rust menggunakan Prusti atau Kani (subset critical paths)
- Post-quantum key encapsulation (CRYSTALS-Kyber atau ML-KEM sebagai standar NIST)
- Hybrid classical+post-quantum key derivation (transitional)
- Hardware Security Module (HSM) key derivation integration guide

---

## 🤝 Contributing

QRD menargetkan kualitas **infrastructure-grade yang dapat diaudit**. Setiap kontribusi pada format binary, kriptografi, FFI, atau ECC memerlukan scrutiny lebih tinggi daripada perubahan dokumentasi atau tooling.

### Proses Kontribusi

1. **Buka issue** — deskripsikan perubahan, referensikan dokumen relevan di `docs/`, dan tunggu acknowledgment dari maintainer
2. **Submit PR** — deskripsi jelas, tests yang sesuai kategori (unit, property, golden, integration), dan benchmark jika relevan
3. **CI harus pass** — semua workflow (test, clippy, fmt, fuzz smoke) harus hijau
4. **Review keamanan** — perubahan pada `encryption/`, `ecc/`, `parser/`, atau format binary wajib review dari maintainer dengan security background

### Standar Kode

- Ikuti Rust idioms di `core/qrd-core/`; gunakan `clippy` dan `rustfmt` (konfigurasi di repo)
- Jaga FFI bindings tipis dan konsisten dengan core interface — jangan tambahkan logika bisnis
- Dokumentasikan semua public API dengan `///` doc comments dan contoh
- Setiap fitur baru: unit test + property test + golden vector jika format berubah
- Semua `unsafe` Rust: dokumentasikan `// SAFETY:` comment dengan invariant lengkap
- Perubahan benchmark: sertakan sebelum/sesudah dengan spesifikasi hardware

### Testing Requirements per Kategori PR

| Kategori PR | Test Minimum | Review Level |
|---|---|---|
| Dokumentasi saja | — | Self-merge setelah CI |
| Tooling / CLI | Unit + integration | 1 reviewer |
| SDK binding baru | Cross-lang integration + golden | 1 reviewer |
| Encoding baru | Unit + property + golden vector | 2 reviewer |
| Compression codec baru | Unit + property + benchmark | 2 reviewer |
| Perubahan format binary | Unit + property + golden + compat | Security review |
| Perubahan kriptografi | Unit + NIST vectors + fuzz + audit | Security review + external |

```bash
# Jalankan sebelum submit PR
cargo test --workspace
cargo clippy --workspace -- -D warnings
cargo fmt --all -- --check
PROPTEST_CASES=1000 cargo test --package qrd-core -- proptest
```

Lihat [`CONTRIBUTING.md`](CONTRIBUTING.md) untuk panduan lengkap termasuk release process, signing requirements, dan ekspektasi CI pipeline.

---

## 📚 Documentation Index

| Dokumen | Deskripsi |
|---|---|
| [`docs/FORMAT_SPEC.md`](docs/FORMAT_SPEC.md) | Binary format specification (canonical, normative) |
| [`docs/architecture/ARCHITECTURE.md`](docs/architecture/ARCHITECTURE.md) | Desain sistem & overview komponen |
| [`docs/security/SECURITY_AUDIT.md`](docs/security/SECURITY_AUDIT.md) | Audit scope, hasil, dan remediation |
| [`docs/security/THREAT_MODEL.md`](docs/security/THREAT_MODEL.md) | Threat analysis, actors, dan mitigasi |
| [`docs/security/CRYPTOGRAPHY.md`](docs/security/CRYPTOGRAPHY.md) | Pilihan primitif kriptografis & justifikasi |
| [`docs/security/FUZZING.md`](docs/security/FUZZING.md) | Fuzz target coverage & corpus management |
| [`docs/ENCRYPTION.md`](docs/ENCRYPTION.md) | Model enkripsi, key management, ZK semantics |
| [`docs/STREAMING_MODEL.md`](docs/STREAMING_MODEL.md) | Semantik streaming write/read |
| [`docs/MEMORY_MODEL.md`](docs/MEMORY_MODEL.md) | Bounded-memory guarantees & row group design |
| [`docs/COMPRESSION.md`](docs/COMPRESSION.md) | Filosofi kompresi, codec guide, tuning |
| [`docs/EDGE_AI.md`](docs/EDGE_AI.md) | Edge AI & telemetry workload guidance |
| [`docs/WASM.md`](docs/WASM.md) | WASM & browser runtime docs |
| [`docs/sdk/SDKS.md`](docs/sdk/SDKS.md) | Status SDK & instruksi instalasi per bahasa |
| [`docs/benchmarks/BENCHMARKS.md`](docs/benchmarks/BENCHMARKS.md) | Metodologi benchmark, hardware specs, hasil |
| [`docs/STABILITY.md`](docs/STABILITY.md) | Compatibility & deprecation policy |
| [`docs/VERSIONING.md`](docs/VERSIONING.md) | Semantic versioning policy |
| [`docs/PERFORMANCE.md`](docs/PERFORMANCE.md) | Performance philosophy & profiling guide |
| [`docs/COMPATIBILITY.md`](docs/COMPATIBILITY.md) | Cross-version compatibility rules |
| [`docs/COMPETITOR_COMPARISON.md`](docs/COMPETITOR_COMPARISON.md) | Format comparison & positioning |
| [`docs/DEPLOYMENT.md`](docs/DEPLOYMENT.md) | Deployment patterns & operational guidance |
| [`CHANGELOG.md`](CHANGELOG.md) | Version history & release notes |
| [`CONTRIBUTING.md`](CONTRIBUTING.md) | Panduan kontribusi lengkap |
| [`SECURITY.md`](SECURITY.md) | Responsible disclosure policy & PGP key |

---

## 📜 License

QRD-SDK dilisensikan di bawah [Business Source License 1.1 (BSL-1.1)](LICENSE).

**Ringkasan:**
- **Source-available**: kode dapat dibaca, dipelajari, dan dimodifikasi
- **Penggunaan produksi gratis** untuk use case non-kompetitif dengan QRD core
- **Setelah 4 tahun** (atau tanggal yang ditentukan), lisensi berkonversi otomatis ke **Apache 2.0**

> **Mengapa bukan MIT?** MIT kompatibel dengan strategi kompetitor yang mengambil core engine, menambahkan proprietary features, dan menjual sebagai produk tertutup tanpa kontribusi balik ke ekosistem. BSL melindungi keberlanjutan development QRD sambil tetap source-available untuk semua pengguna yang tidak berkompetisi langsung.
>
> Penggunaan internal, riset, pendidikan, dan integrasi dalam produk yang bukan "managed QRD container service" adalah gratis tanpa restriction.

Lihat [`LICENSE`](LICENSE) untuk teks lengkap dan definisi use restriction yang tepat.

---

<div align="center">

**QRD-SDK** — Privacy-native encrypted columnar container untuk sistem yang tidak dapat<br/>
berasumsi bahwa server, network, atau storage layer dapat dipercaya.

<br/>

*Data Anda. Kunci Anda. Format Anda.*

<br/>

[![GitHub](https://img.shields.io/badge/GitHub-zenipara%2FQRD--SDK-black?logo=github)](https://github.com/zenipara/QRD-SDK)
[![Documentation](https://img.shields.io/badge/Documentation-docs.qrd.dev-brightgreen)](https://docs.qrd.dev)
[![Security](https://img.shields.io/badge/Security-security%40qrd.dev-red)](mailto:security@qrd.dev)
[![Changelog](https://img.shields.io/badge/Changelog-CHANGELOG.md-blue)](CHANGELOG.md)

<br/>

*Built with Rust · BSL-1.1 → Apache 2.0 · Security-first design*

</div>
