<p align="center">
  <img src="docs/logotipo.png" alt="AnvilDB" width="300">
</p>

<p align="center">
  <a href="LICENSE"><img src="https://img.shields.io/badge/License-MIT-blue.svg" alt="License: MIT"></a>
  <a href="https://www.php.net/"><img src="https://img.shields.io/badge/PHP-%3E%3D%208.1-8892BF.svg" alt="PHP Version"></a>
  <a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/Rust-stable-orange.svg" alt="Rust"></a>
  <a href="https://github.com/Kevinsillo/anvildb/actions/workflows/tests.yml"><img src="https://github.com/Kevinsillo/anvildb/actions/workflows/tests.yml/badge.svg" alt="Tests"></a>
</p>

<p align="center"><strong>Embedded JSON document database for PHP, powered by a Rust core via FFI.</strong></p>

<p align="center">Zero external dependencies. No MySQL, PostgreSQL, SQLite, or PDO required. Just your filesystem and raw speed.</p>

---

## Why AnvilDB?

| | AnvilDB | PHP `json_encode` | SQLite |
|---|---|---|---|
| External dependencies | None | None | ext-pdo + ext-sqlite3 |
| Serialization speed | `serde_json` (Rust) | Native PHP | C library |
| Concurrency safety | `flock` + `RwLock` | Manual | WAL mode |
| In-memory indexing | Hash + Unique | None | B-Tree |
| Query cache | LRU (auto-invalidated) | None | Page cache |
| Atomic writes | temp file + rename | Manual | Journal/WAL |
| Schema validation | Built-in | Manual | SQL constraints |

## Features

- **Rust-powered core** compiled as a native shared library (`.so` / `.dylib` / `.dll`)
- **Joins** — INNER and LEFT joins across collections via hash join (O(n+m))
- **Write buffering** — batched disk writes with configurable threshold and timer
- **Compression** — all data compressed on disk (deflate), transparent to the API
- **Encryption at rest** — optional AES-256-GCM, per-file nonce, key as hex string
- **Lazy loading** — collections loaded on first access, not at startup
- **In-memory indexes** (hash and unique) for sub-millisecond lookups
- **LRU cache** with automatic invalidation on writes
- **Atomic writes** via temp file + rename to prevent corruption
- **Schema validation** to enforce document structure
- **Bulk operations** for efficient batch inserts
- **Cross-platform** precompiled binaries (Linux, macOS, Windows / x86_64, aarch64)

## Wrappers

| Language | Status | Package |
|----------|--------|---------|
| PHP | Included | Fluent API, query builder, filters, sorting, pagination |

More wrappers coming soon. The core exposes a C API (`anvildb.h`) — any language with FFI support can integrate.

## Requirements

- PHP >= 8.1 with FFI extension enabled (`ffi.enable=true` in php.ini)
- Rust toolchain (only for building from source)

## Installation

```bash
composer require kevinsillo/anvildb
```

Build the native library:

```bash
cargo build --release
```

The shared library will be at `target/release/libanvildb.so`. The PHP wrapper auto-detects it.

## Quick Start

```php
<?php

use AnvilDb\AnvilDb;

// Open database (data is compressed on disk automatically)
$db = new AnvilDb(__DIR__ . '/data');

// Or with encryption (64-char hex key = 32 bytes AES-256)
// $db = new AnvilDb(__DIR__ . '/data', 'your-64-char-hex-key-here...');

// Create a collection
$db->createCollection('users');
$users = $db->collection('users');

// Insert — returns document with auto-generated UUID
$user = $users->insert([
    'name'  => 'Kevin',
    'role'  => 'admin',
    'age'   => 30,
]);

// Find by ID
$found = $users->find($user['id']);

// Update
$users->update($user['id'], [
    'name'  => 'Kevin',
    'role'  => 'admin',
    'age'   => 31,
]);

// Delete
$users->delete($user['id']);

// Close
$db->close();
```

## Queries

```php
$results = $db->collection('users')
    ->where('role', '=', 'admin')
    ->where('age', '>', 25)
    ->orderBy('name', 'asc')
    ->limit(10)
    ->offset(20)
    ->get();
```

Operators: `=`, `!=`, `>`, `<`, `>=`, `<=`, `contains`.

## Joins

```php
// INNER JOIN
$results = $db->collection('orders')
    ->join('users', 'user_id', 'id', 'inner', 'user_')
    ->where('user_name', '=', 'Alice')
    ->orderBy('total', 'desc')
    ->get();

// LEFT JOIN
$results = $db->collection('users')
    ->leftJoin('orders', 'id', 'user_id', 'order_')
    ->get();

// Multiple joins
$results = $db->collection('order_items')
    ->join('orders', 'order_id', 'id', 'inner', 'order_')
    ->join('products', 'product_id', 'id', 'inner', 'product_')
    ->get();
```

Joined fields are prefixed to avoid collisions (e.g. `user_name`, `order_total`). Filters, sorting, and pagination apply after the join.

## Write Buffering

Inserts are buffered in memory and flushed to disk by threshold (default: 100 docs) or timer (default: 5s).

```php
$db->configureBuffer(maxDocs: 200, flushIntervalSecs: 10);
$db->flush();                        // manual flush (all collections)
$db->collection('logs')->flush();    // flush single collection
$db->shutdown();                     // flushes + closes
```

## Encryption

```php
// Create a new encrypted database (or open an existing one)
$db = new AnvilDb('/data', 'aabbccdd...64-char-hex-key...');

// Add encryption to an existing unencrypted database
$db->encrypt('aabbccdd...64-char-hex-key...');

// Remove encryption from an encrypted database
$db->decrypt('aabbccdd...64-char-hex-key...');
```

See [API Reference](docs/api-reference.md) for the full API (indexes, schemas, collections, etc.).

## Architecture

```
PHP Application
    |
    v
PHP FFI Wrapper (fluent API)
    | JSON strings + opaque handle
    v
Rust Core Engine (libanvildb.so)
    |  - Write Buffer (dirty tracking + batched flush)
    |  - LRU Cache (auto-invalidated)
    |  - In-memory Indexes (Hash / Unique)
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
| Bulk insert (10x1000) | 199ms | ~50k docs/s |
| Read all (10k docs) | 23ms | ~441k docs/s |
| Filter query | 4.3ms | — |
| Filter + sort + limit | 3.4ms | — |
| Count with filter | 0.2ms | — |

With compression, encryption, atomic writes, and schema validation active.

Full benchmark history: [BENCHMARKS.md](BENCHMARKS.md)

## Project Structure

```
anvildb/
├── rust/src/          # Rust core engine (compiled to .so)
├── src/               # PHP FFI wrapper
│   ├── AnvilDb.php    #   Main facade
│   ├── FFI/           #   Bridge + C header
│   ├── Collection/    #   Collection API
│   ├── Query/         #   Query builder
│   └── Exception/     #   Exception classes
├── tests/             # PHPUnit + Rust integration tests
├── data/              # Runtime data directory
└── lib/               # Precompiled binaries (per platform)
```

## Testing

```bash
# Rust tests (41 tests)
cargo test

# PHP tests (22 tests)
composer install
./vendor/bin/phpunit

# Both
cargo test && ./vendor/bin/phpunit
```

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License

[MIT](LICENSE)
