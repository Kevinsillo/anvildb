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
     * Add a filter condition.
     *
     * @param string $field    Field name
     * @param string $operator Comparison operator (e.g. '=', '!=', '>', '<', '>=', '<=')
     * @param mixed  $value    Value to compare against
     *
     * @return self
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
     * Add a join to another collection.
     *
     * @param string      $collection Target collection name
     * @param string      $leftField  Field on the current collection
     * @param string      $rightField Field on the target collection
     * @param string      $type       Join type ('inner', 'left')
     * @param string|null $prefix     Optional prefix for joined fields
     *
     * @return self
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
     * @param string      $collection Target collection name
     * @param string      $leftField  Field on the current collection
     * @param string      $rightField Field on the target collection
     * @param string|null $prefix     Optional prefix for joined fields
     *
     * @return self
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
     * Add a between filter (inclusive range).
     *
     * @param string $field Field name
     * @param mixed  $min   Minimum value
     * @param mixed  $max   Maximum value
     *
     * @return self
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
     * @param array<mixed> $values Allowed values
     *
     * @return self
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
     * @param array<mixed> $values Excluded values
     *
     * @return self
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
     * Add a regex filter.
     *
     * @param string $field   Field name
     * @param string $pattern Regular expression pattern
     *
     * @return self
     */
    public function whereRegex(string $field, string $pattern): self
    {
        $this->filters[] = [
            'field' => $field,
            'op' => 'regex',
            'value' => $pattern,
        ];
        return $this;
    }

    /**
     * Add a SUM aggregation.
     *
     * @param string      $field Field to sum
     * @param string|null $alias Optional alias for the result
     *
     * @return self
     */
    public function sum(string $field, ?string $alias = null): self
    {
        $this->aggregations[] = ['function' => 'sum', 'field' => $field, 'alias' => $alias];
        return $this;
    }

    /**
     * Add an AVG aggregation.
     *
     * @param string      $field Field to average
     * @param string|null $alias Optional alias for the result
     *
     * @return self
     */
    public function avg(string $field, ?string $alias = null): self
    {
        $this->aggregations[] = ['function' => 'avg', 'field' => $field, 'alias' => $alias];
        return $this;
    }

    /**
     * Add a MIN aggregation.
     *
     * @param string      $field Field to find minimum of
     * @param string|null $alias Optional alias for the result
     *
     * @return self
     */
    public function min(string $field, ?string $alias = null): self
    {
        $this->aggregations[] = ['function' => 'min', 'field' => $field, 'alias' => $alias];
        return $this;
    }

    /**
     * Add a MAX aggregation.
     *
     * @param string      $field Field to find maximum of
     * @param string|null $alias Optional alias for the result
     *
     * @return self
     */
    public function max(string $field, ?string $alias = null): self
    {
        $this->aggregations[] = ['function' => 'max', 'field' => $field, 'alias' => $alias];
        return $this;
    }

    /**
     * Group results by one or more fields.
     *
     * @param string|array<string>        $fields       Field(s) to group by
     * @param array<array<string, mixed>> $aggregations Aggregation definitions for grouped results
     *
     * @return self
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
     * @param string $field     Field to sort by
     * @param string $direction Sort direction ('asc' or 'desc')
     *
     * @return self
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
     * @param int $limit Maximum number of documents
     *
     * @return self
     */
    public function limit(int $limit): self
    {
        $this->limit = $limit;
        return $this;
    }

    /**
     * Skip a number of results (for pagination).
     *
     * @param int $offset Number of documents to skip
     *
     * @return self
     */
    public function offset(int $offset): self
    {
        $this->offset = $offset;
        return $this;
    }

    /**
     * Execute the query and return matching documents.
     *
     * @return array<int, array<string, mixed>> Array of matching documents
     *
     * @throws AnvilDbException If the query fails
     * @throws \JsonException   If encoding/decoding fails
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
     * Count the documents matching the current filters.
     *
     * @return int Number of matching documents
     *
     * @throws AnvilDbException If the count fails
     * @throws \JsonException   If encoding fails
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
