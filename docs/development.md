# Development Guide

## Prerequisites

- PHP >= 8.1 with FFI extension
- Rust toolchain (stable)
- Composer

## Setup

```bash
git clone https://github.com/Kevinsillo/anvildb.git
cd anvildb
composer install
cargo build
```

## Project Layout

| Directory | Language | Purpose |
|-----------|----------|---------|
| `rust/src/` | Rust | Core engine — storage, indexing, queries, cache, validation |
| `src/` | PHP | FFI wrapper — fluent API for PHP consumers |
| `tests/` | PHP | PHPUnit integration tests |
| `rust/tests/` | Rust | Rust integration tests |
| `lib/` | — | Precompiled `.so`/`.dylib`/`.dll` per platform |
| `data/` | — | Runtime data (collections, indexes) |

## How It Works

### FFI Boundary

PHP and Rust communicate through a C-compatible API defined in `src/FFI/anvildb.h`. The Rust side exposes `extern "C"` functions in `rust/src/ffi.rs`.

Data crosses the boundary as:
- **JSON strings** for documents and query results
- **Opaque pointer** (`void*`) for the engine handle
- **`int32_t`/`int64_t`** for status codes and counts

### Memory Management

Rust-allocated strings returned to PHP must be freed with `anvildb_free_string()`. The PHP `Bridge.php` handles this automatically. On PHP 8.4+, FFI may return native PHP strings instead of `CData` pointers — the wrapper handles both cases.

### Engine Lifecycle

1. `anvildb_open(path, key)` — creates an `Engine` instance, discovers collections (lazy), boxed as `*mut Engine`
2. All operations receive the engine handle
3. `anvildb_close(handle)` — reconstructs the `Box<Engine>` and drops it
4. `anvildb_shutdown(handle)` — flushes all write buffers before close (via `Drop`)

### Lazy Loading

Collections are discovered on `open()` but not loaded from disk. Each collection starts as `LazyCollection::Unloaded` and transitions to `LazyCollection::Loaded` on first access via `ensure_loaded()`. This uses a double-check locking pattern: read lock to check, write lock to load.

### Concurrency

- **Process level**: `RwLock` around the collections map protects in-memory state

### Storage

Collections are stored as compressed binary files in `data/collections/{name}.anvil`. The codec pipeline:
- **Write**: NDJSON bytes → deflate compress → (optional) AES-256-GCM encrypt → disk
- **Read**: disk → (optional) decrypt → decompress → parse NDJSON → `Vec<Value>`

All writes are full rewrites via atomic temp file + rename. A `metadata.json` in the DB root tracks the format version and encryption state.

### Write Buffer

The buffer (`rust/src/buffer.rs`) tracks which collections have pending (unflushed) writes as a dirty set. Documents are visible immediately in queries (they're in `Collection.documents`), but disk writes are batched:

- **Threshold flush**: when a collection's dirty count reaches `max_docs` (default 100), it's rewritten synchronously
- **Timer flush**: a background thread rewrites all dirty collections every `flush_interval_secs` (default 5s)
- **Drop/shutdown**: the `Drop` impl stops the thread and flushes remaining dirty collections
- **Update/delete**: clears the dirty flag (since the operation itself does a full rewrite)

### Compression + Encryption

The codec (`rust/src/storage/codec.rs`) handles all data encoding/decoding:
- **Compression**: always active, using `miniz_oxide` (pure Rust deflate). Reduces file sizes 5-10x for typical JSON data.
- **Encryption**: opt-in AES-256-GCM via `aes-gcm` (pure Rust). Each file gets a unique 12-byte random nonce prepended to the ciphertext. Key is a 32-byte value passed as 64-char hex string through the FFI boundary.

### Joins

The query engine (`rust/src/query/engine.rs`) supports INNER and LEFT joins via hash join:

1. Build a `HashMap` on the right collection's join field — O(m)
2. Probe each left document against the map — O(1) per doc
3. Merge matched documents with prefixed field names
4. Apply filters, sort, limit/offset on the merged result set

Multiple joins are applied sequentially (left-to-right).

### Indexes

- **Hash**: `HashMap<String, Vec<usize>>` — equality lookups
- **Unique**: `HashMap<String, usize>` — equality with uniqueness enforcement
- **Range**: `BTreeMap<String, Vec<usize>>` — ordered lookups (>, <, >=, <=, between)

Indexes are persisted to `data/indexes/{collection}_{field}.idx.anvil` (compressed, optionally encrypted) and loaded into memory on first access.

## Building

### Debug

```bash
cargo build
```

Output: `target/debug/libanvildb.so`

### Release

```bash
cargo build --release
```

Output: `target/release/libanvildb.so` (optimized with LTO, stripped)

### Custom Library Path

Set the `ANVILDB_LIB_PATH` environment variable to override auto-detection:

```bash
export ANVILDB_LIB_PATH=/path/to/libanvildb.so
```

## Adding New FFI Functions

1. Add the function signature to `src/FFI/anvildb.h`
2. Implement the `extern "C"` function in `rust/src/ffi.rs`
3. Add the business logic in the appropriate Rust module
4. Expose via the PHP wrapper classes
5. Add tests in both Rust and PHP

## CI/CD

### Continuous Integration

Every push to `main` and every pull request triggers the CI pipeline (`.github/workflows/tests.yml`):

1. **Rust tests** — `cargo test` on `ubuntu-latest`
2. **PHP tests** — `./vendor/bin/phpunit` on PHP 8.1, 8.2, 8.3, and 8.4

PHP tests depend on Rust tests passing first. The native library is built automatically during CI.

### Releasing a New Version

Releases are automated via `.github/workflows/release.yml`. To publish a new version:

1. Make sure `main` is stable — all tests passing, changes merged
2. Update the version in `rust/Cargo.toml` and `composer.json` if needed
3. Create and push a tag:
   ```bash
   git tag v0.2.0
   git push origin v0.2.0
   ```

That's it. The workflow automatically cross-compiles for all 5 platforms (`x86_64-linux`, `aarch64-linux`, `x86_64-darwin`, `aarch64-darwin`, `x86_64-windows`), packages the binaries, creates a GitHub Release with auto-generated notes, and attaches all archives.

### Versioning

Use [Semantic Versioning](https://semver.org/):

- **Patch** (`v0.1.1`): bug fixes, no API changes
- **Minor** (`v0.2.0`): new features, backwards compatible
- **Major** (`v1.0.0`): breaking API changes

### Workflow Summary

```
Feature/fix branch
    |
    v
Pull Request → CI runs tests (Rust + PHP 8.1–8.4)
    |
    v
Merge to main
    |
    v
git tag v0.x.x → push tag
    |
    v
Release workflow → cross-compile → GitHub Release with binaries
```
