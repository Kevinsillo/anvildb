<?php

declare(strict_types=1);

namespace AnvilDb\Query;

use AnvilDb\Exception\AnvilDbException;
use AnvilDb\FFI\Bridge;

class QueryBuilder
{
    private \FFI\CData $handle;
    private string $collection;
    private array $filters = [];
    private ?array $orderBy = null;
    private ?int $limit = null;
    private ?int $offset = null;

    public function __construct(\FFI\CData $handle, string $collection)
    {
        $this->handle = $handle;
        $this->collection = $collection;
    }

    public function where(string $field, string $operator, mixed $value): self
    {
        $this->filters[] = [
            'field' => $field,
            'op' => $operator,
            'value' => $value,
        ];
        return $this;
    }

    public function orderBy(string $field, string $direction = 'asc'): self
    {
        $this->orderBy = [
            'field' => $field,
            'dir' => strtolower($direction),
        ];
        return $this;
    }

    public function limit(int $limit): self
    {
        $this->limit = $limit;
        return $this;
    }

    public function offset(int $offset): self
    {
        $this->offset = $offset;
        return $this;
    }

    public function get(): array
    {
        $spec = [
            'collection' => $this->collection,
            'filters' => $this->filters,
        ];

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
