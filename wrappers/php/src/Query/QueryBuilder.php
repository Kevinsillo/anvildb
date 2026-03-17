<?php

declare(strict_types=1);

namespace AnvilDb\Query;

use AnvilDb\Exception\AnvilDbException;
use AnvilDb\FFI\Bridge;

/**
 * Fluent query builder for constructing and executing document queries.
 */
class QueryBuilder
{
    private \FFI\CData $handle;
    private string $collection;
    private array $filters = [];
    private array $joins = [];
    private array $aggregations = [];
    private ?array $groupBy = null;
    private ?array $orderBy = null;
    private ?int $limit = null;
    private ?int $offset = null;

    /**
     * @param \FFI\CData $handle     Database engine handle
     * @param string     $collection Collection name to query
     */
    public function __construct(\FFI\CData $handle, string $collection)
    {
        $this->handle = $handle;
        $this->collection = $collection;
    }

    /**
     * Add a filter condition. Multiple calls are combined with AND logic.
     *
     * @param string $field    Field name (supports dot notation for nested fields)
     * @param string $operator Comparison operator. Supported values:
     *                         `'='`, `'!='`, `'>'`, `'<'`, `'>='`, `'<='`,
     *                         `'contains'`, `'between'`, `'in'`, `'not_in'`
     * @param mixed  $value    Value to compare against. Type depends on operator:
     *                         - Scalar for `=`, `!=`, `>`, `<`, `>=`, `<=`, `contains`
     *                         - `[min, max]` array for `between`
     *                         - `array` for `in`, `not_in`
     *
     * @return self Chainable
     *
     * ```php
     * $qb->where('age', '>=', 18)
     *     ->where('status', '=', 'active')
     *     ->get();
     * ```
     *
     * @see whereBetween() Shorthand for range queries
     * @see whereIn()      Shorthand for IN queries
     */
    public function where(string $field, string $operator, mixed $value): self
    {
        $this->filters[] = [
            'field' => $field,
            'op' => $operator,
            'value' => $value,
        ];
        return $this;
    }

    /**
     * Add a join to another collection. Can be called multiple times for multi-way joins.
     *
     * Joined fields are prefixed to avoid name collisions.
     * Filters, sorting, and pagination apply **after** all joins.
     *
     * @param string      $collection Target collection name
     * @param string      $leftField  Field on the current collection (the left side)
     * @param string      $rightField Field on the target collection (the right side)
     * @param string      $type       Join type: `'inner'` (default) or `'left'`
     * @param string|null $prefix     Prefix for joined fields (defaults to `{collection}_`)
     *
     * @return self Chainable
     *
     * ```php
     * $qb->join('users', 'user_id', 'id', 'inner', 'u_')
     *     ->join('products', 'product_id', 'id', 'inner', 'p_')
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
    ): self {
        $join = [
            'collection' => $collection,
            'join_type' => $type,
            'left_field' => $leftField,
            'right_field' => $rightField,
        ];

        if ($prefix !== null) {
            $join['prefix'] = $prefix;
        }

        $this->joins[] = $join;
        return $this;
    }

    /**
     * Add a left join to another collection.
     *
     * Unmatched left rows are included in results without right-side fields.
     * Shorthand for `join($collection, $leftField, $rightField, 'left', $prefix)`.
     *
     * @param string      $collection Target collection name
     * @param string      $leftField  Field on the current collection (the left side)
     * @param string      $rightField Field on the target collection (the right side)
     * @param string|null $prefix     Prefix for joined fields (defaults to `{collection}_`)
     *
     * @return self Chainable
     *
     * @see join() For inner joins or more control
     */
    public function leftJoin(
        string $collection,
        string $leftField,
        string $rightField,
        ?string $prefix = null,
    ): self {
        return $this->join($collection, $leftField, $rightField, 'left', $prefix);
    }

    /**
     * Add a between filter (inclusive on both ends).
     *
     * Equivalent to `where($field, 'between', [$min, $max])`.
     *
     * @param string    $field Field name
     * @param int|float $min   Minimum value (inclusive)
     * @param int|float $max   Maximum value (inclusive)
     *
     * @return self Chainable
     */
    public function whereBetween(string $field, mixed $min, mixed $max): self
    {
        $this->filters[] = [
            'field' => $field,
            'op' => 'between',
            'value' => [$min, $max],
        ];
        return $this;
    }

    /**
     * Add an "in" filter for matching any of the given values.
     *
     * @param string       $field  Field name
     * @param array<mixed> $values Allowed values to match against
     *
     * @return self Chainable
     *
     * @see whereNotIn() For the inverse filter
     */
    public function whereIn(string $field, array $values): self
    {
        $this->filters[] = [
            'field' => $field,
            'op' => 'in',
            'value' => $values,
        ];
        return $this;
    }

    /**
     * Add a "not in" filter excluding the given values.
     *
     * @param string       $field  Field name
     * @param array<mixed> $values Values to exclude from results
     *
     * @return self Chainable
     *
     * @see whereIn() For the inverse filter
     */
    public function whereNotIn(string $field, array $values): self
    {
        $this->filters[] = [
            'field' => $field,
            'op' => 'not_in',
            'value' => $values,
        ];
        return $this;
    }

    /**
     * Add a SUM aggregation.
     *
     * When any aggregation is present, `get()` returns a single result object
     * with the computed values instead of document rows.
     *
     * @param string      $field Field to sum (must contain numeric values)
     * @param string|null $alias Result key name (defaults to `sum_{$field}`)
     *
     * @return self Chainable
     *
     * @see groupBy() For grouped aggregations
     */
    public function sum(string $field, ?string $alias = null): self
    {
        $this->aggregations[] = ['function' => 'sum', 'field' => $field, 'alias' => $alias];
        return $this;
    }

    /**
     * Add an AVG aggregation.
     *
     * @param string      $field Field to average (must contain numeric values)
     * @param string|null $alias Result key name (defaults to `avg_{$field}`)
     *
     * @return self Chainable
     *
     * @see groupBy() For grouped aggregations
     */
    public function avg(string $field, ?string $alias = null): self
    {
        $this->aggregations[] = ['function' => 'avg', 'field' => $field, 'alias' => $alias];
        return $this;
    }

    /**
     * Add a MIN aggregation.
     *
     * @param string      $field Field to find the minimum value of
     * @param string|null $alias Result key name (defaults to `min_{$field}`)
     *
     * @return self Chainable
     */
    public function min(string $field, ?string $alias = null): self
    {
        $this->aggregations[] = ['function' => 'min', 'field' => $field, 'alias' => $alias];
        return $this;
    }

    /**
     * Add a MAX aggregation.
     *
     * @param string      $field Field to find the maximum value of
     * @param string|null $alias Result key name (defaults to `max_{$field}`)
     *
     * @return self Chainable
     */
    public function max(string $field, ?string $alias = null): self
    {
        $this->aggregations[] = ['function' => 'max', 'field' => $field, 'alias' => $alias];
        return $this;
    }

    /**
     * Group results by one or more fields with aggregations per group.
     *
     * @param string|array<string>        $fields       Field name or array of field names to group by
     * @param array<array<string, mixed>> $aggregations Aggregation definitions. Each entry is an associative array:
     *                                                  `['function' => string, 'field' => string, 'alias' => string]`
     *                                                  Supported functions: `'sum'`, `'avg'`, `'min'`, `'max'`, `'count'`
     *
     * @return self Chainable
     *
     * ```php
     * $qb->groupBy('department', [
     *     ['function' => 'avg', 'field' => 'salary', 'alias' => 'avg_salary'],
     *     ['function' => 'count', 'field' => 'id', 'alias' => 'total'],
     * ])->get();
     * ```
     */
    public function groupBy(string|array $fields, array $aggregations = []): self
    {
        $fields = is_array($fields) ? $fields : [$fields];
        $this->groupBy = [
            'fields' => $fields,
            'aggregations' => $aggregations,
        ];
        return $this;
    }

    /**
     * Set the sort order for query results.
     *
     * @param string $field     Field name to sort by
     * @param string $direction Sort direction: `'asc'` (default) or `'desc'`
     *
     * @return self Chainable
     */
    public function orderBy(string $field, string $direction = 'asc'): self
    {
        $this->orderBy = [
            'field' => $field,
            'dir' => strtolower($direction),
        ];
        return $this;
    }

    /**
     * Limit the number of results returned.
     *
     * @param int $limit Maximum number of documents to return
     *
     * @return self Chainable
     *
     * @see offset() For pagination
     */
    public function limit(int $limit): self
    {
        $this->limit = $limit;
        return $this;
    }

    /**
     * Skip a number of results (for pagination).
     *
     * @param int $offset Number of documents to skip from the beginning
     *
     * @return self Chainable
     *
     * ```php
     * // Page 3, 20 items per page
     * $qb->orderBy('created_at', 'desc')
     *     ->offset(40)
     *     ->limit(20)
     *     ->get();
     * ```
     *
     * @see limit() To limit the number of results
     */
    public function offset(int $offset): self
    {
        $this->offset = $offset;
        return $this;
    }

    /**
     * Execute the query and return matching documents.
     *
     * When aggregations are present (via `sum()`, `avg()`, etc.), returns
     * a single-element array with the computed values instead of document rows.
     *
     * @return array<int, array<string, mixed>> Array of matching documents (or aggregation results)
     *
     * @throws AnvilDbException If the query fails
     * @throws \JsonException   If encoding/decoding fails
     *
     * ```php
     * $results = $collection->where('active', '=', true)
     *     ->orderBy('name')
     *     ->limit(10)
     *     ->get();
     * ```
     */
    public function get(): array
    {
        $spec = [
            'collection' => $this->collection,
            'filters' => $this->filters,
        ];

        if (!empty($this->joins)) {
            $spec['joins'] = $this->joins;
        }
        if (!empty($this->aggregations)) {
            $spec['aggregate'] = $this->aggregations;
        }
        if ($this->groupBy !== null) {
            $spec['group_by'] = $this->groupBy;
        }
        if ($this->orderBy !== null) {
            $spec['order_by'] = $this->orderBy;
        }
        if ($this->limit !== null) {
            $spec['limit'] = $this->limit;
        }
        if ($this->offset !== null) {
            $spec['offset'] = $this->offset;
        }

        $ffi = Bridge::get();
        $json = json_encode($spec, JSON_THROW_ON_ERROR);
        $resultPtr = $ffi->anvildb_query($this->handle, $json);

        if ($resultPtr === null) {
            $error = $ffi->anvildb_last_error($this->handle);
            $errorMsg = is_string($error) ? $error : ($error !== null ? \FFI::string($error) : 'Unknown query error');
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
     * Count the documents matching the current filters (without fetching them).
     *
     * More efficient than `count(get())` as it doesn't transfer document data.
     *
     * @return int Number of matching documents
     *
     * @throws AnvilDbException If the count fails
     * @throws \JsonException   If encoding fails
     *
     * ```php
     * $total = $collection->where('status', '=', 'active')->count();
     * ```
     */
    public function count(): int
    {
        $ffi = Bridge::get();
        $filterJson = !empty($this->filters) ? json_encode($this->filters, JSON_THROW_ON_ERROR) : null;

        $result = $ffi->anvildb_count($this->handle, $this->collection, $filterJson);

        if ($result < 0) {
            $error = $ffi->anvildb_last_error($this->handle);
            $errorMsg = is_string($error) ? $error : ($error !== null ? \FFI::string($error) : 'Unknown count error');
            throw new AnvilDbException($errorMsg);
        }

        return (int) $result;
    }
}
