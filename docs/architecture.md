# Architecture

[< Back to index](index.md)

## FFI Boundary

PHP and Rust communicate through a C-compatible API defined in `wrappers/php/src/FFI/anvildb.h`. The Rust side exposes `extern "C"` functions in `core/src/ffi.rs`.

Data crosses the boundary as:
- **JSON strings** for documents and query results
- **Opaque pointer** (`void*`) for the engine handle
- **`int32_t`/`int64_t`** for status codes and counts

See the full function list in the [C API Reference](c-api.md).

## Memory Management

Rust-allocated strings returned to PHP must be freed with `anvildb_free_string()`. The PHP `Bridge.php` handles this automatically. On PHP 8.4+, FFI may return native PHP strings instead of `CData` pointers — the wrapper handles both cases.

## Engine Lifecycle

1. `anvildb_open(path, key)` — creates an `Engine` instance, discovers collections (lazy), boxed as `*mut Engine`
2. All operations receive the engine handle
3. `anvildb_close(handle)` — reconstructs the `Box<Engine>` and drops it
4. `anvildb_shutdown(handle)` — flushes all write buffers before close (via `Drop`)

## Lazy Loading

Collections are discovered on `open()` but not loaded from disk. Each collection starts as `LazyCollection::Unloaded` and transitions to `LazyCollection::Loaded` on first access via `ensure_loaded()`. This uses a double-check locking pattern: read lock to check, write lock to load.

## Concurrency

- **Process level**: `RwLock` around the collections map protects in-memory state

## Storage

Collections are stored as compressed binary files in `data/collections/{name}.anvil`. The codec pipeline:
- **Write**: NDJSON bytes → deflate compress → (optional) AES-256-GCM encrypt → disk
- **Read**: disk → (optional) decrypt → decompress → parse NDJSON → `Vec<Value>`

All writes are full rewrites via atomic temp file + rename. A `metadata.json` in the DB root tracks the format version and encryption state.

## Write Buffer

The buffer (`core/src/buffer.rs`) tracks which collections have pending (unflushed) writes as a dirty set. Documents are visible immediately in queries (they're in `Collection.documents`), but disk writes are batched:

- **Threshold flush**: when a collection's dirty count reaches `max_docs` (default 100), it's rewritten synchronously
- **Timer flush**: a background thread rewrites all dirty collections every `flush_interval_secs` (default 5s)
- **Drop/shutdown**: the `Drop` impl stops the thread and flushes remaining dirty collections

## Compression + Encryption

The codec (`core/src/storage/codec.rs`) handles all data encoding/decoding:
- **Compression**: always active, using `miniz_oxide` (pure Rust deflate). Reduces file sizes 5-10x for typical JSON data.
- **Encryption**: opt-in AES-256-GCM via `aes-gcm` (pure Rust). Each file gets a unique 12-byte random nonce prepended to the ciphertext. Key is a 32-byte value passed as 64-char hex string through the FFI boundary.

## Joins

The query engine (`core/src/query/engine.rs`) supports INNER and LEFT joins via hash join:

1. Build a `HashMap` on the right collection's join field — O(m)
2. Probe each left document against the map — O(1) per doc
3. Merge matched documents with prefixed field names
4. Apply filters, sort, limit/offset on the merged result set

Multiple joins are applied sequentially (left-to-right).

## Indexes

- **Hash**: `HashMap<String, Vec<usize>>` — equality lookups
- **Unique**: `HashMap<String, usize>` — equality with uniqueness enforcement
- **Range**: `BTreeMap<String, Vec<usize>>` — ordered lookups (>, <, >=, <=, between)

Indexes are persisted to `data/indexes/{collection}_{field}.idx.anvil` (compressed, optionally encrypted) and loaded into memory on first access.
