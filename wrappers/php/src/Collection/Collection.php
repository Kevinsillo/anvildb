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
     * A UUID `id` field is auto-generated if not provided in the document.
     *
     * @param array<string, mixed> $document Document data as an associative array.
     *                                       Supports nested arrays and scalar types (string, int, float, bool, null).
     *
     * @return array<string, mixed> The inserted document including the generated `id` field
     *
     * @throws AnvilDbException If the insert fails (e.g. schema validation error)
     * @throws \JsonException   If encoding/decoding fails
     *
     * ```php
     * $doc = $collection->insert(['name' => 'Alice', 'age' => 25]);
     * echo $doc['id']; // "550e8400-e29b-41d4-a716-446655440000"
     * ```
     *
     * @see bulkInsert() For inserting multiple documents at once
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
     * More efficient than calling {@see insert()} in a loop, as it sends all
     * documents to the engine in a single FFI call.
     *
     * @param array<int, array<string, mixed>> $documents Indexed array of document arrays
     *
     * @return array<int, array<string, mixed>> The inserted documents with generated IDs
     *
     * @throws AnvilDbException If the bulk insert fails
     * @throws \JsonException   If encoding/decoding fails
     *
     * ```php
     * $docs = $collection->bulkInsert([
     *     ['name' => 'Alice', 'age' => 25],
     *     ['name' => 'Bob', 'age' => 30],
     * ]);
     * ```
     *
     * @see insert() For inserting a single document
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
     * @param string $id Document UUID (e.g. "550e8400-e29b-41d4-a716-446655440000")
     *
     * @return array<string, mixed>|null The document as an associative array, or `null` if not found
     *
     * @throws \JsonException If decoding fails
     *
     * ```php
     * $doc = $collection->find('550e8400-e29b-41d4-a716-446655440000');
     * if ($doc !== null) {
     *     echo $doc['name'];
     * }
     * ```
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
     * Replaces the document fields with the given data. The `id` field is preserved.
     *
     * @param string               $id   Document UUID
     * @param array<string, mixed> $data New field values (replaces existing fields)
     *
     * @return bool `true` if the document was updated successfully
     *
     * @throws AnvilDbException If the update fails (e.g. document not found)
     * @throws \JsonException   If encoding fails
     *
     * ```php
     * $collection->update('550e8400-...', ['name' => 'Alice Updated', 'age' => 26]);
     * ```
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
     * Returns a {@see QueryBuilder} for chaining additional filters, sorting, and pagination.
     *
     * @param string $field    Field name to filter on (supports dot notation for nested fields)
     * @param string $operator Comparison operator. Supported values:
     *                         `'='`, `'!='`, `'>'`, `'<'`, `'>='`, `'<='`,
     *                         `'contains'`, `'between'`, `'in'`, `'not_in'`
     * @param mixed  $value    Value to compare against. Type depends on operator:
     *                         - Scalar for `=`, `!=`, `>`, `<`, `>=`, `<=`, `contains`
     *                         - `[min, max]` array for `between`
     *                         - `array` for `in`, `not_in`
     *
     * @return QueryBuilder Fluent query builder instance
     *
     * ```php
     * // Simple equality
     * $users = $collection->where('status', '=', 'active')->get();
     *
     * // Chained filters with sorting
     * $users = $collection->where('age', '>=', 18)
     *     ->where('status', '=', 'active')
     *     ->orderBy('name', 'asc')
     *     ->limit(10)
     *     ->get();
     * ```
     *
     * @see whereBetween() Shorthand for range queries
     * @see whereIn()      Shorthand for IN queries
     */
    public function where(string $field, string $operator, mixed $value): QueryBuilder
    {
        return (new QueryBuilder($this->handle, $this->name))
            ->where($field, $operator, $value);
    }

    /**
     * Start a query filtering by a range (inclusive on both ends).
     *
     * Equivalent to `where($field, 'between', [$min, $max])`.
     *
     * @param string    $field Field name
     * @param int|float $min   Minimum value (inclusive)
     * @param int|float $max   Maximum value (inclusive)
     *
     * @return QueryBuilder Fluent query builder instance
     *
     * ```php
     * $adults = $collection->whereBetween('age', 18, 65)->get();
     * ```
     *
     * @see where() For other comparison operators
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
     * @param array<mixed> $values Allowed values to match against
     *
     * @return QueryBuilder Fluent query builder instance
     *
     * ```php
     * $docs = $collection->whereIn('status', ['active', 'pending'])->get();
     * ```
     *
     * @see whereNotIn() For the inverse filter
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
     * @param array<mixed> $values Values to exclude from results
     *
     * @return QueryBuilder Fluent query builder instance
     *
     * ```php
     * $docs = $collection->whereNotIn('status', ['banned', 'deleted'])->get();
     * ```
     *
     * @see whereIn() For the inverse filter
     */
    public function whereNotIn(string $field, array $values): QueryBuilder
    {
        return (new QueryBuilder($this->handle, $this->name))
            ->whereNotIn($field, $values);
    }

    /**
     * Start a query with a join to another collection.
     *
     * Joined fields are prefixed to avoid name collisions. By default the prefix is `{collection}_`.
     * Filters, sorting, and pagination apply after all joins, so you can reference prefixed fields.
     *
     * @param string      $collection Target collection name
     * @param string      $leftField  Field on this collection (the left side)
     * @param string      $rightField Field on the target collection (the right side)
     * @param string      $type       Join type: `'inner'` (default) or `'left'`
     * @param string|null $prefix     Prefix for joined fields (defaults to `{collection}_`)
     *
     * @return QueryBuilder Fluent query builder instance
     *
     * ```php
     * // Inner join orders → users
     * $results = $orders->join('users', 'user_id', 'id', 'inner', 'u_')
     *     ->where('u_status', '=', 'active')
     *     ->get();
     * ```
     *
     * @see leftJoin() Shorthand for left joins
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
     * Unmatched left rows are included in results without right-side fields.
     * Shorthand for `join($collection, $leftField, $rightField, 'left', $prefix)`.
     *
     * @param string      $collection Target collection name
     * @param string      $leftField  Field on this collection (the left side)
     * @param string      $rightField Field on the target collection (the right side)
     * @param string|null $prefix     Prefix for joined fields (defaults to `{collection}_`)
     *
     * @return QueryBuilder Fluent query builder instance
     *
     * ```php
     * $results = $orders->leftJoin('users', 'user_id', 'id', 'u_')->get();
     * ```
     *
     * @see join() For inner joins or more control
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
     * Start a query with a SUM aggregation.
     *
     * When any aggregation is present, `get()` returns a single result object
     * with the computed values instead of document rows.
     *
     * @param string      $field Field to sum (must contain numeric values)
     * @param string|null $alias Result key name (defaults to `sum_{$field}`)
     *
     * @return QueryBuilder Fluent query builder instance
     *
     * ```php
     * $result = $collection->sum('amount', 'total')->get();
     * // [['total' => 1500.00]]
     * ```
     *
     * @see avg()     For average aggregation
     * @see groupBy() For grouped aggregations
     */
    public function sum(string $field, ?string $alias = null): QueryBuilder
    {
        return (new QueryBuilder($this->handle, $this->name))
            ->sum($field, $alias);
    }

    /**
     * Start a query with an AVG aggregation.
     *
     * @param string      $field Field to average (must contain numeric values)
     * @param string|null $alias Result key name (defaults to `avg_{$field}`)
     *
     * @return QueryBuilder Fluent query builder instance
     *
     * ```php
     * $result = $collection->avg('age', 'average_age')->get();
     * // [['average_age' => 28.5]]
     * ```
     *
     * @see sum()     For sum aggregation
     * @see groupBy() For grouped aggregations
     */
    public function avg(string $field, ?string $alias = null): QueryBuilder
    {
        return (new QueryBuilder($this->handle, $this->name))
            ->avg($field, $alias);
    }

    /**
     * Start a query with a MIN aggregation.
     *
     * @param string      $field Field to find the minimum value of
     * @param string|null $alias Result key name (defaults to `min_{$field}`)
     *
     * @return QueryBuilder Fluent query builder instance
     *
     * ```php
     * $result = $collection->min('price', 'cheapest')->get();
     * // [['cheapest' => 9.99]]
     * ```
     */
    public function min(string $field, ?string $alias = null): QueryBuilder
    {
        return (new QueryBuilder($this->handle, $this->name))
            ->min($field, $alias);
    }

    /**
     * Start a query with a MAX aggregation.
     *
     * @param string      $field Field to find the maximum value of
     * @param string|null $alias Result key name (defaults to `max_{$field}`)
     *
     * @return QueryBuilder Fluent query builder instance
     *
     * ```php
     * $result = $collection->max('price', 'most_expensive')->get();
     * // [['most_expensive' => 999.99]]
     * ```
     */
    public function max(string $field, ?string $alias = null): QueryBuilder
    {
        return (new QueryBuilder($this->handle, $this->name))
            ->max($field, $alias);
    }

    /**
     * Start a query with a GROUP BY clause.
     *
     * Groups results by the given field(s) and applies aggregations per group.
     *
     * @param string|array<string>        $fields       Field name or array of field names to group by
     * @param array<array<string, mixed>> $aggregations Aggregation definitions. Each entry is an associative array:
     *                                                  `['function' => string, 'field' => string, 'alias' => string]`
     *                                                  Supported functions: `'sum'`, `'avg'`, `'min'`, `'max'`, `'count'`
     *
     * @return QueryBuilder Fluent query builder instance
     *
     * ```php
     * $results = $collection->groupBy('department', [
     *     ['function' => 'avg', 'field' => 'salary', 'alias' => 'avg_salary'],
     *     ['function' => 'count', 'field' => 'id', 'alias' => 'total'],
     * ])->get();
     * // [['department' => 'engineering', 'avg_salary' => 85000, 'total' => 12], ...]
     * ```
     */
    public function groupBy(string|array $fields, array $aggregations = []): QueryBuilder
    {
        return (new QueryBuilder($this->handle, $this->name))
            ->groupBy($fields, $aggregations);
    }

    /**
     * Start a query with an ordering clause.
     *
     * @param string $field     Field name to sort by
     * @param string $direction Sort direction: `'asc'` (default) or `'desc'`
     *
     * @return QueryBuilder Fluent query builder instance
     *
     * ```php
     * $newest = $collection->orderBy('created_at', 'desc')->limit(10)->get();
     * ```
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
     * Create an index on a field to speed up queries that filter on it.
     *
     * Indexes are persisted to disk and survive database restarts.
     *
     * @param string $field Field name to index (must exist in documents)
     * @param string $type  Index type. Supported values:
     *                      - `'hash'`   — O(1) lookups for equality queries (`=`, `!=`, `in`, `not_in`). Default.
     *                      - `'unique'` — Same as hash but enforces unique values per document.
     *                      - `'range'`  — B-tree index for range queries (`>`, `<`, `>=`, `<=`, `between`).
     *
     * @return void
     *
     * @throws AnvilDbException If index creation fails (e.g. duplicate values on unique index)
     *
     * ```php
     * // Hash index for fast equality lookups
     * $collection->createIndex('email', 'unique');
     *
     * // Range index for sorting and range queries
     * $collection->createIndex('age', 'range');
     * ```
     *
     * @see dropIndex() To remove an existing index
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
     * Drop an existing index on a field.
     *
     * @param string $field Field name whose index should be removed
     *
     * @return void
     *
     * @throws AnvilDbException If dropping the index fails (e.g. index does not exist)
     *
     * @see createIndex() To create an index
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
     * Once set, all inserts and updates are validated against this schema.
     * Documents with invalid types will throw an {@see AnvilDbException}.
     *
     * @param array<string, string> $schema Associative array mapping field names to types.
     *                                      Supported types: `'string'`, `'int'`, `'float'`, `'bool'`, `'array'`, `'object'`
     *
     * @return void
     *
     * @throws AnvilDbException If setting the schema fails
     * @throws \JsonException   If encoding fails
     *
     * ```php
     * $collection->setSchema([
     *     'name'   => 'string',
     *     'age'    => 'int',
     *     'score'  => 'float',
     *     'active' => 'bool',
     *     'tags'   => 'array',
     * ]);
     * ```
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
     * Only needed when using buffered writes ({@see \AnvilDb\AnvilDb::configureBuffer()}).
     * Writes are persisted immediately if buffering is not configured.
     *
     * @return void
     *
     * @throws AnvilDbException If the flush fails
     *
     * @see \AnvilDb\AnvilDb::flush() To flush all collections at once
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
     * Nested arrays and objects are JSON-encoded in the CSV cells.
     *
     * @param string             $filePath Absolute or relative path for the output CSV file
     * @param array<string>|null $fields   Column names to export. If `null`, infers columns from the first document's keys.
     *
     * @return int Number of documents exported (0 if collection is empty)
     *
     * @throws AnvilDbException If the file cannot be opened for writing
     *
     * ```php
     * // Export all fields
     * $count = $collection->exportCsv('/tmp/users.csv');
     *
     * // Export specific fields only
     * $count = $collection->exportCsv('/tmp/users.csv', ['name', 'email']);
     * ```
     *
     * @see importCsv() To import documents from a CSV file
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
     * The first row is used as field names (headers). Numeric, boolean, and null
     * values are auto-cast. JSON-encoded cells are decoded back to arrays/objects.
     * Documents are inserted in batches of 1000 for performance.
     *
     * @param string $filePath Absolute or relative path to the CSV file
     *
     * @return int Number of documents imported
     *
     * @throws AnvilDbException If the file cannot be read or insert fails
     *
     * ```php
     * $count = $collection->importCsv('/tmp/users.csv');
     * echo "Imported {$count} documents";
     * ```
     *
     * @see exportCsv() To export documents to a CSV file
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
