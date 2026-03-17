# Error Codes

[< Back to index](index.md)

AnvilDB uses numeric error codes for programmatic error handling across FFI boundaries. Every function that can fail returns `-1` (for `int32_t`) or `NULL` (for `const char*`) on error. After detecting a failure, call `anvildb_last_error()` and `anvildb_last_error_code()` to retrieve the details.

> Both calls consume the stored error — a second call returns `NULL` / `0`.

## Error code ranges

| Range | Category | Description |
|-------|----------|-------------|
| `0` | No error | Operation succeeded |
| `-1` | Generic | FFI input validation failed (invalid pointer, NULL argument) |
| `1000` | IO | Filesystem read/write errors |
| `1100` | JSON | Serialization or deserialization failures |
| `1200–1201` | Collection | Collection-level errors |
| `1300–1301` | Document | Document-level errors |
| `1400` | Validation | Schema validation failures |
| `1500` | Query | Malformed query specification |
| `1600` | Lock | Internal concurrency errors |
| `1700–1702` | Encryption | Encryption and decryption errors |
| `1800` | Argument | Invalid argument passed to the engine |

## Error codes in detail

### IO errors — `1000`

| Code | Error | Message format | Triggered by |
|------|-------|----------------|--------------|
| `1000` | `Io` | `IO error: {details}` | Any function that reads/writes disk: `anvildb_open`, `anvildb_flush`, `anvildb_flush_collection`, `anvildb_create_collection`, `anvildb_drop_collection`, `anvildb_encrypt`, `anvildb_decrypt`, `anvildb_create_index`, `anvildb_insert`, `anvildb_bulk_insert` |

### JSON errors — `1100`

| Code | Error | Message format | Triggered by |
|------|-------|----------------|--------------|
| `1100` | `Json` | `JSON error: {details}` | `anvildb_insert`, `anvildb_bulk_insert`, `anvildb_update`, `anvildb_query`, `anvildb_count`, `anvildb_set_schema` |

### Collection errors — `1200–1201`

| Code | Error | Message format | Triggered by |
|------|-------|----------------|--------------|
| `1200` | `CollectionNotFound` | `Collection not found: {name}` | `anvildb_insert`, `anvildb_bulk_insert`, `anvildb_find_by_id`, `anvildb_update`, `anvildb_delete`, `anvildb_query`, `anvildb_count`, `anvildb_create_index`, `anvildb_drop_index`, `anvildb_set_schema`, `anvildb_flush_collection` |
| `1201` | `CollectionAlreadyExists` | `Collection already exists: {name}` | `anvildb_create_collection` |

### Document errors — `1300–1301`

| Code | Error | Message format | Triggered by |
|------|-------|----------------|--------------|
| `1300` | `DocumentNotFound` | `Document not found: {id}` | `anvildb_find_by_id`, `anvildb_update`, `anvildb_delete` |
| `1301` | `DuplicateKey` | `Duplicate key on field '{field}': {value}` | `anvildb_insert`, `anvildb_bulk_insert`, `anvildb_update` (when a unique index exists) |

### Validation errors — `1400`

| Code | Error | Message format | Triggered by |
|------|-------|----------------|--------------|
| `1400` | `ValidationError` | `Validation error: {details}` | `anvildb_insert`, `anvildb_bulk_insert`, `anvildb_update` (when a schema is set), `anvildb_set_schema` |

### Query errors — `1500`

| Code | Error | Message format | Triggered by |
|------|-------|----------------|--------------|
| `1500` | `InvalidQuery` | `Invalid query: {details}` | `anvildb_query`, `anvildb_count` |

### Lock errors — `1600`

| Code | Error | Message format | Triggered by |
|------|-------|----------------|--------------|
| `1600` | `LockError` | `Lock error: {details}` | Any function (internal `RwLock`/`Mutex` poisoning — should not happen under normal conditions) |

### Encryption errors — `1700–1702`

| Code | Error | Message format | Triggered by |
|------|-------|----------------|--------------|
| `1700` | `EncryptionRequired` | `Database is encrypted — encryption key required` | `anvildb_open` |
| `1701` | `EncryptionError` | `Encryption error: {details}` | `anvildb_encrypt` (e.g. already encrypted) |
| `1702` | `DecryptionFailed` | `Decryption failed: {details}` | `anvildb_open` (wrong key), `anvildb_decrypt` |

### Argument errors — `1800`

| Code | Error | Message format | Triggered by |
|------|-------|----------------|--------------|
| `1800` | `InvalidArgument` | `Invalid argument: {details}` | Any FFI function receiving invalid input |

### Generic FFI errors — `-1`

These are produced by FFI input validation before reaching the engine (NULL pointers, invalid UTF-8, bad hex keys). They set error code `-1` and a descriptive message:

| Message | Triggered by |
|---------|--------------|
| `Invalid collection name` | Any function receiving a `collection` parameter |
| `Invalid JSON document` | `anvildb_insert`, `anvildb_update` |
| `Invalid JSON documents` | `anvildb_bulk_insert` |
| `Invalid id` | `anvildb_find_by_id`, `anvildb_update`, `anvildb_delete` |
| `Invalid query spec` | `anvildb_query` |
| `Invalid field name` | `anvildb_create_index`, `anvildb_drop_index` |
| `Invalid schema JSON` | `anvildb_set_schema` |
| `Invalid encryption key` | `anvildb_encrypt`, `anvildb_decrypt` |
| `Encryption key must be a 64-character hex string (32 bytes)` | `anvildb_encrypt`, `anvildb_decrypt` |
| `max_docs and flush_interval_secs must be >= 1` | `anvildb_configure_buffer` |

## Usage example (PHP)

```php
$result = $ffi->anvildb_insert($handle, 'users', '{"name": "Alice"}');

if ($result === null) {
    $code = $ffi->anvildb_last_error_code($handle);
    $message = $ffi->anvildb_last_error($handle);

    match (true) {
        $code === 1200 => throw new CollectionNotFoundException($message),
        $code === 1301 => throw new DuplicateKeyException($message),
        $code === 1400 => throw new ValidationException($message),
        default        => throw new AnvilDbException($message, $code),
    };
}
```

## Usage example (C)

```c
const char* result = anvildb_insert(handle, "users", "{\"name\": \"Alice\"}");

if (result == NULL) {
    int32_t code = anvildb_last_error_code(handle);
    const char* msg = anvildb_last_error(handle);

    fprintf(stderr, "Error %d: %s\n", code, msg ? msg : "unknown");

    if (msg) anvildb_free_string(msg);
}
```

## Warnings

Warnings are non-fatal messages accumulated during operations. They do not affect the return value of any function. Retrieve them with `anvildb_last_warning()`, which returns a JSON array of strings (or `NULL` if empty) and clears the buffer.

```c
// Returns: '["Encryption key provided but database is not encrypted"]'
// or NULL if no warnings
const char* warnings = anvildb_last_warning(handle);
if (warnings) {
    // parse JSON array...
    anvildb_free_string(warnings);
}
```

Current warning conditions:

| Warning | Condition |
|---------|-----------|
| `Encryption key provided but database is not encrypted` | `anvildb_open` called with an encryption key on a database that was created without encryption |
