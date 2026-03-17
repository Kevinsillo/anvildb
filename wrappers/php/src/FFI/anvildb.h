// anvildb.h — C API for PHP FFI

typedef void* AnvilDbHandle;

// Lifecycle
AnvilDbHandle anvildb_open(const char* data_path, const char* encryption_key);
void anvildb_close(AnvilDbHandle handle);
void anvildb_shutdown(AnvilDbHandle handle);

// Collections
int32_t anvildb_create_collection(AnvilDbHandle handle, const char* name);
int32_t anvildb_drop_collection(AnvilDbHandle handle, const char* name);
const char* anvildb_list_collections(AnvilDbHandle handle);

// CRUD
const char* anvildb_insert(AnvilDbHandle handle, const char* collection, const char* json_doc);
const char* anvildb_find_by_id(AnvilDbHandle handle, const char* collection, const char* id);
int32_t anvildb_update(AnvilDbHandle handle, const char* collection, const char* id, const char* json_doc);
int32_t anvildb_delete(AnvilDbHandle handle, const char* collection, const char* id);
const char* anvildb_bulk_insert(AnvilDbHandle handle, const char* collection, const char* json_docs);

// Queries
const char* anvildb_query(AnvilDbHandle handle, const char* json_query_spec);
int64_t anvildb_count(AnvilDbHandle handle, const char* collection, const char* json_filter);

// Indexes
int32_t anvildb_create_index(AnvilDbHandle handle, const char* collection, const char* field, const char* index_type);
int32_t anvildb_drop_index(AnvilDbHandle handle, const char* collection, const char* field);

// Schema
int32_t anvildb_set_schema(AnvilDbHandle handle, const char* collection, const char* json_schema);

// Buffer control
int32_t anvildb_flush(AnvilDbHandle handle);
int32_t anvildb_flush_collection(AnvilDbHandle handle, const char* collection);
int32_t anvildb_configure_buffer(AnvilDbHandle handle, int32_t max_docs, int32_t flush_interval_secs);

// Encryption
int32_t anvildb_encrypt(AnvilDbHandle handle, const char* encryption_key);
int32_t anvildb_decrypt(AnvilDbHandle handle, const char* encryption_key);

// Cache
void anvildb_clear_cache(AnvilDbHandle handle);

// Errors + Warnings
const char* anvildb_last_error(AnvilDbHandle handle);
int32_t anvildb_last_error_code(AnvilDbHandle handle);
const char* anvildb_last_warning(AnvilDbHandle handle);

// Memory management
void anvildb_free_string(const char* ptr);
