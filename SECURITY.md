# QRD-SDK Security Reference

**Document Type:** Security Reference — Normative  
**Audience:** Security engineers, auditors, compliance teams, enterprise adopters  
**Version:** 1.0  
**Security Contact:** [security@qrd.dev](mailto:security@qrd.dev) · PGP: lihat bagian [Responsible Disclosure](#8-responsible-disclosure)

> Dokumen ini mencakup model kepercayaan, threat model, desain kriptografis, keterbatasan keamanan yang diketahui, dan kebijakan disclosure QRD-SDK. Baca dokumen ini sebelum mendeploykan QRD dalam sistem yang menangani data sensitif.

---

## Daftar Isi

1. [Trust Model](#1-trust-model)
2. [Properti Keamanan Format](#2-properti-keamanan-format)
3. [Threat Model](#3-threat-model)
4. [Batasan Keamanan Eksplisit](#4-batasan-keamanan-eksplisit)
5. [Desain Kriptografis](#5-desain-kriptografis)
6. [Parser Hardening](#6-parser-hardening)
7. [Audit dan Compliance](#7-audit-dan-compliance)
8. [Responsible Disclosure](#8-responsible-disclosure)
9. [Panduan Deployment Aman](#9-panduan-deployment-aman)

---

## 1. Trust Model

### Asumsi Dasar

QRD dirancang dengan asumsi bahwa **tidak ada infrastruktur antara penulis data dan penerima data yang dapat dipercaya secara penuh**. Ini mencakup:

| Komponen | Level Kepercayaan | Mekanisme |
|---|---|---|
| **Client yang memegang kunci** | Dipercaya penuh | Kepercayaan diberikan kepada pemegang master key |
| **Cloud storage** | Tidak dipercaya | Server hanya menyimpan ciphertext; tidak dapat membaca plaintext |
| **Transport layer** | Tidak dipercaya oleh format | TLS tetap direkomendasikan; AES-GCM memberikan lapisan enkripsi redundan |
| **Intermediate processors** | Tidak dipercaya | Setiap processor yang tidak memegang kunci hanya melihat ciphertext |
| **Shared storage systems** | Tidak dipercaya | Enkripsi per-kolom menjamin isolasi bahkan di storage shared |
| **Rust core engine** | Dipercaya (dapat diaudit) | Source code publik, dapat direview dan di-reproduce |
| **Format specification** | Dipercaya | Publik, deterministik, normative |

### Definisi Zero-Knowledge dalam Konteks QRD

QRD menggunakan istilah "zero-knowledge" dalam arti **storage-level zero-knowledge**: server atau storage layer yang menyimpan file QRD **tidak dapat memperoleh informasi tentang nilai plaintext kolom terenkripsi** tanpa kunci dekripsi yang valid.

Ini berbeda dari "zero-knowledge proof" dalam arti kriptografis formal (ZKP). Properti ini lebih tepat disebut **semantic security (IND-CPA)** — ciphertext tidak mengungkapkan informasi tentang plaintext tanpa kunci.

Properti ini lebih kuat dari "encryption at rest" biasa karena:
- Enkripsi bersifat per-kolom dengan kunci yang berbeda
- Statistik (min/max/distinct_count) dapat dienkripsi bersama payload
- Server tidak perlu memegang kunci master; HKDF key derivation terjadi di sisi client

---

## 2. Properti Keamanan Format

### Properti yang Dijamin

| Properti | Mekanisme | Kekuatan |
|---|---|---|
| **Confidentiality** kolom terenkripsi | AES-256-GCM | 256-bit key, IND-CPA secure |
| **Integrity** payload terenkripsi | AES-256-GCM AUTH_TAG | Unforgeable tanpa kunci (128-bit tag) |
| **Integrity** payload plaintext | CRC32 per column chunk | Deteksi korupsi non-adversarial |
| **Integrity** footer/schema | CRC32 footer checksum | Deteksi korupsi, early rejection |
| **Authenticity** schema | Ed25519 signature (opsional) | Verifiable tanpa kepercayaan ke storage |
| **Durability** data di storage terdegradasi | Reed-Solomon ECC | Recovery hingga K dari N chunks |
| **Confidentiality** statistik distribusi | Enkripsi statistik (opsional) | Min/max/distinct tidak bocor ke storage |

### Properti yang Tidak Dijamin (Lihat §4)

- Confidentiality kunci yang dikompromikan
- Protection terhadap timing side-channel
- Confidentiality metadata struktural (ukuran file, jumlah row group)
- Protection data setelah didekripsi di memori

---

## 3. Threat Model

### Aset yang Dilindungi

| Aset | Sensitivitas | Mekanisme Perlindungan |
|---|---|---|
| Plaintext payload kolom terenkripsi | **Tinggi** | AES-256-GCM per-column key |
| Kunci derivasi (column keys) | **Sangat Tinggi** | Tidak pernah disimpan; derived on-demand |
| Schema dan field names kolom sensitif | **Menengah** | Optional schema signing + metadata omission |
| Statistik distribusi data terenkripsi | **Menengah** | `STATS_ENCRYPTED` flag |
| Integritas format (non-adversarial) | **Tinggi** | CRC32 per-chunk dan per-footer |
| Ketersediaan data di storage terdegradasi | **Menengah** | Reed-Solomon ECC |

### Threat Actors dan Mitigasi

---

#### THREAT 1: Curious Storage Provider (On-Path, Passive)

**Deskripsi:** Cloud storage operator membaca file QRD yang disimpan di infrastruktur mereka. Mereka memiliki akses penuh ke file binary tetapi tidak memegang kunci.

**Mitigasi:**
- Kolom terenkripsi menghasilkan ciphertext yang tidak dapat diinterpretasikan tanpa kunci
- Jika `STATS_ENCRYPTED = 1`, statistik distribusi (min/max/distinct_count) juga tidak dapat dibaca
- Nama kolom sensitif dapat di-obfuscate via field `KEY_ID` dan metadata

**Status: ✅ Dimitigasi oleh desain format** — dengan catatan bahwa metadata struktural (jumlah rows, ukuran file) tetap visible.

---

#### THREAT 2: Passive Network Eavesdropper

**Deskripsi:** Penyerang mengintersep file QRD dalam transit antara edge device dan cloud storage.

**Mitigasi:**
- Di luar scope format QRD — gunakan TLS untuk transport layer
- AES-GCM memberikan lapisan enkripsi redundan: bahkan tanpa TLS, ciphertext tidak dapat dibaca tanpa kunci
- Reed-Solomon ECC dapat membantu recovery dari partial corruption selama transmisi

**Status: ⚠️ Partial** — transport security adalah tanggung jawab operator, bukan format.

---

#### THREAT 3: Malicious File (Parser Attack)

**Deskripsi:** Penyerang mengirim file QRD yang di-craft secara khusus untuk menyebabkan parser crash, panic, integer overflow, atau out-of-bounds memory access.

**Mitigasi:**
- **Zero-panic policy:** Parser TIDAK BOLEH panic pada input adversarial apapun
- Semua size fields divalidasi sebelum alokasi memori
- `checked_*` arithmetic mencegah integer overflow
- Fuzz testing berkelanjutan terhadap semua parsing entrypoints
- Fail-fast dengan error eksplisit pada encoding/compression ID tidak dikenal

**Status: ✅ Dimitigasi oleh parser hardening** — lihat §6 untuk detail.

---

#### THREAT 4: Storage Corruption (Non-Adversarial)

**Deskripsi:** Bit rot, media failure, partial write, atau transmission error mengakibatkan corruption pada file QRD.

**Mitigasi:**
- CRC32 per column chunk: deteksi corruption saat baca
- AES-GCM auth tag: deteksi modifikasi pada kolom terenkripsi
- Footer CRC32: early rejection file dengan footer rusak
- Reed-Solomon ECC: recovery hingga K chunks yang korup atau hilang

**Status: ✅ Dimitigasi dengan ECC aktif**

---

#### THREAT 5: Schema Tampering

**Deskripsi:** Penyerang memodifikasi schema footer untuk mengubah tipe data, menambah kolom palsu, atau mengubah nullability field — dengan tujuan menyebabkan incorrect parsing di reader.

**Mitigasi:**
- Footer CRC32 mendeteksi setiap modifikasi footer
- SCHEMA_ID di header di-cross-validate dengan SHA-256 fingerprint schema dari footer
- Optional Ed25519 schema signature untuk non-repudiation kriptografis

**Status: ✅ Dimitigasi** — signature opsional namun sangat direkomendasikan untuk sistem compliance.

---

#### THREAT 6: Nonce Reuse (AES-GCM)

**Deskripsi:** Nonce AES-GCM yang sama digunakan dua kali dengan kunci yang sama. Nonce reuse pada AES-GCM mengakibatkan key recovery yang sepenuhnya menggagalkan confidentiality.

**Mitigasi:**
- Nonce 12-byte di-generate secara kriptografis acak per column chunk menggunakan `OsRng`
- Probabilitas collision: 1/(2^96) ≈ 10^-29 — dapat diabaikan dalam skenario penggunaan normal
- NonceGenerator tidak menyimpan state — setiap chunk mendapat nonce fresh

**Status: ✅ Dimitigasi secara probabilistik**

---

#### THREAT 7: Unauthorized Column Access (Privilege Escalation)

**Deskripsi:** Reader yang hanya berhak membaca kolom A mencoba membaca kolom B yang dienkripsi dengan kunci berbeda.

**Mitigasi:**
- Enkripsi per-kolom dengan kunci yang berbeda (via HKDF domain separation)
- Reader yang tidak memegang kunci untuk kolom B tidak dapat mendekripsinya
- Kolom B menghasilkan `Error::AuthenticationFailed` tanpa mengekspos informasi apapun

**Status: ✅ Dimitigasi oleh desain per-column key**

---

## 4. Batasan Keamanan Eksplisit

QRD **tidak** melindungi terhadap skenario berikut. Ini bukan bug — ini adalah batasan yang disengaja yang perlu dipahami oleh operator.

### Kunci yang Dikompromikan

Jika master key bocor atau dikompromikan, semua kolom terenkripsi yang menggunakan kunci tersebut dapat dibaca oleh pihak yang tidak berwenang. QRD tidak memiliki mekanisme key revocation internal — rotasi kunci memerlukan re-encryption file menggunakan kunci baru.

**Rekomendasi:** Implementasikan key management yang solid (HSM, secret management service) sebelum menggunakan QRD untuk data sangat sensitif.

### Timing Side-Channel

Library AES-GCM yang digunakan (`aes-gcm` RustCrypto) menggunakan constant-time implementation untuk operasi kriptografis inti. Namun, timing variance di luar library (memory allocation, cache effects) mungkin ada dalam kondisi tertentu. Untuk use case high-security yang memerlukan constant-time guarantee absolut, gunakan implementation yang telah diverifikasi secara formal.

Constant-time verification path akan ditambahkan di Phase 2 (lihat ROADMAP.md).

### Metadata Inference

Informasi struktural berikut **tetap visible** bahkan pada file QRD yang sepenuhnya terenkripsi:

- Ukuran total file
- Jumlah row group dan ukuran per row group
- Jumlah kolom dan nama kolom (kecuali di-obfuscate secara manual)
- Timestamp pembuatan file (di header)

Penyerang yang dapat mengobservasi ukuran file dari waktu ke waktu mungkin dapat menginferensi laju data ingestion.

### Attacker dengan Akses Memori Runtime

Enkripsi QRD melindungi data **at rest** dan **in transit**. Setelah data didekripsi di memori (dalam proses reader), data tersebut ada dalam plaintext di RAM. Memory dump, cold boot attack, atau attacker dengan akses root pada sistem yang sama dapat mengekspos data yang sedang aktif didekripsi.

### Implementasi SDK yang Tidak Diaudit

Jaminan keamanan penuh hanya berlaku untuk **Rust core engine** (`qrd-core`). Binding language lain (Python, Go, Java, TypeScript) adalah lapisan tipis yang secara teoritis dapat memiliki kerentanan di lapisan binding tersebut. Audit kriptografis mencakup core engine; lapisan binding di-review untuk correctness tapi tidak dengan kedalaman audit yang sama.

---

## 5. Desain Kriptografis

### Primitive Selection Rationale

Setiap keputusan kriptografis dalam QRD didasarkan pada justifikasi yang dapat diaudit:

#### AES-256-GCM

**Dipilih karena:**
- AEAD: memberikan confidentiality dan authenticity dalam satu primitive
- NIST SP 800-38D standard — dapat di-cite dalam compliance documentation
- Hardware acceleration tersedia di seluruh platform modern (AES-NI, ARMv8 AES)
- 256-bit key size memberikan keamanan yang memadai bahkan terhadap theoretical quantum attacks (Grover's algorithm membutuhkan 2^128 operasi, bukan 2^256)
- Library `aes-gcm` (RustCrypto) telah melalui review publik dan digunakan secara luas

**Trade-off yang diterima:**
- Nonce reuse catastrophic (mitigasi: nonce random per chunk)
- Tidak quantum-resistant untuk authenticated encryption (post-quantum exploration di Phase 5)

#### HKDF-SHA256

**Dipilih karena:**
- RFC 5869 conformant — standar yang well-understood dan banyak diaudit
- Domain separation yang kuat via `info` parameter
- Memungkinkan satu master key menghasilkan kunci yang cryptographically independent per kolom

**Parameter:**
```
IKM:   master_key (32 bytes)
Salt:  file_salt (32 bytes random, disimpan di footer)
Info:  "qrd:col:{column_name}:{schema_id_hex}"
OKM:   32 bytes (column key)
```

#### CRC32 (Non-Cryptographic Integrity)

CRC32 digunakan untuk deteksi korupsi non-adversarial (storage errors, transmission errors). CRC32 **bukan** cryptographic hash — penyerang yang ingin memodifikasi payload tanpa deteksi dapat menghitung CRC32 baru.

Untuk kolom terenkripsi, AES-GCM AUTH_TAG memberikan cryptographic integrity. CRC32 adalah pelengkap untuk kolom plaintext dan sebagai early-rejection mechanism untuk footer.

#### Ed25519 Schema Signature (Opsional)

Schema footer dapat ditandatangani dengan Ed25519 untuk membuktikan bahwa schema berasal dari pemegang private key yang dikenal. Ini berguna untuk:
- Audit trails yang membutuhkan non-repudiation
- Multi-party scenarios di mana reader perlu memverifikasi bahwa schema tidak dimodifikasi oleh storage

### Crypto Standards Compliance

| Standard | Penggunaan dalam QRD |
|---|---|
| NIST SP 800-38D | AES-GCM mode of operation |
| RFC 5869 | HKDF key derivation |
| RFC 8032 | Ed25519 signature scheme |
| FIPS 197 | AES block cipher |
| IEEE 802.3 | CRC32 polynomial (0xEDB88320) |
| FIPS 140-3 Level 1 | Alignment (operasional, bukan sertifikasi penuh) |

---

## 6. Parser Hardening

### Zero-Panic Policy

Core engine memiliki komitmen **zero-panic pada input adversarial**. Artinya: tidak ada input external yang seharusnya dapat menyebabkan Rust `panic!()` di dalam `qrd-core`.

Komitmen ini diverifikasi melalui:
- **Fuzz testing** berkelanjutan terhadap semua parsing entrypoints
- **Property tests** dengan input yang di-generate secara random
- **Clippy lints** yang melarang `unwrap()` dan `expect()` di code paths yang menerima external input

### Strict Input Validation

Setiap field external divalidasi sebelum digunakan:

```rust
// Contoh: validasi footer length sebelum alokasi
fn parse_footer_length(file: &mut impl Read + Seek) -> Result<u32> {
    let file_size = file.seek(SeekFrom::End(0))?;
    ensure!(file_size >= HEADER_SIZE + 4, Error::FileTooSmall { file_size });
    
    file.seek(SeekFrom::End(-4))?;
    let footer_len = file.read_u32::<LittleEndian>()?;
    
    // Validasi range SEBELUM alokasi
    ensure!(
        footer_len > 0 && footer_len <= file_size.saturating_sub(HEADER_SIZE + 4),
        Error::InvalidFooterLength { footer_len, file_size }
    );
    Ok(footer_len)
}
```

### Arithmetic Safety

Semua operasi integer yang dapat overflow menggunakan `checked_*` arithmetic:

```rust
// Benar: checked arithmetic
let end_offset = start_offset.checked_add(chunk_size)
    .ok_or(Error::IntegerOverflow { field: "chunk_end_offset" })?;

// Salah (tidak diizinkan di qrd-core): wrapping arithmetic
let end_offset = start_offset + chunk_size;  // TIDAK DIIZINKAN
```

### Unsafe Rust Policy

Semua blok `unsafe` dalam `qrd-core` WAJIB:
1. Didokumentasikan dengan komentar `// SAFETY:` yang menjelaskan invariant yang menjamin keamanan
2. Di-review oleh minimal dua maintainer sebelum merge
3. Di-justify bahwa tidak ada safe alternative yang equivalent dalam performa

### Fuzz Testing Coverage

Fuzz targets yang aktif (continuous, bukan hanya saat release):

| Target | Deskripsi |
|---|---|
| `parse_header` | Arbitrary bytes sebagai file header |
| `parse_footer` | Arbitrary bytes sebagai file footer |
| `parse_column_chunk` | Arbitrary bytes sebagai column chunk |
| `decode_rle` | Arbitrary bytes melalui RLE decoder |
| `decode_delta` | Arbitrary bytes melalui DELTA decoder |
| `decrypt_chunk` | Arbitrary ciphertext melalui AES-GCM (no panic guarantee) |

Target kumulatif: **100K+ corpus entries per target** sebelum rilis Phase 1.

---

## 7. Audit dan Compliance

### Cryptographic Audit

QRD berkomitmen untuk **audit kriptografis independen** oleh firma eksternal sebelum setiap major release. Scope audit mencakup:

- AES-256-GCM implementation: correctness, nonce generation, auth tag verification
- HKDF key derivation: parameter selection, domain separation, output size
- CRC32 implementation: polynomial, endianness, collision resistance awareness
- Reed-Solomon: galois field arithmetic, encoding/decoding correctness
- Parser hardening: semua external input paths, integer overflow, OOB access

Laporan audit tersedia di: [`docs/security/SECURITY_AUDIT.md`](docs/security/SECURITY_AUDIT.md)

### Compliance Framework

| Framework | Status | Catatan |
|---|---|---|
| **FIPS 140-3 Level 1** | Aligned (bukan sertifikasi penuh) | AES-GCM dan HKDF mengikuti standar NIST |
| **HIPAA** | Panduan tersedia di Phase 2 | Deployment guide untuk healthcare |
| **SOC 2 Type II** | Panduan tersedia di Phase 2 | Untuk adopter di industri keuangan |
| **GDPR** | Didukung oleh desain | Zero-knowledge storage mendukung right-to-erasure via key deletion |

### Dependency Audit

Security audit dependency dilakukan setiap minggu via GitHub Actions:

```bash
# Dijalankan di workflow audit.yml setiap minggu
cargo audit
```

Advisory yang ditemukan di dependency cryptographic di-escalate ke security@qrd.dev dan HARUS diperbaiki sebelum rilis berikutnya.

---

## 8. Responsible Disclosure

### Melaporkan Kerentanan

**Email:** [security@qrd.dev](mailto:security@qrd.dev)  
**Enkripsi:** Gunakan PGP key berikut untuk laporan sensitif

```
-----BEGIN PGP PUBLIC KEY BLOCK-----
[PGP public key tersedia di SECURITY.md di repository]
-----END PGP PUBLIC KEY BLOCK-----
```

### SLA Response

| Severity | Acknowledgment | Initial Assessment | Fix Target |
|---|---|---|---|
| **Critical** (RCE, key compromise) | 24 jam | 48 jam | 7 hari |
| **High** (data exposure, auth bypass) | 48 jam | 72 jam | 14 hari |
| **Medium** (integrity bypass, DoS) | 72 jam | 1 minggu | 30 hari |
| **Low** (informational, minor) | 1 minggu | 2 minggu | Next release |

### Scope

Kerentanan yang masuk dalam scope disclosure:

- Bug di `qrd-core` yang menyebabkan confidentiality atau integrity compromise
- Kerentanan parser yang menyebabkan panic, crash, atau arbitrary code execution
- Kelemahan dalam implementasi kriptografis (AES-GCM, HKDF, ECC)
- Bug di FFI layer yang mengekspos memory atau menyebabkan unsafe behavior

Di luar scope:
- Bug di contoh kode atau dokumentasi
- Kerentanan di dependency pihak ketiga yang belum ada advisory resmi
- Skenario yang membutuhkan akses fisik ke device yang sudah dikompromikan

### Coordinated Disclosure

QRD mengikuti coordinated disclosure: reporter diberikan waktu untuk berkomunikasi tentang timeline fix sebelum informasi kerentanan dipublikasikan. Kami menargetkan window **90 hari** antara laporan dan disclosure publik, kecuali ada eksploitasi aktif.

---

## 9. Panduan Deployment Aman

### Key Management

**Wajib:**
- Master key TIDAK BOLEH disimpan dalam plaintext di disk atau environment variables tanpa proteksi
- Gunakan secret management service (HashiCorp Vault, AWS Secrets Manager, GCP Secret Manager, atau HSM)
- Rotasi kunci secara berkala; QRD mendukung re-encryption dengan kunci baru
- Backup kunci ke lokasi yang terpisah secara fisik dan logis dari data

**Disarankan:**
- Gunakan Hardware Security Module (HSM) untuk operasi kriptografis di lingkungan produksi
- Implementasikan key hierarchy: master key (HSM) → file key (per file) → column key (per column, via HKDF)
- Audit semua akses ke kunci di log yang terpisah dari application log

### Kolom Mana yang Dienkripsi?

Encrypt kolom berdasarkan sensitivitas data, bukan semua kolom:

```
Selalu enkripsi:      PII (nama, email, NIK), health data, biometrik,
                      lokasi GPS, financial data

Pertimbangkan:        Device ID (jika linkable ke individu), timestamp
                      (jika inferrable ke behavior)

Aman sebagai         Tipe sensor non-identifiable, aggregated metrics,
plaintext:           metadata teknis (versi firmware, model device)
```

Mengenkripsi semua kolom mencegah server-side operations yang berguna (sorting, filtering, aggregation pada kolom non-sensitif). Pilih dengan bijak.

### Verifikasi Integritas

Untuk sistem kritcal, verifikasi integritas file setelah write dan sebelum read penting:

```rust
// Setelah write
let mut verify_reader = FileReader::open(BufReader::new(File::open("output.qrd")?))?;
verify_reader.verify_integrity()?;

// Hasilkan integrity report
match verify_reader.verify_integrity() {
    Ok(report) => {
        assert!(report.crc_ok, "CRC integrity check failed");
        assert_eq!(report.auth_tags_valid, report.auth_tags_total,
                   "Authentication failed for {} chunks",
                   report.auth_tags_total - report.auth_tags_valid);
    }
    Err(e) => panic!("File corruption detected: {}", e),
}
```

### Row Group Size Tuning untuk Keamanan

Row group size mempengaruhi granularitas ECC recovery:

```
row_group_size terlalu kecil:  overhead ECC tinggi, I/O banyak
row_group_size terlalu besar:  satu corruption bisa mempengaruhi lebih banyak data

Rekomendasi untuk data kritis:
  ECC aktif dengan RS(16, 4): toleran 4 dari 16 chunks korup per row group
  Row group size: 10,000 – 50,000 rows
```

### Checklist Deployment Produksi

- [ ] Master key disimpan di secret management service atau HSM
- [ ] ECC diaktifkan untuk data yang disimpan jangka panjang
- [ ] `STATS_ENCRYPTED = 1` untuk kolom yang statistiknya sensitif
- [ ] Schema signature (Ed25519) diaktifkan untuk sistem yang memerlukan non-repudiation
- [ ] `verify_integrity()` dijalankan setelah write untuk data kritis
- [ ] Kolom yang dienkripsi sudah sesuai dengan kebijakan privasi organisasi
- [ ] Key rotation procedure sudah didokumentasikan dan ditest
- [ ] Log akses kunci terpisah dari application log
- [ ] Dependency audit (`cargo audit`) dijalankan di CI pipeline

---

*QRD-SDK Security Reference v1.0*  
*Untuk pertanyaan keamanan: [security@qrd.dev](mailto:security@qrd.dev)*  
*Untuk kerentanan: lihat bagian [Responsible Disclosure](#8-responsible-disclosure)*
