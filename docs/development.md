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

PHP and Rust communicate through a C-compatible API defined in `src/FFI/anvildb.h`. The Rust side exposes `extern "C"` functions in `rust/src/lib.rs`.

Data crosses the boundary as:
- **JSON strings** for documents and query results
- **Opaque pointer** (`void*`) for the engine handle
- **`int32_t`/`int64_t`** for status codes and counts

### Memory Management

Rust-allocated strings returned to PHP must be freed with `anvildb_free_string()`. The PHP `Bridge.php` handles this automatically. On PHP 8.4+, FFI may return native PHP strings instead of `CData` pointers — the wrapper handles both cases.

### Engine Lifecycle

1. `anvildb_open(path)` — creates an `Engine` instance, boxed and leaked as `*mut Engine`
2. All operations receive the engine handle
3. `anvildb_close(handle)` — reconstructs the `Box<Engine>` and drops it
4. `anvildb_shutdown(handle)` — flushes all write buffers before close

### Concurrency

- **File level**: `flock` (shared for reads, exclusive for writes) via `fs2` crate
- **Process level**: `RwLock` around the collections map protects in-memory state

### Storage

Collections are stored as JSON arrays in `data/collections/{name}.json`. Writes use atomic temp-file-and-rename to prevent corruption.

### Indexes

- **Hash**: `HashMap<String, Vec<usize>>` — equality lookups
- **Unique**: `HashMap<String, usize>` — equality with uniqueness enforcement

Indexes are persisted to `data/indexes/{collection}_{field}.idx.json` and loaded into memory on engine start.

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
2. Implement the `extern "C"` function in `rust/src/lib.rs`
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
