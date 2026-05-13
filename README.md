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

</div>

---

## Apa itu QRD?

**QRD** (Columnar Row Descriptor) adalah **format binary container kolumnar** yang dirancang dengan **privacy sebagai properti inti format**, bukan fitur tambahan. QRD dibangun untuk analytical workloads di lingkungan **edge, browser, dan offline** — di mana data sensitif bergerak melintasi batas kepercayaan yang tidak dapat diasumsikan aman.

```
QRD adalah encrypted columnar container layer untuk sistem yang membutuhkan:

  ✓ Enkripsi end-to-end sebagai properti format, bukan infrastruktur tambahan
  ✓ Zero-knowledge storage: server tidak dapat membaca konten tanpa kunci
  ✓ Streaming ingestion dari edge ke cloud dengan bounded memory
  ✓ Analytical columnar reads di browser via WASM tanpa dekripsi server-side
  ✓ Deterministic binary output lintas bahasa dan platform
  ✓ Kepercayaan yang dapat diverifikasi secara kriptografis
```

> **Scope yang jelas:** QRD mengisi niche spesifik — privacy-native encrypted columnar streaming. QRD **bukan** pengganti Parquet untuk warehouse analytics, SQLite untuk OLTP, atau Arrow IPC untuk in-process data sharing.

---

## Masalah yang Dipecahkan

Format yang tersedia saat ini memiliki gap kritis untuk privacy-native edge pipelines:

| Masalah pada Format Lain | Solusi QRD |
|---|---|
| Enkripsi adalah plugin eksternal, bukan bagian format | Enkripsi per-kolom native di level format binary |
| Server harus dapat membaca data untuk deduplikasi dan indexing | Zero-knowledge: server hanya menyimpan ciphertext |
| Parquet membutuhkan buffer dataset penuh di memori | Row-group streaming dengan bounded memory |
| Format lain tidak support WASM atau browser | First-class WASM dan browser target |
| Multiple implementasi menyebabkan inkonsistensi lintas bahasa | Satu Rust engine, semua bahasa via FFI/WASM |
| Tidak ada error correction untuk storage yang terdegradasi | Reed-Solomon ECC parity chunks |

---

## Kapan Menggunakan QRD

| ✅ QRD adalah pilihan tepat | ❌ Gunakan format lain |
|---|---|
| Sensor telemetry dengan data sensitif (health, location, biometrics) | Data warehouse analytics tanpa persyaratan privasi → **Parquet** |
| Cross-boundary data transfer dengan zero-trust requirements | In-process data sharing dalam trust boundary yang sama → **Arrow IPC** |
| Browser-native analytics tanpa data meninggalkan perangkat | General-purpose relational database → **SQLite / DuckDB** |
| Audit logs dengan integritas kriptografis terverifikasi | Bulk ETL tanpa enkripsi requirement → **Parquet / CSV** |
| Edge AI inference di perangkat dengan RAM terbatas | Real-time OLTP workloads → **database relasional** |

---

## Keunggulan Kompetitif

| Properti | **QRD** | Parquet | Arrow IPC | SQLite |
|---|---|---|---|---|
| Enkripsi sebagai properti format | **Native** | Eksternal | Eksternal | Optional plugin |
| Zero-knowledge server storage | **Ya, by design** | Tidak | Tidak | Tidak |
| Streaming write dengan bounded memory | **Native** | Butuh buffer | Tidak dirancang | Terbatas |
| Browser / WASM first-class | **Ya** | Terbatas | Arrow JS | Tidak |
| Enkripsi granular per-kolom | **Ya** | Tidak | Tidak | Database-level |
| Error correction (Reed-Solomon) | **Ya** | Tidak | Tidak | Tidak |
| Single engine lintas bahasa | **Ya** | Implementasi ganda | Ref impl | Ya |

---

## Quick Start

### Instalasi

**Rust**
```toml
# Cargo.toml
[dependencies]
qrd-core = "1.0"
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

**Java (Maven)**
```xml
<dependency>
  <groupId>io.qrd</groupId>
  <artifactId>qrd-core</artifactId>
  <version>1.0.0</version>
</dependency>
```

### Contoh Dasar — Streaming Write dengan Enkripsi (Rust)

```rust
use qrd_core::{Schema, SchemaField, LogicalType, Nullability,
               StreamingWriter, WriterConfig, Compression, MasterKey};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let schema = Schema::builder()
        .field(SchemaField::new("device_id",  LogicalType::ENUM,      Nullability::Required))
        .field(SchemaField::new("timestamp",  LogicalType::TIMESTAMP, Nullability::Required))
        .field(SchemaField::new("health_val", LogicalType::FLOAT32,   Nullability::Optional))
        .build()?;

    // Master key dipegang client — tidak pernah dikirim ke server
    let master_key = MasterKey::from_env("QRD_MASTER_KEY")?;

    let config = WriterConfig::builder()
        .row_group_size(50_000)
        .compression(Compression::Zstd { level: 3 })
        .encrypt_columns(&["health_val"], &master_key)  // hanya kolom sensitif
        .ecc(true)
        .build()?;

    let mut writer = StreamingWriter::new(
        std::fs::File::create("telemetry.qrd")?, schema, config)?;

    for record in sensor_stream() {
        writer.write_row(vec![
            Value::Enum(record.device_id),
            Value::Timestamp(record.ts_micros),
            Value::Float32(record.health),
        ])?;
    }

    writer.finish()?; // WAJIB — menulis footer dan memfinalisasi file
    Ok(())
}
```

### Contoh Dasar — Partial Column Read (Python)

```python
import qrd, os

master_key = qrd.MasterKey.from_env("QRD_MASTER_KEY")
reader = qrd.FileReader("telemetry.qrd", master_key=master_key)

# Hanya baca kolom yang dibutuhkan — kolom lain tidak didekripsi
columns = reader.read_columns(["device_id", "health_val"])
print(f"Total rows: {reader.row_count()}")
```

---

## Multi-Language SDK

Semua SDK menggunakan **Rust core engine yang sama** via FFI atau WASM. Tidak ada implementasi mandiri dalam bahasa lain — ini adalah jaminan fidelitas format.

| Language | Mekanisme | Package | Status |
|---|---|---|---|
| **Rust** | Native | `qrd-core` (crates.io) | Stable / Reference |
| **Python** | PyO3 | `qrd-sdk` (PyPI) | Stable |
| **TypeScript** | WASM | `qrd-sdk` (npm) | Stable |
| **Go** | CGO | `github.com/zenipara/QRD-SDK/sdk/go` | Stable |
| **Java** | JNI | Maven `io.qrd:qrd-core` | Stable |
| **C/C++** | C FFI | Header + static lib | Stable |

---

## Use Cases Utama

**Edge & IoT Telemetry** — Sensor health, GPS, biometrik ditulis di perangkat dengan bounded memory, kolom sensitif dienkripsi sebelum upload. Server hanya menyimpan ciphertext.

**Browser Analytics** — Data analitik diproses sepenuhnya di browser via WASM. Data tidak pernah meninggalkan perangkat dalam bentuk plaintext.

**Audit & Compliance Logging** — Setiap event memiliki CRC32 integrity check. Schema dapat ditandatangani dengan Ed25519 untuk non-repudiation. Format self-describing tanpa registry eksternal.

**Cross-Language Data Pipeline** — File QRD yang ditulis oleh Rust dapat dibaca secara identik oleh Python, Go, TypeScript, atau Java. Tidak ada serialization drift lintas bahasa.

---

## Dokumentasi

| Dokumen | Deskripsi |
|---|---|
| [`README.md`](README.md) | Dokumen ini — overview dan quick start |
| [`SPECIFICATION.md`](SPECIFICATION.md) | Binary format specification (normative) |
| [`ARCHITECTURE.md`](ARCHITECTURE.md) | System design, component overview, data flow |
| [`SECURITY.md`](SECURITY.md) | Threat model, kriptografi, audit, disclosure policy |
| [`ROADMAP.md`](ROADMAP.md) | Phase-based evolution plan dengan exit criteria |
| [`docs/`](docs/) | Dokumentasi detail per komponen |

---

## Lisensi

QRD-SDK dilisensikan di bawah **Business Source License 1.1 (BSL-1.1)**.

- **Source-available**: kode dapat dibaca, dipelajari, dan dimodifikasi
- **Penggunaan produksi gratis** untuk use case non-kompetitif dengan QRD core
- **Setelah 4 tahun**, lisensi berkonversi otomatis ke **Apache 2.0**

Penggunaan internal, riset, pendidikan, dan integrasi dalam produk yang bukan "managed QRD container service" adalah bebas tanpa restriction.

---

<div align="center">

**QRD-SDK** — Privacy-native encrypted columnar container untuk sistem yang tidak dapat berasumsi bahwa server, network, atau storage layer dapat dipercaya.

*Data Anda. Kunci Anda. Format Anda.*

[![GitHub](https://img.shields.io/badge/GitHub-zenipara%2FQRD--SDK-black?logo=github)](https://github.com/zenipara/QRD-SDK)
[![Documentation](https://img.shields.io/badge/Documentation-docs.qrd.dev-brightgreen)](https://docs.qrd.dev)
[![Security](https://img.shields.io/badge/Security-security%40qrd.dev-red)](mailto:security@qrd.dev)

*Built with Rust · BSL-1.1 → Apache 2.0 · Security-first design*

</div>
