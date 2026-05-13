# QRD-SDK Architecture

**Document Type:** System Design Reference  
**Audience:** Engineers yang mengintegrasikan, mengextend, atau mengaudit QRD-SDK  
**Version:** 1.0

---

## Daftar Isi

1. [Filosofi Arsitektur](#1-filosofi-arsitektur)
2. [Layered Architecture Overview](#2-layered-architecture-overview)
3. [Rust Core Engine](#3-rust-core-engine)
4. [FFI dan WASM Interface Layer](#4-ffi-dan-wasm-interface-layer)
5. [Language SDK Layer](#5-language-sdk-layer)
6. [Streaming Write Pipeline](#6-streaming-write-pipeline)
7. [Read Modes dan Read Pipeline](#7-read-modes-dan-read-pipeline)
8. [Memory Model](#8-memory-model)
9. [Cryptographic Architecture](#9-cryptographic-architecture)
10. [Repository Structure](#10-repository-structure)
11. [Dependency Graph](#11-dependency-graph)
12. [CI/CD Pipeline](#12-cicd-pipeline)

---

## 1. Filosofi Arsitektur

### Single Engine, Multiple Languages

Keputusan arsitektur paling mendasar dalam QRD adalah **satu Rust core engine sebagai sumber kebenaran tunggal untuk semua bahasa**. Ini bukan keputusan teknologi — ini adalah keputusan tentang jaminan correctness.

Setiap kali ada dua implementasi independen dari format binary yang sama (misalnya implementasi Python dan implementasi Go yang terpisah), ada ruang untuk *drift* — perbedaan kecil dalam interpretasi edge cases, byte order, atau error handling yang hanya terdeteksi saat sistem berinteraksi di produksi.

QRD mengeliminasi drift ini dengan cara: **tidak ada implementasi kedua**. Python, TypeScript, Go, Java, dan C/C++ semuanya adalah lapisan tipis di atas satu Rust core yang sama.

```
Bahasa SDK ──► FFI/WASM ──► Rust Core Engine
                               (satu-satunya
                                implementasi
                                format binary)
```

### Privacy Sebagai Properti, Bukan Fitur

Arsitektur QRD dirancang sehingga enkripsi **tidak bisa dilewati secara tidak sengaja**. Tidak ada "mode plaintext" yang bisa diaktifkan via flag runtime. Jika kolom ditandai terenkripsi dalam schema, enkripsi akan selalu terjadi di pipeline. Ini adalah keputusan desain yang sadar — developer tidak seharusnya dapat mengkonfigurasi keamanan ke kondisi lemah secara tidak disengaja.

### Zero-Trust di Level Format

Format QRD dirancang dengan asumsi bahwa **tidak ada infrastruktur antara penulis dan pembaca yang dapat dipercaya** — termasuk storage server, transport layer (selain TLS), dan intermediate processors. Kepercayaan diberikan hanya kepada kunci kriptografis yang dipegang oleh client.

---

## 2. Layered Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────┐
│                          Application Layer                          │
│      Analytics pipeline · ML inference · Telemetry · Audit log     │
└────────────────────────────────┬────────────────────────────────────┘
                                 │ SDK calls
┌────────────────────────────────▼────────────────────────────────────┐
│                        Language SDK Layer                           │
│                                                                     │
│  ┌──────────┐  ┌────────────┐  ┌──────┐  ┌──────┐  ┌───────────┐  │
│  │  Python  │  │ TypeScript │  │  Go  │  │ Java │  │   C/C++   │  │
│  │  (PyO3)  │  │   (WASM)   │  │(CGO) │  │(JNI) │  │  (C FFI)  │  │
│  └────┬─────┘  └─────┬──────┘  └──┬───┘  └──┬───┘  └─────┬─────┘  │
└───────│───────────────│────────────│──────────│────────────│────────┘
        │               │            │          │            │
┌───────▼───────────────▼────────────▼──────────▼────────────▼────────┐
│                     FFI / WASM Interface Layer                       │
│                                                                      │
│   core/qrd-ffi/   → C-compatible ABI, stable opaque pointer API    │
│   core/qrd-wasm/  → WebAssembly target, WASI + browser runtime     │
└──────────────────────────────┬───────────────────────────────────────┘
                               │ direct Rust calls
┌──────────────────────────────▼───────────────────────────────────────┐
│                        Rust Core Engine                              │
│                          core/qrd-core/                             │
│                                                                      │
│  ┌─────────────┐ ┌──────────────┐ ┌──────────────┐ ┌────────────┐  │
│  │   Schema    │ │    Writer    │ │    Reader    │ │  Encoding  │  │
│  │   Builder   │ │  Streaming   │ │   Partial    │ │ PLAIN/RLE/ │  │
│  │             │ │  Row Group   │ │  Column      │ │ DELTA/BSS  │  │
│  └─────────────┘ └──────────────┘ └──────────────┘ └────────────┘  │
│                                                                      │
│  ┌─────────────┐ ┌──────────────┐ ┌────────────────────────────┐   │
│  │ Compression │ │  Encryption  │ │      ECC / Integrity       │   │
│  │  ZSTD / LZ4 │ │ AES-256-GCM  │ │  Reed-Solomon / CRC32      │   │
│  │  Adaptive   │ │  + HKDF      │ │  + BLAKE3 aux digest       │   │
│  └─────────────┘ └──────────────┘ └────────────────────────────┘   │
│                                                                      │
│  ┌─────────────┐ ┌──────────────┐ ┌────────────────────────────┐   │
│  │  Columnar   │ │   Metadata   │ │       Fuzz Targets         │   │
│  │  Transpose  │ │  Footer I/O  │ │  header/footer/rowgroup    │   │
│  └─────────────┘ └──────────────┘ └────────────────────────────┘   │
└──────────────────────────────────────────────────────────────────────┘
```

---

## 3. Rust Core Engine

### Komponen Inti

#### `schema/`
Bertanggung jawab atas definisi, serialisasi, dan fingerprinting schema.

- **SchemaBuilder** — fluent API untuk mendefinisikan fields, tipe, dan nullability
- **Schema fingerprint** — SHA-256 truncated (8 bytes) untuk cross-validation antara header dan footer
- **Schema serialization** — binary encoding schema untuk footer
- **Dictionary management** — dictionary table untuk ENUM dan DICTIONARY_RLE columns

#### `writer/`
Implementasi streaming writer dengan bounded memory.

- **StreamingWriter** — entry point utama; menerima rows satu per satu atau dalam batch
- **RowGroupBuffer** — buffer in-memory untuk akumulasi rows hingga `row_group_size` tercapai
- **RowGroupFlusher** — mengeksekusi pipeline transpose → encode → compress → encrypt → flush
- **FooterBuilder** — mengakumulasikan metadata row group dan menulis footer di akhir

#### `reader/`
Implementasi reader dengan dukungan partial column read.

- **FileReader** — entry point utama; membaca footer terlebih dahulu untuk mendapatkan schema dan offsets
- **ColumnReader** — membaca dan mendekripsi column chunk spesifik tanpa membaca column lain
- **FooterParser** — parsing footer binary dengan validasi CRC32 dan schema cross-validation
- **IntegrityVerifier** — verifikasi menyeluruh CRC32 per-chunk, AUTH_TAG, dan ECC

#### `encoding/`
Implementasi semua 7 algoritma encoding.

- Setiap encoder/decoder adalah modul terpisah yang dapat ditest secara independen
- Golden vector tests membuktikan binary identik lintas versi
- Property tests memverifikasi roundtrip correctness untuk semua tipe data

#### `compression/`
Wrapper di atas ZSTD dan LZ4 dengan adaptive selection.

- **Adaptive selection** — mengukur entropy data untuk memilih codec optimal
- **Entropy estimator** — sampling cepat untuk menghindari kompresi data acak
- Compression dan decompression selalu menggunakan size bounds yang ketat

#### `encryption/`
AES-256-GCM dan HKDF key derivation.

- **KeyDeriver** — HKDF-SHA256 dengan domain separation per kolom
- **ChunkEncryptor** — enkripsi per column chunk dengan nonce baru per operasi
- **NonceGenerator** — `OsRng` sebagai sumber entropy; tidak ada nonce reuse dalam lifetime objek
- Semua operasi kriptografis menggunakan RustCrypto crates yang telah diaudit

#### `ecc/`
Reed-Solomon error correction coding.

- Implementasi berbasis Galois Field (GF(2^8))
- Encoding menghasilkan K parity chunks dari N data chunks
- Decoding dapat mereconstruct hingga K chunks yang hilang atau korup
- Parameterisasi fleksibel: RS(N, K) dapat dikonfigurasi per file

#### `columnar/`
Row-to-column transposition.

- Mengubah representasi row-oriented (dari aplikasi) ke column-oriented (untuk storage)
- Null bitmap generation untuk kolom OPTIONAL
- Transposisi adalah operasi murni tanpa side effects

#### `integrity/`
CRC32 computation dan verifikasi.

- Hardware-accelerated via `crc32fast` (menggunakan instruksi CPU SSE4.2 bila tersedia)
- Digunakan untuk: column chunk payload, file footer, dan file header

---

## 4. FFI dan WASM Interface Layer

### C FFI (`core/qrd-ffi/`)

C FFI layer menyediakan **stable C-compatible ABI** yang digunakan oleh Go (CGO), Java (JNI), Python (PyO3 memanggil C FFI internally), dan C/C++ langsung.

**Prinsip desain FFI:**
- **Opaque pointer pattern** — semua state disimpan di belakang pointer opaque; client tidak mengakses struct internal
- **No business logic** — FFI layer adalah lapisan tipis; tidak ada logika encoding, kompresi, atau enkripsi di sini
- **Owned memory** — caller bertanggung jawab untuk memanggil `qrd_free_*` functions untuk setiap objek yang dialokasikan
- **Error reporting** — semua fungsi mengembalikan error code; detail error tersedia via `qrd_last_error()`

```c
// Header file: core/qrd-ffi/include/qrd.h (excerpt)
typedef struct QrdWriter QrdWriter;
typedef struct QrdReader QrdReader;

QrdWriter* qrd_writer_new(const char* path, const QrdSchema* schema, const QrdConfig* config);
int        qrd_writer_write_row(QrdWriter* w, const QrdRow* row);
int        qrd_writer_finish(QrdWriter* w);
void       qrd_writer_free(QrdWriter* w);

QrdReader* qrd_reader_open(const char* path, const QrdReadConfig* config);
int        qrd_reader_row_count(const QrdReader* r, uint64_t* out);
int        qrd_reader_read_columns(const QrdReader* r, const char** cols, size_t n, QrdColumnBatch* out);
void       qrd_reader_free(QrdReader* r);
```

### WASM Target (`core/qrd-wasm/`)

WASM target dikompilasi untuk dua runtime: **browser** dan **Node.js (WASI)**.

- Menggunakan `wasm-bindgen` untuk JavaScript interop yang ergonomis
- Tidak ada system calls yang tidak tersedia di browser (tidak ada filesystem langsung, tidak ada threads)
- Memory management menggunakan `wasm_bindgen::JsValue` dan Rust ownership
- Buffer-based API: semua I/O melalui `Uint8Array` di JavaScript

```typescript
// API publik WASM (TypeScript types)
export function initWasm(): Promise<void>;
export function inspectFooter(buffer: Uint8Array): QrdFooterInfo;

export class FileReader {
  constructor(buffer: Uint8Array);
  readColumn(name: string, opts?: ReadOptions): TypedArray;
  get rowCount(): number;
  get schema(): QrdSchema;
}
```

---

## 5. Language SDK Layer

Setiap SDK adalah **lapisan tipis** di atas FFI atau WASM. Tanggung jawab SDK adalah:

1. Menyediakan API yang idiomatik untuk bahasa tersebut (misalnya context manager di Python, async/await di TypeScript)
2. Melakukan type marshaling antara tipe bahasa dan tipe C/WASM
3. Menyediakan dokumentasi dan contoh yang sesuai ekosistem bahasa

**SDK tidak boleh mengimplementasikan logika format.** Jika ada logika yang tampaknya perlu ditambahkan di SDK, itu harus ditambahkan di core engine dan diekspos via FFI.

### Python SDK (`sdk/python/`)

- Menggunakan PyO3 untuk binding Rust-Python yang efisien
- Menyediakan context manager (`with qrd.FileWriter(...) as w:`)
- Type hints penuh untuk IDE support
- Numpy array integration untuk kolom numerik

### TypeScript SDK (`sdk/typescript/`)

- Bundle WASM binary + JavaScript glue dalam satu npm package
- Async API untuk inisialisasi WASM (`await initWasm()`)
- TypeScript definitions untuk seluruh public API
- Support browser (via bundler) dan Node.js

### Go SDK (`sdk/go/`)

- CGO binding ke C FFI
- Idiomatic Go error handling (`val, err := reader.RowCount()`)
- Resource cleanup via `defer reader.Close()`

### Java SDK (`sdk/java/`)

- JNI binding ke C FFI
- Native library loading otomatis dari JAR resources
- Checked exceptions untuk error handling

---

## 6. Streaming Write Pipeline

Ini adalah pipeline yang dieksekusi setiap kali row group penuh dan perlu di-flush ke file. **Urutan ini adalah kontrak format, bukan detail implementasi.**

```
Input Rows (dari aplikasi)
        │
        ▼
┌───────────────────┐
│   Row Buffer      │  Akumulasi rows dalam memori hingga batas
│   (per Row Group) │  row_group_size tercapai
└────────┬──────────┘
         │  [buffer penuh → trigger flush]
         ▼
┌───────────────────┐
│ Columnar Transpose│  Mengubah representasi:
│                   │  row[0..N].col[k] → col[k].values[0..N]
└────────┬──────────┘
         │  [per column]
         ▼
┌───────────────────┐
│ Per-Column        │  PLAIN / RLE / DELTA / BIT_PACKED /
│ Encoding          │  DELTA_BYTE_ARRAY / BYTE_STREAM_SPLIT / DICT_RLE
└────────┬──────────┘
         │
         ▼
┌───────────────────┐
│ Per-Chunk         │  ZSTD (level 3 default) / LZ4_FRAME / NONE
│ Compression       │  Selalu SEBELUM enkripsi
└────────┬──────────┘
         │
         ▼
┌───────────────────┐
│ AES-256-GCM       │  Opsional per-kolom
│ Encryption        │  Nonce baru per chunk, kunci dari HKDF
└────────┬──────────┘
         │
         ▼
┌───────────────────┐
│ CRC32 Append      │  CRC32(uncompressed_payload) di akhir chunk
│ + Auth Tag        │  AES-GCM AUTH_TAG sudah terintegrasi dari step sebelumnya
└────────┬──────────┘
         │
         ▼
┌───────────────────┐
│ Reed-Solomon ECC  │  Opsional; menghasilkan K parity chunks
│ (per Row Group)   │  dari N data chunks
└────────┬──────────┘
         │
         ▼
┌───────────────────┐
│ Row Group Flush   │  Write ke file stream (append-only, no backtrack)
│ → File Stream     │  Header + chunks + ECC chunks + mini footer
└────────┬──────────┘
         │  [setelah semua row groups]
         ▼
┌───────────────────┐
│ File Footer Write │  Schema + row group offsets + statistik
│                   │  + encryption metadata + CRC32 footer
└───────────────────┘
```

---

## 7. Read Modes dan Read Pipeline

### Empat Mode Baca

#### Mode 1: Footer-Only Inspection

Hanya membaca footer — schema, statistik, jumlah row group, field names. Tidak membaca payload data apapun. Cocok untuk discovery, cataloging, dan browser metadata display.

```rust
let meta = FileReader::inspect_footer("file.qrd")?;
println!("{} rows, {} columns", meta.row_count, meta.schema.fields.len());
```

#### Mode 2: Partial Column Read (Mode Utama Analytics)

Membaca hanya subset kolom yang diminta. Seek langsung ke column chunk yang relevan, skip semua column chunk yang tidak diminta.

```
Footer parse → dapatkan offsets
    ↓
Seek ke Col[k] Chunk dalam Row Group[j]
    ↓ (untuk setiap row group)
Baca → Dekripsi (jika perlu) → Dekompresi → Decode
    ↓
Skip Col[k+1] ... Col[N] yang tidak diminta
```

#### Mode 3: Row Group Projection

Pilih hanya subset row groups berdasarkan range atau predicate statistik. Memungkinkan skip row groups yang tidak memenuhi filter tanpa membaca data mereka.

```rust
// Baca hanya row groups dengan timestamp dalam range
let rows = reader.read_row_groups_where(
    |rg_stats| rg_stats.timestamp_max >= start_ts && rg_stats.timestamp_min <= end_ts
)?;
```

#### Mode 4: Full Scan

Iterate semua row groups secara sekuensial. Cocok untuk transformasi batch atau export.

### Read Pipeline (Per Column Chunk)

```
Seek ke offset column chunk
        │
        ▼
Baca Column Chunk Header
        │
        ▼
Baca PAYLOAD (COMPRESSED_SIZE bytes)
        │
        ▼ [jika ENCRYPTION_ID != 0x00]
AES-256-GCM Decrypt
  → Verifikasi AUTH_TAG (tolak jika gagal)
        │
        ▼
Dekompresi (COMPRESSION_ID)
        │
        ▼
Verifikasi CRC32(decompressed) == header CRC32
        │
        ▼
Decode (ENCODING_ID) → typed column values
        │
        ▼
Return ke caller
```

---

## 8. Memory Model

Memory model QRD adalah **bounded** — memory yang digunakan tidak bergantung pada total ukuran file, melainkan hanya pada ukuran data yang sedang aktif diproses.

### Writer Memory Bounds

```
peak_memory_writer =
    row_group_size × avg_row_width_bytes      ← buffer akumulasi rows
    + column_dict_overhead                    ← untuk DICT_RLE columns
    + ecc_parity_overhead                     ← bila ECC aktif (K/N × data size)

Memory TIDAK pernah bergantung pada jumlah total rows atau ukuran file.
```

Implikasi: Writer dengan `row_group_size=50_000` dan `avg_row_width=1KB` menggunakan puncak ~50MB memory — sama apakah menulis 1 juta atau 1 miliar rows.

### Reader Memory Bounds

```
peak_memory_reader =
    footer_size                               ← selalu dimuat (biasanya < 1MB)
    + Σ(selected_column_chunk_size)           ← hanya kolom yang diminta
      × active_parallel_row_groups

Memory TIDAK pernah bergantung pada kolom yang tidak diminta.
```

Implikasi: Membaca 2 kolom dari file dengan 100 kolom menggunakan ~2% memory dari full scan.

### Memory Regression Testing

Memory bounds diverifikasi secara otomatis dalam CI:

```bash
cargo test --package qrd-core -- memory_bounds -- --nocapture
```

Setiap rilis yang melanggar memory bounds dianggap regresi dan WAJIB diperbaiki sebelum merge.

---

## 9. Cryptographic Architecture

### Key Hierarchy

```
Master Key (32 bytes)                         ← dipegang client, tidak pernah ke server
    │
    ├── HKDF-SHA256(info="qrd:col:latitude:{schema_id}")
    │       → column_key_latitude (32 bytes)
    │
    ├── HKDF-SHA256(info="qrd:col:health_val:{schema_id}")
    │       → column_key_health_val (32 bytes)
    │
    └── HKDF-SHA256(info="qrd:col:location:{schema_id}")
            → column_key_location (32 bytes)

Setiap column key digunakan untuk:
    AES-256-GCM(column_key, nonce_random, compressed_payload)
    → (ciphertext, auth_tag)
```

### Keputusan Desain Kriptografis

**Mengapa AES-256-GCM?**
AES-256-GCM adalah AEAD (Authenticated Encryption with Associated Data) — satu algoritma memberikan konfidensialitas (tidak dapat dibaca tanpa kunci) dan integritas (tidak dapat dimodifikasi tanpa terdeteksi). NIST SP 800-38D standard. Hardware acceleration tersedia di hampir semua platform modern.

**Mengapa HKDF dan bukan kunci yang berbeda per kolom secara manual?**
HKDF memungkinkan satu master key menghasilkan kunci unik yang cryptographically independent per kolom, dengan domain separation yang dapat diaudit. Ini jauh lebih aman dari "gunakan key yang berbeda untuk setiap kolom" yang sulit dikelola.

**Mengapa nonce random per chunk?**
Nonce acak (bukan counter) adalah pilihan yang aman untuk file format karena tidak membutuhkan state management yang kompleks. Probabilitas collision dengan 12-byte nonce acak adalah 1/(2^96) — dapat diabaikan.

### Crypto Primitives dan Libraries

| Fungsi | Library | Justifikasi |
|---|---|---|
| AES-256-GCM | `aes-gcm` (RustCrypto) | Constant-time, NIST validated |
| HKDF-SHA256 | `hkdf` + `sha2` (RustCrypto) | RFC 5869 conformant |
| SHA-256 (schema fingerprint) | `sha2` (RustCrypto) | Industry standard |
| CRC32 (integrity) | `crc32fast` | Hardware-accelerated |
| CSPRNG (nonce generation) | `rand::rngs::OsRng` | OS entropy source |
| Ed25519 (schema signature) | `ed25519-dalek` | RFC 8032, fast |

---

## 10. Repository Structure

```
QRD-SDK/
│
├── core/                           ← Core engine (Rust)
│   ├── qrd-core/                   ← Rust core engine — implementasi referensi
│   │   ├── src/
│   │   │   ├── schema/             ← Schema builder, serialization, fingerprint
│   │   │   ├── writer/             ← StreamingWriter, row group flush, footer write
│   │   │   ├── reader/             ← FileReader, partial reads, footer parse
│   │   │   ├── encoding/           ← 7 encoding algorithms
│   │   │   ├── compression/        ← ZSTD, LZ4, adaptive selection
│   │   │   ├── encryption/         ← AES-256-GCM, HKDF, nonce management
│   │   │   ├── ecc/                ← Reed-Solomon encode/decode/recovery
│   │   │   ├── columnar/           ← Row-to-column transposition
│   │   │   ├── integrity/          ← CRC32 computation dan verification
│   │   │   └── error/              ← Error types, structured taxonomy
│   │   ├── benches/                ← Criterion benchmark suite
│   │   └── examples/              ← Usage examples per feature
│   │
│   ├── qrd-ffi/                    ← C-compatible FFI layer
│   │   ├── src/                    ← Thin wrapper, opaque pointer management
│   │   └── include/qrd.h           ← C header file (canonical ABI contract)
│   │
│   └── qrd-wasm/                   ← WebAssembly target
│       ├── src/                    ← wasm-bindgen bindings
│       └── pkg/                    ← Generated WASM + JS glue
│
├── sdk/                            ← Language-specific SDKs
│   ├── python/                     ← PyO3 Python binding
│   ├── typescript/                 ← WASM + TypeScript packaging
│   ├── go/                         ← CGO Go binding
│   └── java/                       ← JNI Java binding
│
├── tests/                          ← Test suite
│   ├── unit/                       ← ~2,500 unit tests
│   ├── property/                   ← ~2,000 property-based tests (proptest)
│   ├── golden/                     ← ~1,500 golden vector tests
│   ├── integration/                ← ~1,500 cross-language + E2E tests
│   ├── fuzz/                       ← Fuzzing targets (libfuzzer)
│   ├── regression/                 ← Memory bounds + perf regression
│   └── compliance/                 ← NIST vectors + compliance checks
│
├── tools/                          ← CLI tools
│   ├── qrd-inspect/                ← Inspect footer, schema, stats
│   ├── qrd-verify/                 ← Verify integrity semua chunk + ECC
│   ├── qrd-convert/                ← Konversi CSV/Parquet → QRD
│   └── qrd-keygen/                 ← Generate master key dengan entropy tepat
│
├── docs/                           ← Dokumentasi detail
├── examples/                       ← Top-level usage examples
├── Cargo.toml                      ← Workspace manifest
├── Makefile                        ← Common dev commands
└── CHANGELOG.md
```

---

## 11. Dependency Graph

```
qrd-core (lib)
    ├── aes-gcm          ← encryption
    ├── hkdf             ← key derivation
    ├── sha2             ← hashing
    ├── crc32fast        ← integrity
    ├── rand             ← nonce generation
    ├── ed25519-dalek    ← schema signing (optional feature)
    ├── zstd             ← compression
    ├── lz4              ← compression
    └── reed-solomon     ← ECC

qrd-ffi (cdylib / staticlib)
    └── qrd-core

qrd-wasm (wasm)
    ├── qrd-core
    └── wasm-bindgen

sdk/python
    ├── qrd-core (via PyO3)
    └── pyo3

sdk/typescript
    └── qrd-wasm (npm package)

sdk/go
    └── qrd-ffi (via CGO)

sdk/java
    └── qrd-ffi (via JNI)
```

**Prinsip dependency:** Tidak ada SDK yang memiliki dependency pada SDK lain. Semua dependency konvergen ke `qrd-core`.

---

## 12. CI/CD Pipeline

### Workflows

| Workflow | Trigger | Jobs |
|---|---|---|
| `ci.yml` | Push, PR | `cargo test --workspace`, clippy, fmt, fuzz smoke |
| `golden.yml` | Push ke main | Regenerate + validate golden vectors semua SDK |
| `bench.yml` | Tag release | Benchmark run + comparison dengan baseline |
| `audit.yml` | Weekly | `cargo audit` untuk dependency vulnerabilities |
| `cross-lang.yml` | PR + push | Cross-language integration tests (semua SDK) |

### Quality Gates

Setiap PR WAJIB lulus semua gate berikut sebelum merge:

```bash
# Gate 1: Unit dan integration tests
cargo test --workspace

# Gate 2: Linting
cargo clippy --workspace -- -D warnings

# Gate 3: Formatting
cargo fmt --all -- --check

# Gate 4: Property tests (CI default 1000 cases)
PROPTEST_CASES=1000 cargo test --package qrd-core -- proptest

# Gate 5: Fuzz smoke (singkat, per CI run)
cargo +nightly fuzz run parse_header -- -max_total_time=60
cargo +nightly fuzz run parse_footer -- -max_total_time=60
```

### Release Process

1. Semua tests hijau di main branch
2. Update `CHANGELOG.md` dengan semua perubahan sejak release terakhir
3. Tag release dengan format `vMAJOR.MINOR.PATCH`
4. CI otomatis: build artifacts, publish ke crates.io / PyPI / npm / Maven
5. Untuk MAJOR release: wajib external security review sebelum tag

---

*QRD-SDK Architecture Document v1.0*  
*Pertanyaan arsitektur: buka GitHub Discussion atau issue dengan label `architecture`*
