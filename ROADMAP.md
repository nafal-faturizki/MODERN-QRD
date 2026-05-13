# QRD-SDK Roadmap

**Document Type:** Product Roadmap — Living Document  
**Audience:** Adopters, contributors, enterprise evaluators  
**Version:** 1.0 · Last Updated: 2025-01

> Roadmap QRD terorganisir per phase berdasarkan **kematangan teknis dan exit criteria**, bukan tanggal kalender. Setiap phase memiliki daftar deliverables dan exit criteria yang harus terpenuhi sepenuhnya sebelum phase berikutnya dimulai. Pendekatan ini lebih jujur terhadap kompleksitas software infrastruktur daripada deadline artifisial.

---

## Status Saat Ini

| Phase | Status | Keterangan |
|---|---|---|
| **Phase 1 — Foundation** | 🔄 In Progress | Core engine aktif dikembangkan |
| **Phase 2 — Hardening & Compliance** | 📋 Planned | Menunggu Phase 1 selesai |
| **Phase 3 — Composite Types & Query** | 📋 Planned | Menunggu Phase 2 selesai |
| **Phase 4 — Extended Ecosystem** | 🔮 Future | Exploratory |
| **Phase 5 — Formal Verification & Post-Quantum** | 🔮 Future | Long-term research |

---

## Phase 1 — Foundation

**Tujuan:** Menghasilkan Rust core engine yang stabil, benar, aman, dan siap diaudit. Semua SDK bahasa utama tersedia dan divalidasi dengan golden vector tests.

**Filosofi Phase 1:** Correctness sebelum performa. Keamanan sebelum fitur. Setiap komponen harus benar sebelum dioptimasi.

### Deliverables

#### Core Engine
- [x] Rust core engine dengan streaming writer dan partial column reader
- [x] 7 encoding algorithms: PLAIN, RLE, BIT_PACKED, DELTA_BINARY, DELTA_BYTE_ARRAY, BYTE_STREAM_SPLIT, DICTIONARY_RLE
- [x] ZSTD dan LZ4 compression dengan adaptive selection
- [x] AES-256-GCM per-column encryption dengan HKDF-SHA256
- [x] Reed-Solomon ECC (configurable RS(N, K) per row group)
- [x] CRC32 per-chunk dan per-footer integrity verification
- [x] Zero-panic parser dengan strict bounds checking
- [x] Bounded memory guarantees (writer dan reader)

#### Interface Layer
- [x] C FFI layer dengan stable ABI (`core/qrd-ffi/`)
- [x] WASM target untuk browser dan Node.js (`core/qrd-wasm/`)

#### Language SDKs
- [x] Python SDK (PyO3) — stable, di PyPI
- [x] TypeScript SDK (WASM) — stable, di npm
- [x] Go SDK (CGO) — stable
- [x] Java SDK (JNI) — stable, di Maven Central
- [x] C/C++ SDK (C FFI header + static lib) — stable

#### Quality Assurance
- [ ] Test suite mencapai 10.000+ test cases (unit, property, golden, integration, fuzz, regression, compliance)
- [ ] Fuzzing corpus: 100K+ corpus entries per target (parse_header, parse_footer, parse_column_chunk, decode_rle, decode_delta, decrypt_chunk)
- [ ] Audit kriptografis oleh firma independen
- [ ] Golden vector suite tersedia untuk semua 6 SDK dengan cross-language validation

#### CLI Tools
- [x] `qrd-inspect` — inspeksi footer, schema, statistik tanpa membaca payload
- [x] `qrd-verify` — verifikasi integritas semua chunk + ECC check
- [x] `qrd-convert` — konversi CSV/Parquet → QRD (satu arah)
- [x] `qrd-keygen` — generate master key dengan entropy yang tepat

### Exit Criteria Phase 1

Phase 1 dianggap selesai ketika **semua** kriteria berikut terpenuhi:

```
□ Semua deliverables di atas selesai (✓ atau verified)
□ Audit kriptografis independen selesai, semua finding severity High/Critical sudah di-remediate
□ Test suite: ≥ 10.000 test cases, semua hijau di CI
□ Fuzz corpus: ≥ 100K entries per target, tidak ada crash baru dalam 30 hari terakhir
□ Cross-language golden vector tests lulus di semua 6 SDK
□ Memory regression tests: writer dan reader tidak melebihi bounds pada 1M rows
□ Documentation: README, SPECIFICATION, ARCHITECTURE, SECURITY, ROADMAP lengkap
□ Benchmark baseline tersimpan untuk semua operasi kunci
□ Tidak ada known security vulnerability yang belum di-remediate
```

---

## Phase 2 — Hardening & Compliance

**Tujuan:** Menjadikan QRD production-ready untuk regulated industries — healthcare (HIPAA), keuangan (SOC 2), dan enterprise deployment umum. Menambahkan bahasa tambahan dan tooling yang dibutuhkan untuk adopsi skala besar.

**Filosofi Phase 2:** Production-readiness adalah lebih dari sekedar "bekerja" — ini tentang keamanan yang dapat diaudit, documentation yang cukup untuk compliance, dan tooling yang memudahkan operator.

### Deliverables

#### Security Hardening
- [ ] **Constant-time AES-GCM verification path** — eliminasi timing side-channel dalam verify path
- [ ] **FIPS 140-3 Level 1 alignment verification** — dokumentasi formal bahwa implementasi memenuhi persyaratan operasional (bukan sertifikasi lab, namun dapat di-cite dalam compliance docs)
- [ ] **Formal spec dalam format RFC-style** — untuk third-party implementors dan compliance teams
- [ ] **Schema signing via Ed25519 sebagai fitur stabil** (saat ini ada, tapi belum fully documented)

#### Compliance Documentation
- [ ] **Deployment guide untuk healthcare (HIPAA)** — cara mengkonfigurasi QRD untuk memenuhi persyaratan PHI (Protected Health Information)
- [ ] **Deployment guide untuk keuangan (SOC 2)** — cara mengkonfigurasi QRD untuk audit trail finansial
- [ ] **Edge telemetry deployment guide** — IoT dan edge device deployment patterns
- [ ] **Key management integration guide** — integrasi dengan HashiCorp Vault, AWS KMS, Azure Key Vault, GCP Secret Manager

#### CLI Tools (Production-Ready)
- [ ] `qrd-inspect`: production-ready dengan output JSON untuk machine parsing, schema diff, format version detection
- [ ] `qrd-verify`: production-ready dengan exit codes yang konsisten untuk CI/CD integration
- [ ] `qrd-convert`: bidireksional CSV ↔ QRD, validasi schema saat konversi
- [ ] `qrd-migrate`: tool untuk re-encryption file dengan kunci baru (key rotation)

#### Language Expansion
- [ ] **Swift SDK (iOS/macOS edge)** — untuk health dan fitness applications di Apple platform
- [ ] **Kotlin/Android SDK** — untuk edge telemetry di Android devices
- [ ] **.NET/C# SDK** — untuk enterprise integrations yang menggunakan .NET ecosystem

### Exit Criteria Phase 2

```
□ Constant-time AES-GCM path terverifikasi oleh audit eksternal
□ FIPS 140-3 alignment documented dan peer-reviewed
□ RFC-style spec dipublikasikan di docs/FORMAT_RFC.md
□ HIPAA dan SOC 2 deployment guides direview oleh compliance expert
□ Semua CLI tools memiliki test suite comprehensive + man page
□ Swift, Kotlin, .NET SDKs lulus golden vector cross-language tests
□ Key rotation workflow documented dan ditest end-to-end
```

---

## Phase 3 — Composite Types & Query Layer

**Tujuan:** Meningkatkan expressiveness format dengan composite types, dan menambahkan kemampuan query terbatas untuk analytical use cases tanpa membutuhkan query engine eksternal.

**Filosofi Phase 3:** Kompleksitas format harus dibayar dengan value yang nyata. Setiap fitur baru harus memiliki use case yang jelas dan tidak boleh mengorbankan keamanan atau bounded-memory guarantee.

### Deliverables

#### Format Extensions
- [ ] **`STRUCT` type** — named nested field set dalam format binary
- [ ] **`ARRAY` type** — homogeneous variable-length list per row
- [ ] Binary encoding spec untuk STRUCT dan ARRAY (normative, dengan golden vectors)

#### Reader Enhancements
- [ ] **Predicate pushdown** — filter row groups berdasarkan min/max statistics di footer
- [ ] **Bloom filter per column chunk** — untuk point lookup yang efisien
- [ ] **Predicate-aware partial reads** — skip row groups yang tidak memenuhi filter tanpa membaca payload

#### Query Layer
- [ ] **`qrd-query` tool** — minimal SQL-like query engine di atas partial reads untuk single file
  ```sql
  SELECT device_id, AVG(health_val)
  FROM telemetry.qrd
  WHERE timestamp >= 1700000000000000
  GROUP BY device_id
  ```
- [ ] Syntax terbatas: `SELECT`, `FROM`, `WHERE` (dengan comparisons dan range), `GROUP BY`, aggregasi dasar (AVG, SUM, COUNT, MIN, MAX)

#### Schema Evolution
- [ ] **Schema evolution tooling** — detect dan migrate compatible schema changes
- [ ] **Schema diff tool** — bandingkan dua schema dan identifikasi breaking vs non-breaking changes

### Exit Criteria Phase 3

```
□ STRUCT dan ARRAY types memiliki golden vectors cross-language
□ Predicate pushdown terverifikasi dengan benchmark: ≥ 10× speedup pada selective queries
□ qrd-query: lulus test suite dengan 500+ query test cases
□ Schema evolution: roundtrip migration tests untuk semua compatible change scenarios
□ Memory bounds tetap terjaga dengan composite types (regression tests update)
```

---

## Phase 4 — Extended Ecosystem

**Tujuan:** Memperluas interoperabilitas QRD dengan ekosistem data yang lebih luas, dan menambahkan abstraksi untuk multi-file datasets.

**Filosofi Phase 4:** QRD tidak harus menggantikan semua format — cukup berinteroperasi dengan baik dengan format yang sudah ada.

### Deliverables

#### Interoperabilitas
- [ ] **Konversi bidireksional Parquet ↔ QRD** — dengan dokumentasi eksplisit tentang keterbatasan (enkripsi tidak dapat dipertahankan saat konversi ke Parquet)
- [ ] **Arrow IPC integration** — QRD sebagai persistent storage layer, Arrow sebagai in-memory layer
- [ ] **`MAP` type** — key-value pairs dengan typed key dan value

#### Streaming Protocol
- [ ] **QRD over TCP/QUIC** — streaming protocol untuk real-time telemetry pipelines dengan framing yang efisien
- [ ] Protocol spec: connection establishment, row group streaming, error recovery, flow control

#### Multi-File Dataset
- [ ] **Multi-file dataset abstraction** — shared schema registry untuk koleksi file QRD yang merepresentasikan satu logical dataset
- [ ] Dataset-level statistics dan indexing (opsional, tidak mengubah format per-file)

#### Cryptographic Extensions
- [ ] **Formal ZK proof system exploration** — investigasi integrasi dengan ZK-SNARK atau ZK-STARK untuk provability yang lebih kuat
- [ ] **Post-quantum key encapsulation (exploratory)** — evaluasi CRYSTALS-Kyber (ML-KEM, NIST standard) sebagai kandidat

### Exit Criteria Phase 4

```
□ Parquet bidireksional: roundtrip test suite, limitations documented
□ Arrow integration: benchmark memory overhead vs native Arrow IPC
□ MAP type: golden vectors cross-language
□ TCP/QUIC protocol: reference implementation + interop test dengan dua independent implementations
□ Post-quantum: research report dengan rekomendasi, bukan implementasi produksi
```

---

## Phase 5 — Formal Verification & Post-Quantum

**Tujuan:** Memberikan jaminan keamanan jangka panjang melalui formal verification subset kritis dan transisi ke kriptografi post-quantum.

**Filosofi Phase 5:** Ini adalah investasi jangka panjang untuk ekosistem yang bergantung pada QRD selama 10+ tahun. Format yang didesain hari ini harus bertahan dari ancaman komputasi kuantum.

### Deliverables

#### Formal Verification
- [ ] **Formal verification parser Rust** menggunakan Prusti atau Kani untuk subset critical paths (parse_footer_length, parse_column_chunk_header, validate_bounds)
- [ ] Coverage: zero-panic guarantee untuk semua external input paths dalam scope verification

#### Post-Quantum Cryptography
- [ ] **Post-quantum key encapsulation** — CRYSTALS-Kyber (ML-KEM, NIST FIPS 203) sebagai mekanisme encapsulation kunci
- [ ] **Hybrid classical+post-quantum key derivation** — transisi yang aman: `column_key = HKDF(classical_secret XOR pq_secret)`
- [ ] Format extension untuk menyimpan KEM public key dan encapsulated secret di footer
- [ ] Migration path dari file QRD dengan classical keys ke post-quantum keys

#### HSM Integration
- [ ] **Hardware Security Module integration guide** — menggunakan HSM sebagai key derivation backend, bukan software OsRng
- [ ] Reference implementation dengan PKCS#11 interface

### Exit Criteria Phase 5

```
□ Formal verification: tool-verified zero-panic untuk critical paths
□ ML-KEM integration: NIST test vectors lulus, spec update ke format v2.0
□ Hybrid key derivation: migrasi backward-compatible dari v1.0 ke v2.0
□ HSM guide: ditest dengan minimal dua HSM vendor (software emulator acceptable)
□ Security audit eksternal untuk seluruh Phase 5 changes
```

---

## Prinsip Evolusi Format

### Breaking vs Non-Breaking Changes

Format QRD menggunakan MAJOR.MINOR versioning. Aturan ini tidak berubah:

| Jenis Perubahan | Versi | Contoh |
|---|---|---|
| Tambah fitur opsional (field baru di footer, flag baru, codec baru) | MINOR bump | Menambahkan `MAP` type |
| Perubahan yang mengubah parsing existing fields | MAJOR bump | Mengubah struktur Column Chunk Header |
| Perubahan format yang tidak backward-compatible | MAJOR bump | Mengubah byte order atau MAGIC bytes |

### Komitmen Backward Compatibility

- Reader v1.x WAJIB dapat membaca file yang ditulis oleh writer v1.y untuk semua y ≤ x
- Reader v2.x TIDAK WAJIB dapat membaca file v1.x (tapi BOLEH dengan fallback mode)
- Setiap MAJOR version akan didukung minimal 2 tahun setelah MAJOR version berikutnya dirilis

### Contribution ke Roadmap

Proposal untuk fitur baru atau perubahan roadmap dikirim melalui GitHub Issues dengan label `roadmap`. Setiap proposal HARUS menyertakan:

1. **Use case yang jelas** — siapa yang membutuhkan, masalah apa yang dipecahkan
2. **Impact analysis** — apakah membutuhkan format change? Breaking?
3. **Security implications** — apakah ada dampak terhadap threat model?
4. **Alternative approaches** — kenapa solusi yang diusulkan lebih baik dari alternatif?

---

## Apa yang Tidak Akan Berubah

Beberapa properti fundamental QRD **tidak akan berubah** dalam versi manapun — ini adalah kontrak jangka panjang dengan adopter:

- **Rust core engine sebagai single source of truth** — tidak akan ada implementasi kedua dalam bahasa lain
- **Privacy-native by design** — enkripsi tidak akan pernah menjadi fitur opsional yang off by default
- **Bounded memory guarantee** — writer dan reader memory tidak boleh bergantung pada ukuran total file
- **Deterministic binary output** — input identik menghasilkan binary identik (kecuali field kriptografis)
- **Zero-panic policy di parser** — tidak ada input adversarial yang menyebabkan panic

---

*QRD-SDK Roadmap v1.0*  
*Untuk diskusi roadmap: buka GitHub Issue dengan label `roadmap`*  
*Untuk pertanyaan umum: [docs.qrd.dev](https://docs.qrd.dev)*
