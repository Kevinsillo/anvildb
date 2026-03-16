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
     * @param string      $dataPath      Filesystem path to the database directory
     * @param string|null $encryptionKey Optional encryption key for at-rest encryption
     *
     * @throws FFIException If the engine fails to open
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
        $warningPtr = $ffi->anvildb_last_warning($this->handle);
        if ($warningPtr !== null) {
            $warning = is_string($warningPtr) ? $warningPtr : \FFI::string($warningPtr);
            $ffi->anvildb_free_string($warningPtr);
            trigger_error("AnvilDB: {$warning}", E_USER_WARNING);
        }
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
     * @return void
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
     * @return void
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
     * Flush all pending buffered writes to disk.
     *
     * @return void
     *
     * @throws AnvilDbException If the flush operation fails
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
     * @param int $maxDocs           Maximum number of documents to buffer before flushing
     * @param int $flushIntervalSecs Automatic flush interval in seconds
     *
     * @return void
     *
     * @throws AnvilDbException If the configuration fails
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
     * @param string $name Collection name
     *
     * @return Collection
     *
     * @throws AnvilDbException If the database is closed
     */
    public function collection(string $name): Collection
    {
        $this->ensureOpen();
        return new Collection($this->handle, $name);
    }

    /**
     * Create a new collection.
     *
     * @param string $name Collection name
     *
     * @return void
     *
     * @throws AnvilDbException If creation fails
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
     * Drop an existing collection and all its documents.
     *
     * @param string $name Collection name
     *
     * @return void
     *
     * @throws AnvilDbException If the drop operation fails
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
     * Enable at-rest encryption with the given key.
     *
     * @param string $key Encryption key
     *
     * @return void
     *
     * @throws AnvilDbException If encryption fails
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
     * Decrypt the database using the given key.
     *
     * @param string $key Encryption key
     *
     * @return void
     *
     * @throws AnvilDbException If decryption fails
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

    private function ensureOpen(): void
    {
        if ($this->closed) {
            throw new AnvilDbException('AnvilDb instance is already closed');
        }
    }
}
