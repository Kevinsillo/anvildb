<?php

declare(strict_types=1);

namespace AnvilDb\Collection;

use AnvilDb\Exception\AnvilDbException;
use AnvilDb\FFI\Bridge;
use AnvilDb\Query\QueryBuilder;

class Collection
{
    private \FFI\CData $handle;
    private string $name;

    public function __construct(\FFI\CData $handle, string $name)
    {
        $this->handle = $handle;
        $this->name = $name;
    }

    public function insert(array $document): array
    {
        $ffi = Bridge::get();
        $json = json_encode($document, JSON_THROW_ON_ERROR);
        $resultPtr = $ffi->anvildb_insert($this->handle, $this->name, $json);

        if ($resultPtr === null) {
            $error = $ffi->anvildb_last_error($this->handle);
            $errorMsg = is_string($error) ? $error : ($error !== null ? \FFI::string($error) : 'Unknown insert error');
            throw new AnvilDbException($errorMsg);
        }

        if (is_string($resultPtr)) {
            $resultJson = $resultPtr;
        } else {
            $resultJson = \FFI::string($resultPtr);
            $ffi->anvildb_free_string($resultPtr);
        }

        return json_decode($resultJson, true, 512, JSON_THROW_ON_ERROR);
    }

    public function bulkInsert(array $documents): array
    {
        $ffi = Bridge::get();
        $json = json_encode($documents, JSON_THROW_ON_ERROR);
        $resultPtr = $ffi->anvildb_bulk_insert($this->handle, $this->name, $json);

        if ($resultPtr === null) {
            $error = $ffi->anvildb_last_error($this->handle);
            $errorMsg = is_string($error) ? $error : ($error !== null ? \FFI::string($error) : 'Unknown bulk insert error');
            throw new AnvilDbException($errorMsg);
        }

        if (is_string($resultPtr)) {
            $resultJson = $resultPtr;
        } else {
            $resultJson = \FFI::string($resultPtr);
            $ffi->anvildb_free_string($resultPtr);
        }

        return json_decode($resultJson, true, 512, JSON_THROW_ON_ERROR);
    }

    public function find(string $id): ?array
    {
        $ffi = Bridge::get();
        $resultPtr = $ffi->anvildb_find_by_id($this->handle, $this->name, $id);

        if ($resultPtr === null) {
            return null;
        }

        if (is_string($resultPtr)) {
            $resultJson = $resultPtr;
        } else {
            $resultJson = \FFI::string($resultPtr);
            $ffi->anvildb_free_string($resultPtr);
        }

        return json_decode($resultJson, true, 512, JSON_THROW_ON_ERROR);
    }

    public function update(string $id, array $data): bool
    {
        $ffi = Bridge::get();
        $json = json_encode($data, JSON_THROW_ON_ERROR);
        $result = $ffi->anvildb_update($this->handle, $this->name, $id, $json);

        if ($result < 0) {
            $error = $ffi->anvildb_last_error($this->handle);
            $errorMsg = is_string($error) ? $error : ($error !== null ? \FFI::string($error) : 'Unknown update error');
            throw new AnvilDbException($errorMsg);
        }

        return $result === 0;
    }

    public function delete(string $id): bool
    {
        $ffi = Bridge::get();
        $result = $ffi->anvildb_delete($this->handle, $this->name, $id);

        if ($result < 0) {
            $error = $ffi->anvildb_last_error($this->handle);
            $errorMsg = is_string($error) ? $error : ($error !== null ? \FFI::string($error) : 'Unknown delete error');
            throw new AnvilDbException($errorMsg);
        }

        return $result === 0;
    }

    public function where(string $field, string $operator, mixed $value): QueryBuilder
    {
        return (new QueryBuilder($this->handle, $this->name))
            ->where($field, $operator, $value);
    }

    public function orderBy(string $field, string $direction = 'asc'): QueryBuilder
    {
        return (new QueryBuilder($this->handle, $this->name))
            ->orderBy($field, $direction);
    }

    public function all(): array
    {
        return (new QueryBuilder($this->handle, $this->name))->get();
    }

    public function count(): int
    {
        return (new QueryBuilder($this->handle, $this->name))->count();
    }

    public function createIndex(string $field, string $type = 'hash'): void
    {
        $ffi = Bridge::get();
        $result = $ffi->anvildb_create_index($this->handle, $this->name, $field, $type);

        if ($result < 0) {
            $error = $ffi->anvildb_last_error($this->handle);
            $errorMsg = is_string($error) ? $error : ($error !== null ? \FFI::string($error) : 'Unknown index error');
            throw new AnvilDbException($errorMsg);
        }
    }

    public function dropIndex(string $field): void
    {
        $ffi = Bridge::get();
        $result = $ffi->anvildb_drop_index($this->handle, $this->name, $field);

        if ($result < 0) {
            $error = $ffi->anvildb_last_error($this->handle);
            $errorMsg = is_string($error) ? $error : ($error !== null ? \FFI::string($error) : 'Unknown drop index error');
            throw new AnvilDbException($errorMsg);
        }
    }

    public function setSchema(array $schema): void
    {
        $ffi = Bridge::get();
        $json = json_encode($schema, JSON_THROW_ON_ERROR);
        $result = $ffi->anvildb_set_schema($this->handle, $this->name, $json);

        if ($result < 0) {
            $error = $ffi->anvildb_last_error($this->handle);
            $errorMsg = is_string($error) ? $error : ($error !== null ? \FFI::string($error) : 'Unknown schema error');
            throw new AnvilDbException($errorMsg);
        }
    }

    public function getName(): string
    {
        return $this->name;
    }
}
