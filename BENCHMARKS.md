# Benchmarks

Performance evolution of AnvilDB compared to pure PHP (`json_encode` + `file_put_contents`).

Run benchmarks yourself:

```bash
php benchmarks/benchmark.php 10000
```

---

## v0.2.0 — NDJSON append-only storage

> 2026-03-15 | PHP 8.4.18 | Rust stable | Linux x86_64

| Operation | AnvilDB (ms) | Pure PHP (ms) | Winner | vs v0.1.0 |
|---|---:|---:|---|---|
| Bulk insert (10x1000) | 221.8 | 7.8 | PHP | **4.9x faster** |
| Read all | 124.6 | 8.6 | PHP | ~same |
| Filter (`=` admin) | 33.9 | 0.6 | PHP | ~same |
| Filter + sort + limit | 15.6 | 5.9 | PHP | ~same |
| Count with filter | 1.8 | 0.6 | PHP | ~same |

**10,000 records**

### Changes

- Storage migrated from JSON array (`[{...},{...}]`) to NDJSON (one JSON object per line)
- Inserts now use `append` mode — O(1) per document instead of O(n) full rewrite
- Updates and deletes still rewrite the file (expected — append-only can't modify in place)
- Automatic migration from legacy JSON array format on first open

### Impact

- **Bulk insert 4.9x faster** (1085ms → 222ms)
- Read/query performance unchanged (expected — reads still parse full file)

---

## v0.1.0 — JSON array storage + atomic read-modify-write

> 2026-03-15 | PHP 8.4.18 | Rust stable | Linux x86_64

| Operation | AnvilDB (ms) | Pure PHP (ms) | Winner |
|---|---:|---:|---|
| Bulk insert (10x1000) | 1085.3 | 7.8 | PHP |
| Read all | 123.3 | 8.6 | PHP |
| Filter (`=` admin) | 32.5 | 0.6 | PHP |
| Filter + sort + limit | 13.7 | 4.7 | PHP |
| Count with filter | 1.5 | 0.4 | PHP |

**10,000 records**

### Notes

- AnvilDB uses atomic read-modify-write (exclusive file lock + temp file + rename) on every write operation, which re-reads and re-writes the entire JSON array per batch. This guarantees multi-process safety at the cost of write throughput.
- Pure PHP does a single `file_put_contents` with no locking, no atomicity, and no corruption protection. It is the fastest possible but unsafe for concurrent access.
- Write performance is the main bottleneck and will improve significantly with NDJSON append-only storage (planned).
