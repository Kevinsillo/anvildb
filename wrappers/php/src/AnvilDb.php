<?php

declare(strict_types=1);

namespace AnvilDb;

use AnvilDb\Collection\Collection;
use AnvilDb\Exception\FFIException;
use AnvilDb\Exception\AnvilDbException;
use AnvilDb\FFI\Bridge;

/**
 * Main entry point for the AnvilDB embedded document database.
 */
class AnvilDb
{
    private \FFI\CData $handle;
    private bool $closed = false;

    /**
     * Open an AnvilDB database at the given path.
     *
     * Creates the data directory if it does not exist. To open an encrypted database,
     * pass the same encryption key used during {@see encrypt()}.
     *
     * @param string      $dataPath      Filesystem path to the database directory (created if missing)
     * @param string|null $encryptionKey 64-character hex string (32 bytes) for AES-256-GCM at-rest encryption.
     *                                   Pass `null` for an unencrypted database.
     *
     * @throws FFIException If the native engine fails to load or open
     *
     * ```php
     * // Unencrypted
     * $db = new AnvilDb('/var/data/mydb');
     *
     * // Encrypted
     * $db = new AnvilDb('/var/data/mydb', 'aabbccdd...64hex_chars...');
     * ```
     */
    public function __construct(string $dataPath, ?string $encryptionKey = null)
    {
        $ffi = Bridge::get();
        $handle = $ffi->anvildb_open($dataPath, $encryptionKey);

        if ($handle === null) {
            throw new FFIException('Failed to open AnvilDb engine');
        }

        $this->handle = $handle;

        // Surface any warnings from the engine (e.g. key passed to unencrypted DB)
        $this->consumeWarnings();
    }

    /**
     * Destructor that ensures the database handle is closed.
     */
    public function __destruct()
    {
        $this->close();
    }

    /**
     * Close the database handle and release resources.
     *
     * Called automatically on object destruction. Safe to call multiple times.
     *
     * @return void
     *
     * @see shutdown() For a graceful shutdown that also flushes pending writes
     */
    public function close(): void
    {
        if (!$this->closed) {
            $ffi = Bridge::get();
            $ffi->anvildb_close($this->handle);
            $this->closed = true;
        }
    }

    /**
     * Gracefully shut down the database engine.
     *
     * Flushes all pending write buffers and closes the engine.
     * The handle is no longer usable after this call.
     *
     * @return void
     *
     * @see close() For closing without flushing
     * @see flush() To flush without closing
     */
    public function shutdown(): void
    {
        if (!$this->closed) {
            $ffi = Bridge::get();
            $ffi->anvildb_shutdown($this->handle);
            $this->closed = true;
        }
    }

    /**
     * Flush all pending buffered writes to disk across all collections.
     *
     * Only needed when using buffered writes ({@see configureBuffer()}).
     *
     * @return void
     *
     * @throws AnvilDbException If the flush operation fails
     *
     * @see configureBuffer()             To configure buffer thresholds
     * @see Collection\Collection::flush() To flush a single collection
     */
    public function flush(): void
    {
        $this->ensureOpen();
        $ffi = Bridge::get();
        $result = $ffi->anvildb_flush($this->handle);

        if ($result < 0) {
            $error = $ffi->anvildb_last_error($this->handle);
            $errorMsg = is_string($error) ? $error : ($error !== null ? \FFI::string($error) : 'Unknown flush error');
            throw new AnvilDbException("Failed to flush: {$errorMsg}");
        }
    }

    /**
     * Configure the write buffer size and auto-flush interval.
     *
     * When buffering is enabled, inserts are batched in memory and flushed to disk
     * either when the document threshold is reached or the timer fires — whichever comes first.
     *
     * @param int $maxDocs           Per-collection document threshold that triggers an auto-flush (default: 100)
     * @param int $flushIntervalSecs Background timer interval in seconds (default: 5)
     *
     * @return void
     *
     * @throws AnvilDbException If the configuration fails
     *
     * ```php
     * $db->configureBuffer(200, 10); // flush every 200 docs or 10 seconds
     * ```
     *
     * @see flush()    To manually flush all pending writes
     * @see shutdown() Flushes automatically before closing
     */
    public function configureBuffer(int $maxDocs = 100, int $flushIntervalSecs = 5): void
    {
        $this->ensureOpen();
        $ffi = Bridge::get();
        $result = $ffi->anvildb_configure_buffer($this->handle, $maxDocs, $flushIntervalSecs);

        if ($result < 0) {
            $error = $ffi->anvildb_last_error($this->handle);
            $errorMsg = is_string($error) ? $error : ($error !== null ? \FFI::string($error) : 'Unknown error');
            throw new AnvilDbException("Failed to configure buffer: {$errorMsg}");
        }
    }

    /**
     * Get a collection handle for querying and manipulating documents.
     *
     * The collection does not need to exist beforehand — it is created implicitly on first insert.
     * Use {@see createCollection()} if you need to create it explicitly.
     *
     * @param string $name Collection name
     *
     * @return Collection Fluent collection interface for CRUD, queries, indexes, and aggregations
     *
     * @throws AnvilDbException If the database is closed
     *
     * ```php
     * $users = $db->collection('users');
     * $users->insert(['name' => 'Alice']);
     * ```
     */
    public function collection(string $name): Collection
    {
        $this->ensureOpen();
        return new Collection($this->handle, $name);
    }

    /**
     * Create a new collection explicitly.
     *
     * Not required for normal use — collections are created implicitly on first insert.
     *
     * @param string $name Collection name
     *
     * @return void
     *
     * @throws AnvilDbException If creation fails (e.g. collection already exists)
     *
     * @see dropCollection() To remove a collection
     * @see collection()     To get a handle without creating explicitly
     */
    public function createCollection(string $name): void
    {
        $this->ensureOpen();
        $ffi = Bridge::get();
        $result = $ffi->anvildb_create_collection($this->handle, $name);

        if ($result < 0) {
            $error = $ffi->anvildb_last_error($this->handle);
            $errorMsg = is_string($error) ? $error : ($error !== null ? \FFI::string($error) : 'Unknown error');
            throw new AnvilDbException("Failed to create collection: {$errorMsg}");
        }
    }

    /**
     * Drop an existing collection and all its documents and indexes.
     *
     * **This operation is irreversible.**
     *
     * @param string $name Collection name
     *
     * @return void
     *
     * @throws AnvilDbException If the drop operation fails (e.g. collection does not exist)
     *
     * @see createCollection() To create a collection
     */
    public function dropCollection(string $name): void
    {
        $this->ensureOpen();
        $ffi = Bridge::get();
        $result = $ffi->anvildb_drop_collection($this->handle, $name);

        if ($result < 0) {
            $error = $ffi->anvildb_last_error($this->handle);
            $errorMsg = is_string($error) ? $error : ($error !== null ? \FFI::string($error) : 'Unknown error');
            throw new AnvilDbException("Failed to drop collection: {$errorMsg}");
        }
    }

    /**
     * List all collection names in the database.
     *
     * @return array<string> Array of collection names
     *
     * @throws AnvilDbException       If the database is closed
     * @throws \JsonException         If the engine returns invalid JSON
     */
    public function listCollections(): array
    {
        $this->ensureOpen();
        $ffi = Bridge::get();
        $resultPtr = $ffi->anvildb_list_collections($this->handle);

        if ($resultPtr === null) {
            return [];
        }

        if (is_string($resultPtr)) {
            $resultJson = $resultPtr;
        } else {
            $resultJson = \FFI::string($resultPtr);
            $ffi->anvildb_free_string($resultPtr);
        }

        return json_decode($resultJson, true, 512, JSON_THROW_ON_ERROR);
    }

    /**
     * Encrypt an unencrypted database with AES-256-GCM at-rest encryption.
     *
     * Rewrites all collection and index files encrypted. After calling this,
     * the same key must be passed to the constructor to open the database.
     *
     * @param string $key 64-character hex string (32 bytes). Generate with `bin2hex(random_bytes(32))`.
     *
     * @return void
     *
     * @throws AnvilDbException If encryption fails
     *
     * ```php
     * $key = bin2hex(random_bytes(32));
     * $db->encrypt($key);
     * // Save $key securely — you'll need it to open the database
     * ```
     *
     * @see decrypt() To remove encryption
     */
    public function encrypt(string $key): void
    {
        $this->ensureOpen();
        $ffi = Bridge::get();
        $result = $ffi->anvildb_encrypt($this->handle, $key);

        if ($result < 0) {
            $error = $ffi->anvildb_last_error($this->handle);
            $errorMsg = is_string($error) ? $error : ($error !== null ? \FFI::string($error) : 'Unknown error');
            throw new AnvilDbException("Failed to encrypt: {$errorMsg}");
        }
    }

    /**
     * Decrypt an encrypted database, rewriting all files without encryption.
     *
     * After this call, the database can be opened without a key.
     *
     * @param string $key 64-character hex string used to encrypt the database
     *
     * @return void
     *
     * @throws AnvilDbException If decryption fails (e.g. wrong key)
     *
     * @see encrypt() To encrypt the database
     */
    public function decrypt(string $key): void
    {
        $this->ensureOpen();
        $ffi = Bridge::get();
        $result = $ffi->anvildb_decrypt($this->handle, $key);

        if ($result < 0) {
            $error = $ffi->anvildb_last_error($this->handle);
            $errorMsg = is_string($error) ? $error : ($error !== null ? \FFI::string($error) : 'Unknown error');
            throw new AnvilDbException("Failed to decrypt: {$errorMsg}");
        }
    }

    /**
     * Clear the in-memory query cache.
     *
     * @return void
     *
     * @throws AnvilDbException If the database is closed
     */
    public function clearCache(): void
    {
        $this->ensureOpen();
        $ffi = Bridge::get();
        $ffi->anvildb_clear_cache($this->handle);
    }

    /**
     * Consume and surface all accumulated warnings from the engine.
     *
     * Each warning is emitted as an `E_USER_WARNING` via `trigger_error()`.
     */
    private function consumeWarnings(): void
    {
        $ffi = Bridge::get();
        $ptr = $ffi->anvildb_last_warning($this->handle);

        if ($ptr === null) {
            return;
        }

        $json = is_string($ptr) ? $ptr : \FFI::string($ptr);
        $ffi->anvildb_free_string($ptr);

        $warnings = json_decode($json, true);
        if (!is_array($warnings)) {
            return;
        }

        foreach ($warnings as $warning) {
            trigger_error("AnvilDB: {$warning}", E_USER_WARNING);
        }
    }

    private function ensureOpen(): void
    {
        if ($this->closed) {
            throw new AnvilDbException('AnvilDb instance is already closed');
        }
    }
}
