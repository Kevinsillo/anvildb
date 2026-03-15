# AnvilDB

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![PHP Version](https://img.shields.io/badge/PHP-%3E%3D%208.1-8892BF.svg)](https://www.php.net/)
[![Rust](https://img.shields.io/badge/Rust-stable-orange.svg)](https://www.rust-lang.org/)
[![Tests](https://github.com/Kevinsillo/anvildb/actions/workflows/tests.yml/badge.svg)](https://github.com/Kevinsillo/anvildb/actions/workflows/tests.yml)

**Embedded JSON document database for PHP, powered by a Rust core via FFI.**

Zero external dependencies. No MySQL, PostgreSQL, SQLite, or PDO required. Just your filesystem and raw speed.

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
- **Fluent PHP API** with query builder, filters, sorting, and pagination
- **In-memory indexes** (hash and unique) for sub-millisecond lookups
- **LRU cache** with automatic invalidation on writes
- **Atomic writes** via temp file + rename to prevent corruption
- **Schema validation** to enforce document structure
- **Bulk operations** for efficient batch inserts
- **Cross-platform** precompiled binaries (Linux, macOS, Windows / x86_64, aarch64)

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

$db = new AnvilDb(__DIR__ . '/data');

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

Fluent query builder with filters, sorting, and pagination — executed as a **single FFI call**:

```php
$results = $db->collection('users')
    ->where('role', '=', 'admin')
    ->where('age', '>', 25)
    ->orderBy('name', 'asc')
    ->limit(10)
    ->offset(20)
    ->get();
```

Supported operators: `=`, `!=`, `>`, `<`, `>=`, `<=`, `contains`.

```php
// Count with filters
$count = $db->collection('users')
    ->where('role', '=', 'admin')
    ->count();

// Get all documents
$all = $db->collection('users')->all();

// Bulk insert
$docs = $db->collection('users')->bulkInsert([
    ['name' => 'Alice', 'role' => 'user'],
    ['name' => 'Bob',   'role' => 'admin'],
]);
```

## Indexes

Create indexes for faster queries:

```php
// Hash index — fast equality lookups
$db->collection('users')->createIndex('role', 'hash');

// Unique index — enforces uniqueness
$db->collection('users')->createIndex('email', 'unique');

// Drop index
$db->collection('users')->dropIndex('role');
```

## Schema Validation

Define schemas to enforce document structure at insert/update time:

```php
$db->collection('users')->setSchema([
    'name'   => 'string',
    'age'    => 'int',
    'active' => 'bool',
]);

// This will throw AnvilDbException — name must be string
$db->collection('users')->insert(['name' => 123]);
```

Supported types: `string`, `int`, `float`, `bool`, `array`, `object`.

## Collections Management

```php
$db->createCollection('orders');
$db->dropCollection('orders');
$collections = $db->listCollections(); // ['orders', 'users']
```

## Architecture

```
PHP Application
    |
    v
PHP FFI Wrapper (fluent API)
    | JSON strings + opaque handle
    v
Rust Core Engine (libanvildb.so)
    |  - LRU Cache (auto-invalidated)
    |  - In-memory Indexes (Hash / Unique)
    |  - Query Engine (filter, sort, paginate)
    |  - Schema Validation
    |  - Atomic Storage (temp file + rename)
    v
Filesystem (JSON collections + index files)
```

The Rust core handles all heavy operations:

- **Storage**: Atomic writes (temp file + rename), file locking via `flock`
- **Indexes**: In-memory `HashMap`/`BTreeMap`, persisted to disk
- **Cache**: LRU cache in Rust heap, auto-invalidated on writes
- **Queries**: Filter, sort, limit, offset — single FFI call per query
- **Validation**: Schema enforcement before insert/update
- **Serialization**: `serde_json` (orders of magnitude faster than PHP's `json_encode`/`json_decode`)

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
# Rust tests (14 tests)
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
