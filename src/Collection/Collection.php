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

    public function whereBetween(string $field, mixed $min, mixed $max): QueryBuilder
    {
        return (new QueryBuilder($this->handle, $this->name))
            ->whereBetween($field, $min, $max);
    }

    public function whereIn(string $field, array $values): QueryBuilder
    {
        return (new QueryBuilder($this->handle, $this->name))
            ->whereIn($field, $values);
    }

    public function whereNotIn(string $field, array $values): QueryBuilder
    {
        return (new QueryBuilder($this->handle, $this->name))
            ->whereNotIn($field, $values);
    }

    public function whereRegex(string $field, string $pattern): QueryBuilder
    {
        return (new QueryBuilder($this->handle, $this->name))
            ->whereRegex($field, $pattern);
    }

    public function join(
        string $collection,
        string $leftField,
        string $rightField,
        string $type = 'inner',
        ?string $prefix = null,
    ): QueryBuilder {
        return (new QueryBuilder($this->handle, $this->name))
            ->join($collection, $leftField, $rightField, $type, $prefix);
    }

    public function leftJoin(
        string $collection,
        string $leftField,
        string $rightField,
        ?string $prefix = null,
    ): QueryBuilder {
        return (new QueryBuilder($this->handle, $this->name))
            ->leftJoin($collection, $leftField, $rightField, $prefix);
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

    public function flush(): void
    {
        $ffi = Bridge::get();
        $result = $ffi->anvildb_flush_collection($this->handle, $this->name);

        if ($result < 0) {
            $error = $ffi->anvildb_last_error($this->handle);
            $errorMsg = is_string($error) ? $error : ($error !== null ? \FFI::string($error) : 'Unknown flush error');
            throw new AnvilDbException("Failed to flush collection: {$errorMsg}");
        }
    }

    public function exportCsv(string $filePath, ?array $fields = null): int
    {
        $docs = $this->all();
        if (empty($docs)) {
            return 0;
        }

        // Use provided fields or infer from first document
        $fields = $fields ?? array_keys($docs[0]);

        $fp = fopen($filePath, 'w');
        if ($fp === false) {
            throw new AnvilDbException("Cannot open file for writing: {$filePath}");
        }

        // Header
        fputcsv($fp, $fields);

        // Rows
        foreach ($docs as $doc) {
            $row = [];
            foreach ($fields as $field) {
                $val = $doc[$field] ?? null;
                $row[] = is_array($val) || is_object($val) ? json_encode($val) : $val;
            }
            fputcsv($fp, $row);
        }

        fclose($fp);
        return count($docs);
    }

    public function importCsv(string $filePath): int
    {
        $fp = fopen($filePath, 'r');
        if ($fp === false) {
            throw new AnvilDbException("Cannot open file for reading: {$filePath}");
        }

        // First row is header
        $headers = fgetcsv($fp);
        if ($headers === false) {
            fclose($fp);
            return 0;
        }

        $batch = [];
        $total = 0;

        while (($row = fgetcsv($fp)) !== false) {
            $doc = [];
            foreach ($headers as $i => $field) {
                $val = $row[$i] ?? null;
                // Try to decode JSON values (arrays, objects)
                if ($val !== null && $val !== '') {
                    $decoded = json_decode($val, true);
                    $doc[$field] = ($decoded !== null && json_last_error() === JSON_ERROR_NONE && (is_array($decoded) || is_object($decoded)))
                        ? $decoded
                        : $this->castValue($val);
                } else {
                    $doc[$field] = null;
                }
            }
            $batch[] = $doc;

            // Flush in batches of 1000
            if (count($batch) >= 1000) {
                $this->bulkInsert($batch);
                $total += count($batch);
                $batch = [];
            }
        }

        // Flush remaining
        if (!empty($batch)) {
            $this->bulkInsert($batch);
            $total += count($batch);
        }

        fclose($fp);
        return $total;
    }

    private function castValue(string $val): mixed
    {
        if ($val === 'true') return true;
        if ($val === 'false') return false;
        if ($val === 'null') return null;
        if (is_numeric($val)) {
            return str_contains($val, '.') ? (float) $val : (int) $val;
        }
        return $val;
    }

    public function getName(): string
    {
        return $this->name;
    }
}
