# C API Reference

[< Back to index](index.md)

The core engine exposes a C-compatible API via `extern "C"` functions. Any language with FFI support can use it.

The header file is at `wrappers/php/src/FFI/anvildb.h`.

## Handle

```c
typedef void* AnvilDbHandle;
```

An opaque pointer to the engine instance. All operations receive this handle.

## Lifecycle

```c
// Open a database. encryption_key is NULL for unencrypted DBs, or a 64-char hex string.
// Returns a handle, or NULL on error.
AnvilDbHandle anvildb_open(const char* data_path, const char* encryption_key);

// Close the engine and free resources. The handle is invalid after this call.
void anvildb_close(AnvilDbHandle handle);

// Flush all pending write buffers, then close.
void anvildb_shutdown(AnvilDbHandle handle);
```

## Collections

```c
// Returns 0 on success, -1 on error.
int32_t anvildb_create_collection(AnvilDbHandle handle, const char* name);
int32_t anvildb_drop_collection(AnvilDbHandle handle, const char* name);

// Returns a JSON array of collection names. Caller must free with anvildb_free_string().
const char* anvildb_list_collections(AnvilDbHandle handle);
```

## CRUD

```c
// Insert a JSON document. Returns the inserted document (with auto-generated "id") as JSON.
// Caller must free. Returns NULL on error.
const char* anvildb_insert(AnvilDbHandle handle, const char* collection, const char* json_doc);

// Find by ID. Returns JSON or NULL if not found. Caller must free.
const char* anvildb_find_by_id(AnvilDbHandle handle, const char* collection, const char* id);

// Update a document. Returns 0 on success, -1 on error.
int32_t anvildb_update(AnvilDbHandle handle, const char* collection, const char* id, const char* json_doc);

// Delete a document. Returns 0 on success, -1 on error.
int32_t anvildb_delete(AnvilDbHandle handle, const char* collection, const char* id);

// Bulk insert. json_docs is a JSON array. Returns a JSON array of inserted documents.
// Caller must free. Returns NULL on error.
const char* anvildb_bulk_insert(AnvilDbHandle handle, const char* collection, const char* json_docs);
```

## Queries

```c
// Execute a query. json_query_spec is a JSON object:
// {
//   "collection": "users",
//   "filters": [{"field": "age", "operator": ">", "value": 25}],
//   "sort": {"field": "name", "direction": "asc"},
//   "limit": 10,
//   "offset": 0,
//   "joins": [{"collection": "orders", "left_field": "id", "right_field": "user_id",
//              "type": "inner", "prefix": "order_"}],
//   "aggregations": [{"function": "sum", "field": "total", "alias": "total_sum"}],
//   "group_by": ["category"]
// }
// Returns a JSON array of results. Caller must free.
const char* anvildb_query(AnvilDbHandle handle, const char* json_query_spec);

// Count documents. json_filter is a JSON array of filter objects, or NULL for all.
// Returns the count, or -1 on error.
int64_t anvildb_count(AnvilDbHandle handle, const char* collection, const char* json_filter);
```

### Filter operators

`=`, `!=`, `>`, `<`, `>=`, `<=`, `contains`, `between`, `in`, `not_in`

### Join types

`inner`, `left`

### Aggregation functions

`sum`, `avg`, `min`, `max`, `count`

## Indexes

```c
// index_type: "hash", "unique", or "range"
int32_t anvildb_create_index(AnvilDbHandle handle, const char* collection, const char* field, const char* index_type);
int32_t anvildb_drop_index(AnvilDbHandle handle, const char* collection, const char* field);
```

## Schema Validation

```c
// json_schema maps field names to types: {"name": "string", "age": "int"}
// Types: "string", "int", "float", "bool", "array", "object"
int32_t anvildb_set_schema(AnvilDbHandle handle, const char* collection, const char* json_schema);
```

## Buffer Control

```c
int32_t anvildb_flush(AnvilDbHandle handle);
int32_t anvildb_flush_collection(AnvilDbHandle handle, const char* collection);

// Both values must be >= 1.
int32_t anvildb_configure_buffer(AnvilDbHandle handle, int32_t max_docs, int32_t flush_interval_secs);
```

## Encryption

```c
// encryption_key must be a 64-char hex string (32 bytes).
int32_t anvildb_encrypt(AnvilDbHandle handle, const char* encryption_key);
int32_t anvildb_decrypt(AnvilDbHandle handle, const char* encryption_key);
```

## Cache

```c
void anvildb_clear_cache(AnvilDbHandle handle);
```

## Error Handling

```c
// Returns the last error message, or NULL. Consuming (second call returns NULL). Caller must free.
const char* anvildb_last_error(AnvilDbHandle handle);

// Returns the last error code, or 0. Consuming.
int32_t anvildb_last_error_code(AnvilDbHandle handle);

// Returns a JSON array of warning strings, or NULL. Consuming. Caller must free.
const char* anvildb_last_warning(AnvilDbHandle handle);
```

See [Error Codes](error-codes.md) for the full list of codes and their meanings.

## Memory Management

```c
// Free a string returned by any anvildb_* function. MUST be called for every non-NULL string.
void anvildb_free_string(const char* ptr);
```

## Return Value Conventions

| Return type | Success | Error |
|------------|---------|-------|
| `const char*` | JSON string (must free) | `NULL` |
| `int32_t` | `0` | `-1` |
| `int64_t` | count value | `-1` |
| `AnvilDbHandle` | valid pointer | `NULL` |

After any error, call `anvildb_last_error_code()` and `anvildb_last_error()` to get details.
