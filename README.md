<p align="center">
  <img src="docs/logotipo.png" alt="AnvilDB" width="648">
</p>

<p align="center">
  <a href="LICENSE"><img src="https://img.shields.io/badge/License-MIT-blue.svg" alt="License: MIT"></a>
  <a href="https://www.php.net/"><img src="https://img.shields.io/badge/PHP-%3E%3D%208.1-8892BF.svg" alt="PHP Version"></a>
  <a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/Rust-stable-orange.svg" alt="Rust"></a>
  <a href="https://github.com/Kevinsillo/anvildb/actions/workflows/tests.yml"><img src="https://github.com/Kevinsillo/anvildb/actions/workflows/tests.yml/badge.svg" alt="Tests"></a>
</p>

<p align="center"><strong>Embedded JSON document database powered by a Rust core, with language wrappers via FFI.</strong></p>

<p align="center">Zero external dependencies. No MySQL, PostgreSQL, SQLite, or PDO required. Just your filesystem and raw speed.</p>

---

## Motivation

AnvilDB was born out of a real need: working in environments where you have no installation permissions on the operating system. Without being able to install MySQL, PostgreSQL, or additional extensions, I needed a lightweight database with zero external dependencies that could run on the filesystem alone.

The goal was to have something fast for scaffolding projects and prototyping ideas without friction — an implementation that is often temporary, but needs to work from the very first moment with no configuration or infrastructure. Just copy, use, and move on.

## Features

- **Rust-powered core** compiled as a native shared library (`.so` / `.dylib` / `.dll`)
- **Joins** — INNER and LEFT joins across collections via hash join (O(n+m))
- **Write buffering** — batched disk writes with configurable threshold and timer
- **Compression** — all data compressed on disk (deflate), transparent to the API
- **Encryption at rest** — optional AES-256-GCM, per-file nonce, key as hex string
- **Lazy loading** — collections loaded on first access, not at startup
- **In-memory indexes** (hash, unique, and range/BTreeMap) for sub-millisecond lookups
- **Aggregations** — sum, avg, min, max, count with optional group_by
- **CSV export/import** for data portability
- **LRU cache** with automatic invalidation on writes
- **Atomic writes** via temp file + rename to prevent corruption
- **Schema validation** to enforce document structure
- **Bulk operations** for efficient batch inserts
- **Cross-platform** precompiled binaries (Linux, macOS, Windows / x86_64, aarch64)

## Wrappers

The core exposes a C API (`anvildb.h`) — any language with FFI support can integrate.

| Language | Package | Status |
|----------|---------|--------|
| PHP | [anvildb-php](https://github.com/kevinsillo/anvildb-php) | Available |

Want to create a wrapper for another language? See the [Wrapper Development Guide](docs/wrapper-development.md).

## Architecture

```
Application
    |
    v
Language Wrapper (PHP, Python, etc.)
    | JSON strings + opaque handle
    v
Rust Core Engine (libanvildb.so)
    |  - Write Buffer (dirty tracking + batched flush)
    |  - LRU Cache (auto-invalidated)
    |  - In-memory Indexes (Hash / Unique / Range)
    |  - Query Engine (filter, join, sort, paginate)
    |  - Schema Validation
    |  - Codec (deflate compression + optional AES-256-GCM)
    |  - Atomic Storage (temp file + rename)
    v
Filesystem (.anvil compressed + metadata.json)
```

## Performance

10,000 records | PHP 8.4 | Linux x86_64

| Operation | Time | Throughput |
|---|---:|---|
| Bulk insert (10x1000) | 204ms | ~49k docs/s |
| Read all (10k docs) | 22ms | ~454k docs/s |
| Filter query | 4.6ms | — |
| Filter + sort + limit | 3.7ms | — |
| Count with filter | 0.2ms | — |

With compression, encryption, atomic writes, and schema validation active.

Full benchmark history: [BENCHMARKS.md](BENCHMARKS.md)

## Project Structure

```
anvildb/
├── core/                # Rust core engine
│   ├── src/             #   Engine, FFI, storage, query, indexing
│   └── tests/           #   Rust integration tests
├── wrappers/php/        # PHP FFI wrapper
│   ├── src/             #   AnvilDb, Collection, Query, FFI, Exception
│   ├── tests/           #   PHPUnit tests
│   └── docs/            #   PHP-specific documentation
├── docs/                # Core documentation
├── benchmarks/          # Benchmark scripts
└── Cargo.toml           # Workspace root
```

## Documentation

Full documentation at [docs/index.md](docs/index.md) — development, architecture, C API reference, error codes, testing, CI/CD, wrapper development.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## Acknowledgments

AnvilDB is built on top of these excellent open-source projects:

| Crate | Description | License |
|-------|-------------|---------|
| [serde](https://github.com/serde-rs/serde) / [serde_json](https://github.com/serde-rs/json) | Serialization framework & JSON support | MIT / Apache-2.0 |
| [uuid](https://github.com/uuid-rs/uuid) | UUID generation (v4) | MIT / Apache-2.0 |
| [miniz_oxide](https://github.com/Frommi/miniz_oxide) | Pure Rust deflate compression | MIT / Apache-2.0 |
| [aes-gcm](https://github.com/RustCrypto/AEADs) | AES-256-GCM authenticated encryption | MIT / Apache-2.0 |
| [log](https://github.com/rust-lang/log) | Logging facade | MIT / Apache-2.0 |
| [env_logger](https://github.com/rust-cli/env_logger) | Log output to stderr (optional, dev only) | MIT / Apache-2.0 |
| [getrandom](https://github.com/rust-random/getrandom) | OS-level random number generation | MIT / Apache-2.0 |

## License

[MIT](LICENCE)
