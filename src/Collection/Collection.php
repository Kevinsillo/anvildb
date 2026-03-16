<?php

declare(strict_types=1);

namespace AnvilDb\Collection;

use AnvilDb\Exception\AnvilDbException;
use AnvilDb\FFI\Bridge;
use AnvilDb\Query\QueryBuilder;

/**
 * Represents a document collection within an AnvilDB database.
 */
class Collection
{
    private \FFI\CData $handle;
    private string $name;

    /**
     * @param \FFI\CData $handle Database engine handle
     * @param string     $name   Collection name
     */
    public function __construct(\FFI\CData $handle, string $name)
    {
        $this->handle = $handle;
        $this->name = $name;
    }

    /**
     * Insert a single document into the collection.
     *
     * @param array<string, mixed> $document Document data as an associative array
     *
     * @return array<string, mixed> The inserted document (with generated ID)
     *
     * @throws AnvilDbException If the insert fails
     * @throws \JsonException   If encoding/decoding fails
     */
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

    /**
     * Insert multiple documents into the collection in a single operation.
     *
     * @param array<int, array<string, mixed>> $documents Array of document arrays
     *
     * @return array<int, array<string, mixed>> The inserted documents
     *
     * @throws AnvilDbException If the bulk insert fails
     * @throws \JsonException   If encoding/decoding fails
     */
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

    /**
     * Find a document by its ID.
     *
     * @param string $id Document ID
     *
     * @return array<string, mixed>|null The document, or null if not found
     *
     * @throws \JsonException If decoding fails
     */
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

    /**
     * Update a document by its ID.
     *
     * @param string               $id   Document ID
     * @param array<string, mixed> $data Fields to update
     *
     * @return bool True if the document was updated
     *
     * @throws AnvilDbException If the update fails
     * @throws \JsonException   If encoding fails
     */
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

    /**
     * Delete a document by its ID.
     *
     * @param string $id Document ID
     *
     * @return bool True if the document was deleted
     *
     * @throws AnvilDbException If the delete fails
     */
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

    /**
     * Start a query with a where clause.
     *
     * @param string $field    Field name to filter on
     * @param string $operator Comparison operator (e.g. '=', '>', '<')
     * @param mixed  $value    Value to compare against
     *
     * @return QueryBuilder
     */
    public function where(string $field, string $operator, mixed $value): QueryBuilder
    {
        return (new QueryBuilder($this->handle, $this->name))
            ->where($field, $operator, $value);
    }

    /**
     * Start a query filtering by a range (inclusive).
     *
     * @param string $field Field name
     * @param mixed  $min   Minimum value
     * @param mixed  $max   Maximum value
     *
     * @return QueryBuilder
     */
    public function whereBetween(string $field, mixed $min, mixed $max): QueryBuilder
    {
        return (new QueryBuilder($this->handle, $this->name))
            ->whereBetween($field, $min, $max);
    }

    /**
     * Start a query filtering where a field matches any value in the list.
     *
     * @param string       $field  Field name
     * @param array<mixed> $values Allowed values
     *
     * @return QueryBuilder
     */
    public function whereIn(string $field, array $values): QueryBuilder
    {
        return (new QueryBuilder($this->handle, $this->name))
            ->whereIn($field, $values);
    }

    /**
     * Start a query filtering where a field does not match any value in the list.
     *
     * @param string       $field  Field name
     * @param array<mixed> $values Excluded values
     *
     * @return QueryBuilder
     */
    public function whereNotIn(string $field, array $values): QueryBuilder
    {
        return (new QueryBuilder($this->handle, $this->name))
            ->whereNotIn($field, $values);
    }

    /**
     * Start a query filtering by a regular expression pattern.
     *
     * @param string $field   Field name
     * @param string $pattern Regex pattern
     *
     * @return QueryBuilder
     */
    public function whereRegex(string $field, string $pattern): QueryBuilder
    {
        return (new QueryBuilder($this->handle, $this->name))
            ->whereRegex($field, $pattern);
    }

    /**
     * Start a query with a join to another collection.
     *
     * @param string      $collection Target collection name
     * @param string      $leftField  Field on this collection
     * @param string      $rightField Field on the target collection
     * @param string      $type       Join type ('inner', 'left')
     * @param string|null $prefix     Optional prefix for joined fields
     *
     * @return QueryBuilder
     */
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

    /**
     * Start a query with a left join to another collection.
     *
     * @param string      $collection Target collection name
     * @param string      $leftField  Field on this collection
     * @param string      $rightField Field on the target collection
     * @param string|null $prefix     Optional prefix for joined fields
     *
     * @return QueryBuilder
     */
    public function leftJoin(
        string $collection,
        string $leftField,
        string $rightField,
        ?string $prefix = null,
    ): QueryBuilder {
        return (new QueryBuilder($this->handle, $this->name))
            ->leftJoin($collection, $leftField, $rightField, $prefix);
    }

    /**
     * Start a query with an ordering clause.
     *
     * @param string $field     Field to sort by
     * @param string $direction Sort direction ('asc' or 'desc')
     *
     * @return QueryBuilder
     */
    public function orderBy(string $field, string $direction = 'asc'): QueryBuilder
    {
        return (new QueryBuilder($this->handle, $this->name))
            ->orderBy($field, $direction);
    }

    /**
     * Retrieve all documents in the collection.
     *
     * @return array<int, array<string, mixed>> All documents
     *
     * @throws AnvilDbException If the query fails
     */
    public function all(): array
    {
        return (new QueryBuilder($this->handle, $this->name))->get();
    }

    /**
     * Count all documents in the collection.
     *
     * @return int Number of documents
     *
     * @throws AnvilDbException If the count fails
     */
    public function count(): int
    {
        return (new QueryBuilder($this->handle, $this->name))->count();
    }

    /**
     * Create an index on a field.
     *
     * @param string $field Field name to index
     * @param string $type  Index type ('hash', 'btree', etc.)
     *
     * @return void
     *
     * @throws AnvilDbException If index creation fails
     */
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

    /**
     * Drop an index on a field.
     *
     * @param string $field Field name whose index should be dropped
     *
     * @return void
     *
     * @throws AnvilDbException If dropping the index fails
     */
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

    /**
     * Set a validation schema for this collection.
     *
     * @param array<string, mixed> $schema Schema definition
     *
     * @return void
     *
     * @throws AnvilDbException If setting the schema fails
     * @throws \JsonException   If encoding fails
     */
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

    /**
     * Flush buffered writes for this collection to disk.
     *
     * @return void
     *
     * @throws AnvilDbException If the flush fails
     */
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

    /**
     * Export all documents to a CSV file.
     *
     * @param string             $filePath Output CSV file path
     * @param array<string>|null $fields   Columns to export (defaults to keys of first document)
     *
     * @return int Number of documents exported
     *
     * @throws AnvilDbException If the file cannot be opened
     */
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

    /**
     * Import documents from a CSV file into the collection.
     *
     * @param string $filePath Path to the CSV file
     *
     * @return int Number of documents imported
     *
     * @throws AnvilDbException If the file cannot be read or insert fails
     */
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

    /**
     * Get the collection name.
     *
     * @return string
     */
    public function getName(): string
    {
        return $this->name;
    }
}
