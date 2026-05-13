# QRD Binary Format Specification

**Document Status:** Normative — Canonical Reference  
**Format Version:** 1.0  
**Revision:** 2025-01  
**Maintainer:** QRD Core Team · [security@qrd.dev](mailto:security@qrd.dev)

> Dokumen ini adalah **sumber kebenaran tunggal** untuk format binary QRD. Semua implementasi — dalam bahasa apapun — wajib mengikuti spesifikasi ini. Setiap deviasi dari dokumen ini adalah bug implementasi, bukan ambiguitas spesifikasi.

---

## Daftar Isi

1. [Konvensi Dokumen](#1-konvensi-dokumen)
2. [Gambaran Umum Format](#2-gambaran-umum-format)
3. [File Header](#3-file-header)
4. [Row Group](#4-row-group)
5. [Column Chunk Header](#5-column-chunk-header)
6. [File Footer](#6-file-footer)
7. [Encoding Algorithms](#7-encoding-algorithms)
8. [Compression Codecs](#8-compression-codecs)
9. [Type System](#9-type-system)
10. [Enkripsi Per-Kolom](#10-enkripsi-per-kolom)
11. [Error Correction (Reed-Solomon ECC)](#11-error-correction-reed-solomon-ecc)
12. [Integritas dan Verifikasi](#12-integritas-dan-verifikasi)
13. [Protokol Parsing (Wajib)](#13-protokol-parsing-wajib)
14. [Versioning dan Kompatibilitas Format](#14-versioning-dan-kompatibilitas-format)
15. [Compliance Checklist](#15-compliance-checklist)

---

## 1. Konvensi Dokumen

### Terminologi

| Kata Kunci | Makna |
|---|---|
| **MUST / WAJIB** | Requirement absolut; implementasi yang melanggar ini tidak conformant |
| **MUST NOT / DILARANG** | Larangan absolut |
| **SHOULD / DISARANKAN** | Best practice; boleh diabaikan dengan justifikasi yang tepat |
| **MAY / BOLEH** | Fitur opsional |

### Konvensi Byte Order

Seluruh integer multi-byte dalam format QRD menggunakan **little-endian (LE)** kecuali dinyatakan eksplisit. Platform big-endian WAJIB melakukan byte-swap saat baca dan tulis. Ini adalah kontrak canonical — output binary identik di semua arsitektur.

### Notasi Tipe

| Notasi | Deskripsi |
|---|---|
| `U8` | Unsigned 8-bit integer |
| `U16LE` | Unsigned 16-bit little-endian integer |
| `U32LE` | Unsigned 32-bit little-endian integer |
| `U64LE` | Unsigned 64-bit little-endian integer |
| `[U8; N]` | Array of N bytes |
| `BYTES` | Variable-length byte sequence |

---

## 2. Gambaran Umum Format

### Layout File Keseluruhan

```
┌──────────────────────────────────────────┐
│           FILE HEADER (32 bytes)         │  ← Wajib, fixed size
│   MAGIC · VERSION · SCHEMA_ID · FLAGS    │
├──────────────────────────────────────────┤
│              ROW GROUP 0                 │
│  ┌────────────────────────────────────┐  │
│  │  Row Group Header                  │  │  ← Per row group
│  ├────────────────────────────────────┤  │
│  │  Col Chunk 0  [enc│comp│crc32]     │  │  ← Per kolom
│  │  Col Chunk 1  [enc│comp│crc32]     │  │
│  │  ...                               │  │
│  │  Col Chunk N  [enc│comp│crc32]     │  │
│  ├────────────────────────────────────┤  │
│  │  [ECC Parity Chunks — optional]    │  │  ← Jika FLAGS.ECC = 1
│  ├────────────────────────────────────┤  │
│  │  Row Group Footer (mini)           │  │
│  └────────────────────────────────────┘  │
├──────────────────────────────────────────┤
│              ROW GROUP 1 ... N           │
├──────────────────────────────────────────┤
│              FILE FOOTER                 │  ← Variable size
│   Schema · Offsets · Stats · CRC32       │
├──────────────────────────────────────────┤
│       FOOTER_LENGTH (4 bytes U32LE)      │  ← 4 bytes terakhir file
└──────────────────────────────────────────┘
```

### Prinsip Desain Format

Format QRD dirancang di atas prinsip-prinsip berikut yang tidak boleh dilanggar oleh implementasi apapun:

**Streaming-first:** Penulis TIDAK BOLEH melakukan backtrack atau re-seek ke posisi sebelumnya. Footer ditulis terakhir setelah semua row group selesai.

**Append-only row groups:** Setiap row group yang sudah di-flush tidak dapat dimodifikasi. File QRD bersifat immutable setelah `finish()` dipanggil.

**Compression sebelum enkripsi:** Pipeline WAJIB mengkompresi data terlebih dahulu, baru mengenkripsinya. Mengenkripsi dahulu lalu mengompresi tidak menghasilkan rasio kompresi yang berarti dan merupakan violation format.

**Deterministik:** Input yang identik HARUS menghasilkan binary yang identik di semua implementasi dan platform, kecuali pada field yang memang dirancang acak (nonce, IV kriptografis).

---

## 3. File Header

File header adalah **32 bytes fixed-size** dan WAJIB berada di awal file (offset 0).

### Struktur

```
Offset  Size  Type     Field             Deskripsi
──────  ────  ───────  ────────────────  ──────────────────────────────────────
0       4     [U8;4]   MAGIC             0x51 0x52 0x44 0x01 (ASCII "QRD\x01")
4       2     U16LE    MAJOR_VERSION     Major version format (saat ini: 1)
6       2     U16LE    MINOR_VERSION     Minor version format (saat ini: 0)
8       8     [U8;8]   SCHEMA_ID         SHA-256 fingerprint schema (8 bytes pertama)
16      4     U32LE    FLAGS             Feature flags (lihat tabel di bawah)
20      4     U32LE    ROW_GROUP_COUNT   Jumlah total row groups dalam file
24      4     U32LE    CREATED_AT_SEC    Unix timestamp pembuatan file (UTC)
28      4     U32LE    HEADER_CRC32      CRC32 dari bytes 0–27
```

### FLAGS Bitmask

| Bit | Nama | Nilai | Deskripsi |
|---|---|---|---|
| 0 | `ENCRYPTED` | `0x00000001` | File mengandung kolom terenkripsi |
| 1 | `ECC_ENABLED` | `0x00000002` | Reed-Solomon ECC parity chunks aktif |
| 2 | `STATS_ENCRYPTED` | `0x00000004` | Statistik kolom terenkripsi (min/max/distinct tidak dapat dibaca tanpa kunci) |
| 3 | `SCHEMA_SIGNED` | `0x00000008` | Footer mengandung Ed25519 schema signature |
| 4–31 | Reserved | — | WAJIB diset 0 oleh writer; reader HARUS ignore bila bit 4–7, warn bila bit 0–3 tidak dikenal |

### Aturan Validasi Header

Reader WAJIB melakukan validasi berikut secara berurutan:

1. Baca 32 bytes pertama; tolak file jika kurang dari 32 bytes
2. Verifikasi MAGIC = `[0x51, 0x52, 0x44, 0x01]`; tolak jika mismatch
3. Verifikasi `HEADER_CRC32` = CRC32(bytes[0..28]); tolak jika mismatch
4. Verifikasi `MAJOR_VERSION` == versi yang didukung; tolak dengan `Error::UnsupportedMajorVersion` jika tidak cocok
5. Catat `SCHEMA_ID` untuk cross-validation dengan footer schema

---

## 4. Row Group

Row group adalah unit streaming utama dalam QRD. Setiap row group bersifat independen dan dapat dibaca tanpa membaca row group lain.

### Row Group Header

```
Offset  Size  Type    Field             Deskripsi
──────  ────  ──────  ────────────────  ──────────────────────────────────
0       4     U32LE   ROW_COUNT         Jumlah rows dalam row group ini
4       2     U16LE   COLUMN_COUNT      Jumlah column chunks dalam row group
6       2     U16LE   RG_FLAGS          Row group-level flags (reserved, set 0)
8       4     U32LE   RG_HEADER_CRC32   CRC32 dari bytes 0–11
```

### Urutan Column Chunk

Column chunks dalam satu row group WAJIB disimpan dalam urutan kolom yang sama persis dengan urutan field dalam schema footer. Implementasi TIDAK BOLEH menyimpan kolom dalam urutan berbeda.

---

## 5. Column Chunk Header

Setiap kolom dalam setiap row group memiliki header sebelum payload data.

### Struktur Column Chunk Header

```
Offset  Size  Type      Field             Deskripsi
──────  ────  ────────  ────────────────  ──────────────────────────────────────
0       1     U8        ENCODING_ID       ID algoritma encoding (lihat §7)
1       1     U8        COMPRESSION_ID    ID codec kompresi (lihat §8)
2       1     U8        ENCRYPTION_ID     0x00=none, 0x01=AES-256-GCM
3       1     U8        CHUNK_FLAGS       Reserved, set 0
4       4     U32LE     COMPRESSED_SIZE   Ukuran payload setelah kompresi (dan enkripsi)
8       4     U32LE     UNCOMPRESSED_SIZE Ukuran payload sebelum kompresi
12      4     U32LE     NULL_COUNT        Jumlah null values dalam chunk ini
16      4     U32LE     ROW_COUNT_CHUNK   Jumlah rows dalam chunk ini
20      8     U64LE     ROW_OFFSET        Offset row pertama chunk ini dalam row group

                        [Hanya jika ENCRYPTION_ID != 0x00]:
28      12    [U8;12]   NONCE             AES-GCM nonce (random kriptografis, per-chunk)
40      16    [U8;16]   AUTH_TAG          AES-GCM authentication tag
56      2     U16LE     KEY_ID_LEN        Panjang KEY_ID (0 jika tidak ada)
58      V     BYTES     KEY_ID            Identifier kunci (opsional, variable length)

                        [Payload]:
?       B     BYTES     PAYLOAD           Data: encoded → compressed → [encrypted]
?+B     4     U32LE     CRC32             CRC32 dari uncompressed payload (sebelum enkripsi)
```

### Aturan Penting Column Chunk

- **Nonce WAJIB baru per chunk:** Setiap column chunk yang dienkripsi WAJIB menggunakan nonce 12-byte yang di-generate ulang secara kriptografis acak. Reuse nonce dengan kunci yang sama adalah pelanggaran fatal keamanan.
- **CRC32 dari uncompressed data:** Field `CRC32` mengacu pada data setelah dekompresi, sebelum dekripsi. Ini memungkinkan deteksi korupsi bahkan pada kolom plaintext.
- **Reader WAJIB tolak chunk dengan AUTH_TAG gagal:** Kegagalan verifikasi AES-GCM authentication tag HARUS menghasilkan error `Error::AuthenticationFailed` tanpa mengekspos detail kegagalan (prevent oracle attack).

---

## 6. File Footer

File footer adalah struktur variable-length yang berisi schema, indeks row group, statistik, dan metadata opsional. Footer selalu ditulis terakhir.

### Protokol Menemukan Footer

```
1. Seek ke file_size - 4
2. Baca FOOTER_LENGTH sebagai U32LE
3. Validasi: FOOTER_LENGTH < (file_size - 32)    ← tolak jika tidak valid
4. Seek ke file_size - 4 - FOOTER_LENGTH
5. Baca FOOTER_LENGTH bytes sebagai footer content
6. Verifikasi CRC32 footer                        ← tolak jika mismatch
```

### Struktur Footer Content

```
[footer_version: U16LE]                    ← versi struktur footer (saat ini: 1)

─── Schema Section ───────────────────────────────────────────────────
[schema_length: U32LE]
[schema_version: U16LE]
[field_count: U16LE]
For each field:
  [name_len: U16LE]
  [name: UTF-8 bytes]                      ← nama kolom, WAJIB valid UTF-8
  [logical_type_id: U8]                    ← lihat §9 Type System
  [nullability_id: U8]                     ← 0=REQUIRED, 1=OPTIONAL, 2=REPEATED
  [encoding_hint: U8]                      ← preferred encoding untuk kolom ini
  [compression_hint: U8]                   ← preferred codec untuk kolom ini
  [encryption_id: U8]                      ← 0x00=none, 0x01=AES-256-GCM
  [metadata_count: U16LE]
  For each metadata entry:
    [key_len: U16LE] [key: UTF-8]
    [value_len: U16LE] [value: UTF-8]

─── Row Group Index ───────────────────────────────────────────────────
[row_group_count: U32LE]
For each row group:
  [byte_offset: U64LE]                     ← offset dari awal file (byte 0)
  [row_count: U32LE]                       ← jumlah rows dalam row group ini

─── Statistics Section ────────────────────────────────────────────────
[statistics_flag: U8]
  0x00 = tidak ada statistik
  0x01 = statistik plaintext (dapat dibaca tanpa kunci)
  0x02 = statistik terenkripsi (membutuhkan kunci untuk akses)
[statistics_length: U32LE]
[statistics_bytes]
  Per kolom: min_value, max_value, null_count, distinct_count

─── Encryption Metadata (hanya jika FLAGS.ENCRYPTED = 1) ─────────────
[key_derivation_algo: U8]                  ← 0x01 = HKDF-SHA256
[kdf_params_length: U16LE]
[kdf_params_bytes]                         ← salt (32 bytes), info prefix, output_len

─── Schema Signature (hanya jika FLAGS.SCHEMA_SIGNED = 1) ────────────
[sig_algo: U8]                             ← 0x01 = Ed25519
[signature: 64 bytes]
[public_key: 32 bytes]

─── File Metadata ─────────────────────────────────────────────────────
[file_metadata_length: U32LE]
[file_metadata_bytes]                      ← key-value pairs opsional (UTF-8)

─── Footer Checksum ───────────────────────────────────────────────────
[footer_checksum: U32LE]                   ← CRC32 dari seluruh footer content di atas
─────────────────────────────────────────────────────────────────────
[FOOTER_LENGTH: U32LE]                     ← 4 bytes terakhir file
```

---

## 7. Encoding Algorithms

Encoding diterapkan **sebelum kompresi dan sebelum enkripsi**. Tujuan encoding adalah mentransformasi nilai ke representasi yang lebih compressible. Setiap column chunk menyimpan `ENCODING_ID` di header.

### Registry Encoding ID

| ID | Nama | Keterangan Singkat |
|---|---|---|
| `0x00` | `PLAIN` | Raw serialized values, baseline |
| `0x01` | `RLE` | Run-length encoding, pasangan (count, value) |
| `0x02` | `BIT_PACKED` | Integer dan boolean dikemas sesuai bit-width minimum |
| `0x03` | `DELTA_BINARY` | Selisih antar integer berurutan, Parquet DELTA compatible |
| `0x04` | `DELTA_BYTE_ARRAY` | Prefix sharing untuk byte array berurutan |
| `0x05` | `BYTE_STREAM_SPLIT` | Reorder bytes floating-point per byte position |
| `0x06` | `DICTIONARY_RLE` | Dictionary index + RLE untuk low-cardinality strings |

### PLAIN (0x00)

Nilai disimpan dalam bentuk serialized mentah, little-endian untuk numerik.

```
[value_0][value_1]...[value_N]
```

Digunakan untuk: data dengan entropy tinggi (UUID, hash), atau data yang sudah dikompresi secara optimal oleh ZSTD.

### RLE — Run-Length Encoding (0x01)

Format: pasangan `[run_length: U32LE][value: T]` berulang.

```
Contoh: nilai [A, A, A, B, B] → [(3, A), (2, B)]
Wire format: [03 00 00 00][A...][02 00 00 00][B...]
```

Optimal untuk: kolom status/boolean yang terurut, ENUM dengan cardinality rendah.

### BIT_PACKED (0x02)

Integer dan boolean dikemas rapat sesuai bit-width minimum nilai dalam chunk.

```
Header: [bit_width: U8]
Body:   [packed_bits...]

8 boolean values  → 1 byte (bit_width=1)
4-bit integers    → 2 nilai per byte (bit_width=4)
```

### DELTA_BINARY (0x03)

Menyimpan selisih (delta) antar nilai integer berurutan. Compatible dengan Parquet DELTA_BINARY_PACKED.

```
Input:  [100, 102, 105, 109]
Output: first_value=100, deltas=[+2, +3, +4]
Format: [first_value: T][delta_min: T][bit_width: U8][deltas: BIT_PACKED]
```

Rasio kompresi tipikal: **4–8× vs PLAIN** untuk timestamp monoton atau auto-increment ID.

### DELTA_BYTE_ARRAY (0x04)

Prefix sharing untuk byte array: menyimpan `(shared_prefix_len, suffix)` per nilai.

```
["https://api.example.com/v1/users", "https://api.example.com/v1/orders"]
→ prefix_len=30, suffixes=["users", "orders"]
```

### BYTE_STREAM_SPLIT (0x05)

Reorder bytes floating-point ke stream terpisah per byte position, meningkatkan compressibility float secara dramatis.

```
FLOAT32 stream: [b0b1b2b3, b0b1b2b3, ...]
Setelah split:  stream-0:[b0,b0,...], stream-1:[b1,b1,...], ...
```

Kombinasi dengan ZSTD menghasilkan rasio tipikal **3–6× vs PLAIN** untuk sensor float.

### DICTIONARY_RLE (0x06)

Dictionary index `U16LE` + RLE untuk low-cardinality string. Dictionary table disimpan di footer schema field metadata.

```
Dictionary: {"active": 0, "inactive": 1, "pending": 2}
Values:     [active, active, inactive, active]
Wire:       RLE([(2, 0), (1, 1), (1, 0)])
Batas:      65,535 nilai unik per kolom
```

### Aturan Encoding

- Reader WAJIB fail-fast dengan `Error::UnknownEncoding { id }` bila menemukan ENCODING_ID yang tidak dikenal
- Reader WAJIB toleran terhadap empty column chunk (0 rows) dengan encoding apapun
- Writer BOLEH memilih encoding berbeda per row group untuk kolom yang sama

---

## 8. Compression Codecs

Kompresi diterapkan setelah encoding dan sebelum enkripsi.

### Registry Compression ID

| ID | Nama | Keterangan |
|---|---|---|
| `0x00` | `NONE` | Tidak ada kompresi |
| `0x01` | `ZSTD` | Zstandard, level 1–22 (default: level 3) |
| `0x02` | `LZ4_FRAME` | LZ4 framed format, ultra-low latency |
| `0x03` | `SNAPPY` | Reserved untuk kompatibilitas, belum diimplementasikan |

### Adaptive Selection

Writer DISARANKAN menggunakan heuristik berikut untuk pemilihan codec:

```
Latency-sensitive (streaming edge):   LZ4_FRAME
Archive/batch (space-sensitive):      ZSTD level 3–9
Sudah random/terenkripsi:             NONE
```

### Aturan Kompresi

- Reader WAJIB fail-fast dengan `Error::UnknownCompression { id }` bila menemukan COMPRESSION_ID tidak dikenal
- `COMPRESSED_SIZE` dan `UNCOMPRESSED_SIZE` WAJIB konsisten dengan payload aktual
- Jika kompresi menghasilkan output lebih besar dari input, writer BOLEH menyimpan dengan `COMPRESSION_ID=NONE` dan `COMPRESSED_SIZE == UNCOMPRESSED_SIZE`

---

## 9. Type System

### Numeric Types

| Type ID | Nama | Bytes | Range / Representasi |
|---|---|---|---|
| `0x01` | `BOOLEAN` | 1/8 (bit-packed) | true/false, 8 values per byte |
| `0x02` | `INT8` | 1 | -128 … 127 |
| `0x03` | `INT16` | 2 | -32,768 … 32,767 (LE) |
| `0x04` | `INT32` | 4 | -2³¹ … 2³¹-1 (LE) |
| `0x05` | `INT64` | 8 | -2⁶³ … 2⁶³-1 (LE) |
| `0x06` | `UINT8` | 1 | 0 … 255 |
| `0x07` | `UINT16` | 2 | 0 … 65,535 (LE) |
| `0x08` | `UINT32` | 4 | 0 … 2³²-1 (LE) |
| `0x09` | `UINT64` | 8 | 0 … 2⁶⁴-1 (LE) |
| `0x0A` | `FLOAT32` | 4 | IEEE 754 single precision (LE) |
| `0x0B` | `FLOAT64` | 8 | IEEE 754 double precision (LE) |

### Temporal Types

| Type ID | Nama | Bytes | Semantik |
|---|---|---|---|
| `0x10` | `TIMESTAMP` | 8 | Microseconds sejak Unix epoch, UTC (INT64 LE) |
| `0x11` | `DATE` | 4 | Hari sejak 1970-01-01 (INT32 LE) |
| `0x12` | `TIME` | 8 | Microseconds sejak 00:00:00 UTC (INT64 LE) |
| `0x13` | `DURATION` | 8 | Microseconds signed (INT64 LE) |

### Text dan Binary Types

| Type ID | Nama | Format | Batas |
|---|---|---|---|
| `0x20` | `UTF8_STRING` | `[U32LE length][UTF-8 bytes]` | 4 GB per value |
| `0x21` | `ENUM` | `[U16LE dict_index]` + dict di footer | 65,535 nilai unik |
| `0x22` | `UUID` | `[16 bytes]` raw, RFC 4122 big-endian | — |
| `0x23` | `BLOB` | `[U32LE length][bytes]` | 4 GB per value |
| `0x24` | `DECIMAL` | `[U8 sign][U8 scale][variable magnitude]` | Arbitrary precision |

### Composite Types (Planned — Phase 3 dan 4)

| Type ID | Nama | Status |
|---|---|---|
| `0x30` | `STRUCT` | Phase 3 |
| `0x31` | `ARRAY` | Phase 3 |
| `0x32` | `MAP` | Phase 4 |
| `0xFF` | `ANY` | Phase 4 |

### Nullability

| ID | Nama | Null Bitmap | Overhead |
|---|---|---|---|
| `0x00` | `REQUIRED` | Tidak ada | 0 bytes |
| `0x01` | `OPTIONAL` | Present, bit-packed | ⌈N/8⌉ bytes per N rows |
| `0x02` | `REPEATED` | Present + offset array | Variable |

---

## 10. Enkripsi Per-Kolom

### Model Enkripsi

QRD menggunakan **AES-256-GCM** untuk enkripsi payload kolom. Enkripsi bersifat **opsional per kolom** — satu file dapat mengandung kolom plaintext dan terenkripsi secara bersamaan.

### Key Derivation (HKDF-SHA256)

Kunci per-kolom diturunkan dari master key menggunakan HKDF dengan domain separation yang ketat:

```
column_key = HKDF-SHA256(
    ikm   = master_key,           // 32 bytes, dipegang oleh client
    salt  = file_salt,            // 32 bytes random, disimpan di footer encryption_metadata
    info  = "qrd:col:{col_name}:{schema_id_hex}"
)

Contoh info string:
  "qrd:col:health_val:a1b2c3d4e5f6a7b8"
```

Domain separation via `info` string memastikan kunci yang berbeda dihasilkan untuk kolom berbeda, meskipun berasal dari master key yang sama.

### Enkripsi Payload

```
Untuk setiap column chunk yang dienkripsi:

1. nonce = OsRng::fill_bytes([0u8; 12])    // 12 bytes kriptografis acak
2. (ciphertext, auth_tag) = AES_256_GCM(
       key   = column_key,
       nonce = nonce,
       plaintext = compressed_payload,
       aad   = []                           // no additional authenticated data
   )
3. Simpan: [NONCE (12)][AUTH_TAG (16)][CIPHERTEXT]
```

### Nonce Uniqueness Guarantee

Setiap column chunk WAJIB menggunakan nonce yang di-generate ulang secara acak. Probabilitas collision dengan nonce 12-byte acak adalah **1/(2^96)** — dapat diabaikan untuk jumlah chunk yang wajar.

**Konsekuensi desain:** File QRD terenkripsi yang ditulis dua kali dari input yang identik akan menghasilkan binary yang **berbeda** karena nonce berbeda. Cloud deduplication berbasis content-hash tidak efektif pada file QRD terenkripsi — ini adalah trade-off yang disengaja untuk keamanan semantik (IND-CPA).

### Statistik Terenkripsi

Jika `FLAGS.STATS_ENCRYPTED = 1`, field statistik (min/max/null_count/distinct_count) untuk kolom terenkripsi JUGA dienkripsi menggunakan kunci kolom yang sama. Footer tidak membocorkan distribusi data kolom sensitif.

---

## 11. Error Correction (Reed-Solomon ECC)

Reed-Solomon ECC adalah fitur opsional yang diaktifkan via `FLAGS.ECC_ENABLED`. Ketika aktif, setiap row group menyertakan parity chunks yang memungkinkan recovery dari korupsi partial.

### Parameter ECC

```
DATA_CHUNKS   : N  = jumlah column chunks dalam row group
PARITY_CHUNKS : K  = jumlah parity chunks tambahan

Recovery: hingga K chunks yang hilang atau korup dapat di-reconstruct

Konfigurasi tipikal:
  RS(32, 8)  → toleran 8 chunk korup dari 32 total (25%)
  RS(16, 4)  → toleran 4 chunk korup dari 16 total (25%)
```

### Parity Chunk Placement

Parity chunks ditempatkan setelah semua data column chunks dalam row group, sebelum row group footer. Reader HARUS mengidentifikasi parity chunks via row group header `RG_FLAGS.ECC_PRESENT` bit.

### Penggunaan ECC

ECC DISARANKAN diaktifkan untuk:

- Cold storage jangka panjang (bit rot prevention)
- Transmisi via kanal yang tidak reliable
- Media yang terdegradasi (HDD dengan bad sectors)
- Archival storage dengan SLA durability tinggi

---

## 12. Integritas dan Verifikasi

QRD memvalidasi integritas di tiga level yang independen:

### Level 1: Per Column Chunk (CRC32)

```
CRC32(uncompressed_payload) disimpan di column chunk header
Reader WAJIB verifikasi setelah dekompresi, sebelum dekoding
Deteksi: storage corruption, transmission errors, partial writes
Error: Error::ChunkChecksumMismatch
```

### Level 2: AES-GCM Authentication Tag

```
AUTH_TAG memverifikasi integritas DAN keaslian ciphertext
Gagal jika ciphertext dimodifikasi (adversarial atau accidental)
Lebih kuat dari CRC32 — unforgeable tanpa kunci
Error: Error::AuthenticationFailed
```

### Level 3: Per File Footer (CRC32)

```
CRC32(footer_content) disimpan sebagai field terakhir footer
Diverifikasi sebelum metadata apapun diparse
Reader WAJIB menolak file dengan footer CRC mismatch
Error: Error::FooterChecksumMismatch
```

### Hierarki Kegagalan

Reader WAJIB menerapkan hierarki ini: jika footer CRC32 gagal, tolak seluruh file. Jika chunk CRC32 gagal tetapi ECC aktif, coba recovery. Jika AUTH_TAG gagal, tolak chunk tanpa mengekspos detail kegagalan.

---

## 13. Protokol Parsing (Wajib)

Seluruh implementasi QRD WAJIB mengikuti urutan operasi ini. Deviasi dari urutan ini adalah non-conformant.

### Urutan Parse File

```
1. Baca dan validasi FILE HEADER (32 bytes)
   a. Verifikasi MAGIC bytes
   b. Verifikasi HEADER_CRC32
   c. Verifikasi MAJOR_VERSION compatibility
   d. Catat SCHEMA_ID untuk cross-validation

2. Baca FILE FOOTER
   a. Seek ke file_size - 4; baca FOOTER_LENGTH
   b. Validasi FOOTER_LENGTH range
   c. Seek ke file_size - 4 - FOOTER_LENGTH; baca footer bytes
   d. Verifikasi footer CRC32
   e. Parse schema, row group offsets, statistik

3. Cross-validate SCHEMA_ID dari header dengan SHA-256 fingerprint schema dari footer
   Tolak jika tidak cocok: Error::SchemaIdMismatch

4. Baca ROW GROUPs menggunakan offsets dari footer
   a. Parse Row Group Header
   b. Per Column Chunk:
      i.   Baca Column Chunk Header
      ii.  Baca PAYLOAD (COMPRESSED_SIZE bytes)
      iii. Dekripsi bila ENCRYPTION_ID != 0x00 (verifikasi AUTH_TAG)
      iv.  Dekompresi payload
      v.   Verifikasi CRC32(decompressed) == chunk header CRC32
      vi.  Decode payload dengan ENCODING_ID
```

### Parser Hardening Requirements

Implementasi WAJIB memenuhi seluruh requirements berikut:

- **Zero-panic policy:** Parser TIDAK BOLEH panic pada input adversarial apapun
- **Strict bounds checking:** Semua size fields WAJIB divalidasi sebelum alokasi memori
- **Integer overflow:** WAJIB menggunakan checked arithmetic, bukan wrapping
- **Fail-fast pada ID tidak dikenal:** Tolak ENCODING_ID atau COMPRESSION_ID yang tidak dikenal dengan error eksplisit
- **UTF-8 validation:** Semua field nama dan string WAJIB divalidasi sebagai UTF-8 valid

### Contoh Parser Hardening (Rust)

```rust
fn parse_footer_length(file: &mut impl Read + Seek) -> Result<u32> {
    let file_size = file.seek(SeekFrom::End(0))?;
    // Pastikan file cukup besar untuk header + footer_length field
    ensure!(
        file_size >= HEADER_SIZE + 4,
        Error::FileTooSmall { file_size }
    );
    
    file.seek(SeekFrom::End(-4))?;
    let footer_len = file.read_u32::<LittleEndian>()?;
    
    // Validasi range sebelum alokasi
    ensure!(
        footer_len > 0 && footer_len <= file_size.saturating_sub(HEADER_SIZE + 4),
        Error::InvalidFooterLength { footer_len, file_size }
    );
    
    Ok(footer_len)
}
```

---

## 14. Versioning dan Kompatibilitas Format

### Semantic Versioning Format

```
MAJOR.MINOR (dalam FILE HEADER)

MAJOR → Perubahan binary tidak backward-compatible
MINOR → Penambahan fitur opsional yang backward-compatible
```

### Compatibility Matrix

| Skenario | Behavior yang Diwajibkan |
|---|---|
| Reader MAJOR == Writer MAJOR | Fully compatible |
| Reader MAJOR < Writer MAJOR | Tolak: `Error::UnsupportedMajorVersion` |
| Reader MINOR < Writer MINOR | Ignore unknown optional fields; partial support |
| Unknown ENCODING_ID | `Error::UnknownEncoding { id }` |
| Unknown COMPRESSION_ID | `Error::UnknownCompression { id }` |
| Unknown FLAGS bit (≥ bit 4) | Ignore |
| Unknown FLAGS bit (bit 0–3) | Warn |
| Korup CRC32 (chunk) | `Error::ChunkChecksumMismatch` |
| Korup CRC32 (footer) | `Error::FooterChecksumMismatch` |
| AES-GCM auth tag fail | `Error::AuthenticationFailed` |

### Schema Compatibility

| Perubahan Schema | Backward Compatible? | Efek pada SCHEMA_ID |
|---|---|---|
| Tambah kolom OPTIONAL di akhir | Ya | Berubah |
| Tambah optional metadata field | Ya | Tidak berubah |
| Rename field | Tidak — Breaking | Berubah |
| Ubah tipe field | Tidak — Breaking | Berubah |
| Ubah REQUIRED → OPTIONAL | Tidak — Breaking | Berubah |
| Ubah urutan kolom | Tidak — Breaking | Berubah |

---

## 15. Compliance Checklist

Implementasi QRD dianggap conformant jika memenuhi seluruh checklist ini:

### Writer Compliance

- [ ] MAGIC bytes di header = `[0x51, 0x52, 0x44, 0x01]`
- [ ] Semua integer multi-byte dalam little-endian
- [ ] HEADER_CRC32 dihitung dengan benar
- [ ] `finish()` menulis footer sebelum close
- [ ] Kompresi terjadi **sebelum** enkripsi
- [ ] Nonce 12-byte di-generate ulang per column chunk terenkripsi
- [ ] Column chunks disimpan dalam urutan field schema
- [ ] CRC32 per column chunk dihitung dari uncompressed payload

### Reader Compliance

- [ ] Verifikasi MAGIC dan HEADER_CRC32 sebelum operasi apapun
- [ ] Verifikasi footer CRC32 sebelum parse schema
- [ ] Cross-validate SCHEMA_ID header vs footer
- [ ] Fail-fast pada ENCODING_ID atau COMPRESSION_ID tidak dikenal
- [ ] Tolak AES-GCM AUTH_TAG yang gagal tanpa mengekspos detail
- [ ] Tidak panic pada input adversarial apapun
- [ ] Semua size fields divalidasi sebelum alokasi memori

### Golden Vector Validation

Semua implementasi WAJIB lulus golden vector test suite yang tersedia di:

```
tests/golden/vectors/v1.0/
├── minimal_schema.qrd
├── all_types_plaintext.qrd
├── encrypted_columns.qrd
├── ecc_enabled.qrd
└── cross-lang/     ← File ditulis oleh satu SDK, dibaca oleh semua
```

---

*QRD Binary Format Specification v1.0 — © Zenipara / QRD Core Team*  
*Dokumen ini bersifat normative. Pertanyaan atau klarifikasi: buka issue di GitHub atau email ke [security@qrd.dev](mailto:security@qrd.dev)*
