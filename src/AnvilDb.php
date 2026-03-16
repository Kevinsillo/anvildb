<?php

declare(strict_types=1);

namespace AnvilDb;

use AnvilDb\Collection\Collection;
use AnvilDb\Exception\FFIException;
use AnvilDb\Exception\AnvilDbException;
use AnvilDb\FFI\Bridge;

class AnvilDb
{
    private \FFI\CData $handle;
    private bool $closed = false;

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

    public function __destruct()
    {
        $this->close();
    }

    public function close(): void
    {
        if (!$this->closed) {
            $ffi = Bridge::get();
            $ffi->anvildb_close($this->handle);
            $this->closed = true;
        }
    }

    public function shutdown(): void
    {
        if (!$this->closed) {
            $ffi = Bridge::get();
            $ffi->anvildb_shutdown($this->handle);
            $this->closed = true;
        }
    }

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

    public function collection(string $name): Collection
    {
        $this->ensureOpen();
        return new Collection($this->handle, $name);
    }

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
