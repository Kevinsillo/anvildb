<?php

declare(strict_types=1);

namespace AnvilDb\FFI;

/**
 * Type stub for the AnvilDB native FFI interface.
 *
 * This interface is never implemented at runtime — it exists solely to provide
 * IDE autocompletion and static analysis for the FFI methods loaded from `anvildb.h`.
 *
 * @internal
 */
interface AnvilDbFFI
{
    // ── Lifecycle ──────────────────────────────────────────────

    /**
     * Open a database at the given path.
     *
     * @param string      $data_path      Filesystem path to the database directory
     * @param string|null $encryption_key  64-char hex key for AES-256-GCM, or `null` for unencrypted
     *
     * @return \FFI\CData|null Opaque engine handle, or `null` on failure
     */
    public function anvildb_open(string $data_path, ?string $encryption_key): ?\FFI\CData;

    /**
     * Close the engine and free resources.
     *
     * @param \FFI\CData $handle Engine handle from {@see anvildb_open()}
     */
    public function anvildb_close(\FFI\CData $handle): void;

    /**
     * Flush all buffers and close the engine.
     *
     * @param \FFI\CData $handle Engine handle
     */
    public function anvildb_shutdown(\FFI\CData $handle): void;

    // ── Collections ────────────────────────────────────────────

    /**
     * Create a new collection.
     *
     * @param \FFI\CData $handle Engine handle
     * @param string     $name   Collection name
     *
     * @return int `0` on success, negative on error
     */
    public function anvildb_create_collection(\FFI\CData $handle, string $name): int;

    /**
     * Drop a collection and delete its data.
     *
     * @param \FFI\CData $handle Engine handle
     * @param string     $name   Collection name
     *
     * @return int `0` on success, negative on error
     */
    public function anvildb_drop_collection(\FFI\CData $handle, string $name): int;

    /**
     * List all collection names as a JSON array string.
     *
     * @param \FFI\CData $handle Engine handle
     *
     * @return string|\FFI\CData|null JSON string `["col1","col2"]`, or `null` if none
     */
    public function anvildb_list_collections(\FFI\CData $handle): string|\FFI\CData|null;

    // ── CRUD ───────────────────────────────────────────────────

    /**
     * Insert a document (JSON string) into a collection.
     *
     * @param \FFI\CData $handle     Engine handle
     * @param string     $collection Collection name
     * @param string     $json_doc   JSON-encoded document
     *
     * @return string|\FFI\CData|null JSON string of inserted document (with `id`), or `null` on error
     */
    public function anvildb_insert(\FFI\CData $handle, string $collection, string $json_doc): string|\FFI\CData|null;

    /**
     * Find a document by its ID.
     *
     * @param \FFI\CData $handle     Engine handle
     * @param string     $collection Collection name
     * @param string     $id         Document UUID
     *
     * @return string|\FFI\CData|null JSON string of the document, or `null` if not found
     */
    public function anvildb_find_by_id(\FFI\CData $handle, string $collection, string $id): string|\FFI\CData|null;

    /**
     * Update a document by its ID.
     *
     * @param \FFI\CData $handle     Engine handle
     * @param string     $collection Collection name
     * @param string     $id         Document UUID
     * @param string     $json_doc   JSON-encoded replacement data
     *
     * @return int `0` on success, negative on error
     */
    public function anvildb_update(\FFI\CData $handle, string $collection, string $id, string $json_doc): int;

    /**
     * Delete a document by its ID.
     *
     * @param \FFI\CData $handle     Engine handle
     * @param string     $collection Collection name
     * @param string     $id         Document UUID
     *
     * @return int `0` on success, negative on error
     */
    public function anvildb_delete(\FFI\CData $handle, string $collection, string $id): int;

    /**
     * Bulk insert multiple documents.
     *
     * @param \FFI\CData $handle     Engine handle
     * @param string     $collection Collection name
     * @param string     $json_docs  JSON-encoded array of documents
     *
     * @return string|\FFI\CData|null JSON string of inserted documents, or `null` on error
     */
    public function anvildb_bulk_insert(\FFI\CData $handle, string $collection, string $json_docs): string|\FFI\CData|null;

    // ── Queries ────────────────────────────────────────────────

    /**
     * Execute a query from a JSON spec.
     *
     * @param \FFI\CData $handle          Engine handle
     * @param string     $json_query_spec JSON-encoded query specification
     *
     * @return string|\FFI\CData|null JSON string of matching documents, or `null` on error
     */
    public function anvildb_query(\FFI\CData $handle, string $json_query_spec): string|\FFI\CData|null;

    /**
     * Count documents matching a filter.
     *
     * @param \FFI\CData  $handle      Engine handle
     * @param string      $collection  Collection name
     * @param string|null $json_filter JSON-encoded filter array, or `null` for all documents
     *
     * @return int Document count, or negative on error
     */
    public function anvildb_count(\FFI\CData $handle, string $collection, ?string $json_filter): int;

    // ── Indexes ────────────────────────────────────────────────

    /**
     * Create an index on a field.
     *
     * @param \FFI\CData $handle     Engine handle
     * @param string     $collection Collection name
     * @param string     $field      Field name to index
     * @param string     $index_type Index type: `'hash'`, `'unique'`, or `'range'`
     *
     * @return int `0` on success, negative on error
     */
    public function anvildb_create_index(\FFI\CData $handle, string $collection, string $field, string $index_type): int;

    /**
     * Drop an index on a field.
     *
     * @param \FFI\CData $handle     Engine handle
     * @param string     $collection Collection name
     * @param string     $field      Indexed field name
     *
     * @return int `0` on success, negative on error
     */
    public function anvildb_drop_index(\FFI\CData $handle, string $collection, string $field): int;

    // ── Schema ─────────────────────────────────────────────────

    /**
     * Set a validation schema for a collection.
     *
     * @param \FFI\CData $handle      Engine handle
     * @param string     $collection  Collection name
     * @param string     $json_schema JSON-encoded schema `{"field": "type", ...}`
     *
     * @return int `0` on success, negative on error
     */
    public function anvildb_set_schema(\FFI\CData $handle, string $collection, string $json_schema): int;

    // ── Buffer control ─────────────────────────────────────────

    /**
     * Flush all pending buffered writes to disk.
     *
     * @param \FFI\CData $handle Engine handle
     *
     * @return int `0` on success, negative on error
     */
    public function anvildb_flush(\FFI\CData $handle): int;

    /**
     * Flush buffered writes for a single collection.
     *
     * @param \FFI\CData $handle     Engine handle
     * @param string     $collection Collection name
     *
     * @return int `0` on success, negative on error
     */
    public function anvildb_flush_collection(\FFI\CData $handle, string $collection): int;

    /**
     * Configure the write buffer thresholds.
     *
     * @param \FFI\CData $handle              Engine handle
     * @param int        $max_docs            Per-collection document threshold
     * @param int        $flush_interval_secs Background flush timer in seconds
     *
     * @return int `0` on success, negative on error
     */
    public function anvildb_configure_buffer(\FFI\CData $handle, int $max_docs, int $flush_interval_secs): int;

    // ── Encryption ─────────────────────────────────────────────

    /**
     * Encrypt the database with AES-256-GCM.
     *
     * @param \FFI\CData $handle         Engine handle
     * @param string     $encryption_key 64-char hex string (32 bytes)
     *
     * @return int `0` on success, negative on error
     */
    public function anvildb_encrypt(\FFI\CData $handle, string $encryption_key): int;

    /**
     * Decrypt an encrypted database.
     *
     * @param \FFI\CData $handle         Engine handle
     * @param string     $encryption_key 64-char hex string used to encrypt
     *
     * @return int `0` on success, negative on error
     */
    public function anvildb_decrypt(\FFI\CData $handle, string $encryption_key): int;

    // ── Cache ──────────────────────────────────────────────────

    /**
     * Clear the in-memory query cache.
     *
     * @param \FFI\CData $handle Engine handle
     */
    public function anvildb_clear_cache(\FFI\CData $handle): void;

    // ── Errors + Warnings ──────────────────────────────────────

    /**
     * Get the last error message.
     *
     * @param \FFI\CData $handle Engine handle
     *
     * @return string|\FFI\CData|null Error message, or `null` if no error
     */
    public function anvildb_last_error(\FFI\CData $handle): string|\FFI\CData|null;

    /**
     * Get the last error code.
     *
     * @param \FFI\CData $handle Engine handle
     *
     * @return int Error code (`0` = no error)
     */
    public function anvildb_last_error_code(\FFI\CData $handle): int;

    /**
     * Get all accumulated warnings as a JSON array and clear them.
     *
     * Returns a JSON-encoded string array (e.g. `["warning1", "warning2"]`),
     * or `null` if no warnings are pending.
     *
     * @param \FFI\CData $handle Engine handle
     *
     * @return string|\FFI\CData|null JSON array of warnings, or `null` if none
     */
    public function anvildb_last_warning(\FFI\CData $handle): string|\FFI\CData|null;

    // ── Memory management ──────────────────────────────────────

    /**
     * Free a string allocated by the engine.
     *
     * @param string|\FFI\CData $ptr Pointer returned by a query/insert/error function
     */
    public function anvildb_free_string(string|\FFI\CData $ptr): void;
}
