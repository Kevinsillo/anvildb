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

    public function __construct(string $dataPath)
    {
        $ffi = Bridge::get();
        $handle = $ffi->anvildb_open($dataPath);

        if ($handle === null) {
            throw new FFIException('Failed to open AnvilDb engine');
        }

        $this->handle = $handle;
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
