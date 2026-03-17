# Development Guide

[< Back to index](index.md)

## Prerequisites

- PHP >= 8.1 with FFI extension
- Rust toolchain (stable) — install from https://rustup.rs
- Composer

## Setup

```bash
git clone https://github.com/Kevinsillo/anvildb.git
cd anvildb
cargo build
cd wrappers/php && composer install
```

## Project Layout

| Directory | Language | Purpose |
|-----------|----------|---------|
| `core/src/` | Rust | Core engine — storage, indexing, queries, cache, validation |
| `core/tests/` | Rust | Rust integration tests |
| `wrappers/php/src/` | PHP | FFI wrapper — fluent API for PHP consumers |
| `wrappers/php/tests/` | PHP | PHPUnit integration tests |
| `benchmarks/` | PHP | Benchmark scripts |
| `docs/` | — | Documentation |
| `scripts/` | PHP | Manual test scripts |

This is a **monorepo** — the Rust core and all language wrappers live in the same repository. Each wrapper is published independently via subtree split (see [CI/CD](ci-cd.md)).

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

### Debug Logs

By default, the release build has no logging output. To enable Rust log messages (useful for debugging FFI calls), build with the `debug-logs` feature:

```bash
cargo build --features debug-logs
```

Then run your PHP script with:

```bash
RUST_LOG=debug php your_script.php
```

Log levels: `error`, `warn`, `info`, `debug`, `trace`.

### Custom Library Path

Set the `ANVILDB_LIB_PATH` environment variable to override auto-detection:

```bash
export ANVILDB_LIB_PATH=/path/to/libanvildb.so
```

## Adding New FFI Functions

1. Add the function signature to `wrappers/php/src/FFI/anvildb.h`
2. Implement the `extern "C"` function in `core/src/ffi.rs`
3. Add the business logic in the appropriate Rust module
4. Expose via the PHP wrapper classes
5. Add tests in both Rust and PHP

## Next

- [Architecture](architecture.md) — how the engine works internally
- [Testing](testing.md) — running and writing tests
- [CI/CD](ci-cd.md) — workflows, releases, subtree split
