# Testing Guide

## Overview

The project has two test suites:

| Suite | Language | Tool | Location |
|-------|----------|------|----------|
| Rust integration tests | Rust | `cargo test` | `rust/tests/integration_test.rs` |
| PHP integration tests | PHP | PHPUnit | `tests/Integration/AnvilDbTest.php` |

## Running Tests

### All Tests

```bash
cargo test && ./vendor/bin/phpunit
```

### Rust Tests Only

```bash
cargo test
```

51 tests covering:
- Engine lifecycle (open, close, reopen)
- Collection CRUD (create, drop, insert, find, update, delete)
- Query engine (all filter operators, sort, limit, offset)
- Joins (inner, left, multiple, custom prefix, filters/sort/pagination on joined results, error cases)
- Write buffering (visibility before flush, threshold auto-flush, manual flush, shutdown flush, interaction with update/delete)
- Lazy loading (list without loading, partial loading, lazy join)
- Compression (transparent, file format verification)
- Encryption (encrypted DB, encrypt/decrypt existing, wrong key, key required)
- New operators (between, in, not_in, regex)
- Aggregations (sum, avg, min, max, count, group_by with filters)
- Range indexes (create, persist, drop)
- Index operations (hash, unique, duplicate rejection)
- Schema validation (valid/invalid documents)
- Bulk insert
- List collections
- Count with filters

### PHP Tests Only

```bash
./vendor/bin/phpunit
```

22 tests covering:
- Full CRUD flow through FFI
- Query builder (where, orderBy, limit, offset, combined)
- Bulk insert
- Collection management (list, drop)
- Count operations
- Index create/drop
- Schema validation (accept/reject)
- Connection lifecycle

## Writing New Tests

### Rust

Add test functions to `rust/tests/integration_test.rs` or create new test files in `rust/tests/`.

```rust
#[test]
fn test_my_feature() {
    let dir = tempdir().unwrap();
    let engine = Engine::open(dir.path().to_str().unwrap(), None).unwrap();

    // test logic...
}
```

### PHP

Add test methods to `tests/Integration/AnvilDbTest.php` or create new test classes in `tests/`.

```php
public function testMyFeature(): void
{
    $this->db->createCollection('test');
    $collection = $this->db->collection('test');

    // test logic...

    $this->assertEquals($expected, $actual);
}
```

## Test Data

Both test suites use temporary directories created in `setUp`/test setup and cleaned up after each test. No persistent test data is needed.

## Environment

- The `.so` must be built before running PHP tests (`cargo build`)
- PHP FFI extension must be enabled (`ffi.enable=true`)
- No database or external service required
