# Benchmarks

Performance evolution of AnvilDB across versions.

Run benchmarks yourself:

```bash
php benchmarks/benchmark.php 10000
```

---

## v0.3.0 — Compression, encryption, joins, lazy loading

> 2026-03-16 | PHP 8.4.18 | Rust stable | Linux x86_64 | 10,000 records

| Operation | Time | Throughput | vs v0.2.0 |
|---|---:|---|---|
| Bulk insert (10x1000) | 199ms | ~50k docs/s | ~same |
| Read all (10k docs) | 23ms | ~441k docs/s | **5.4x faster** |
| Filter (`=` admin) | 4.3ms | — | **7.9x faster** |
| Filter + sort + limit | 3.4ms | — | **4.6x faster** |
| Count with filter | 0.2ms | — | **9x faster** |

All operations include compression, atomic writes, schema validation, and index enforcement.

### Changes

- Deflate compression on all collection and index files (`.anvil` format)
- Optional AES-256-GCM encryption at rest
- INNER and LEFT joins across collections (hash join)
- Lazy loading — collections loaded on first access, not at startup
- Write buffer simplified to dirty-set tracking with full rewrite on flush
- FFI functions moved from `lib.rs` to `ffi.rs`

### Impact

- **Read/query performance dramatically improved** — lazy loading + compressed format means less I/O
- Insert throughput maintained despite switching from append to full rewrite (compression reduces file size)

---

## v0.2.0 — NDJSON append-only storage

> 2026-03-15 | PHP 8.4.18 | Rust stable | Linux x86_64 | 10,000 records

| Operation | Time | Throughput | vs v0.1.0 |
|---|---:|---|---|
| Bulk insert (10x1000) | 222ms | ~45k docs/s | **4.9x faster** |
| Read all (10k docs) | 125ms | ~80k docs/s | ~same |
| Filter (`=` admin) | 34ms | — | ~same |
| Filter + sort + limit | 16ms | — | ~same |
| Count with filter | 1.8ms | — | ~same |

All operations include atomic writes, compression, schema validation, and index enforcement.

### Changes

- Storage migrated from JSON array to NDJSON (one JSON object per line)
- Inserts use `append` mode — O(1) per document instead of O(n) full rewrite
- Automatic migration from legacy format on first open

### Impact

- **Bulk insert 4.9x faster** (1085ms → 222ms)
- Read/query performance unchanged (expected — reads still parse full file)

---

## v0.1.0 — JSON array storage + atomic read-modify-write

> 2026-03-15 | PHP 8.4.18 | Rust stable | Linux x86_64 | 10,000 records

| Operation | Time | Throughput |
|---|---:|---|
| Bulk insert (10x1000) | 1085ms | ~9k docs/s |
| Read all (10k docs) | 123ms | ~81k docs/s |
| Filter (`=` admin) | 33ms | — |
| Filter + sort + limit | 14ms | — |
| Count with filter | 1.5ms | — |

### Notes

- Every write operation uses atomic read-modify-write (exclusive file lock + temp file + rename), re-reading and re-writing the entire JSON array per batch
- This guarantees multi-process safety at the cost of write throughput
- Write performance was the main bottleneck, addressed in v0.2.0 with NDJSON
