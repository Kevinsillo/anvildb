# API Reference (PHP)

[< Back to README](../README.md)

## AnvilDb (Main Facade)

```php
use AnvilDb\AnvilDb;
```

### `__construct(string $dataPath, ?string $encryptionKey = null)`

Opens the database engine. Creates the data directory if it doesn't exist. Pass a 64-character hex string to open an encrypted database.

```php
$db = new AnvilDb('/path/to/data');
$db = new AnvilDb('/path/to/data', 'aabbccdd...64chars...');
```

### `close(): void`

Closes the engine and frees resources. Called automatically on destruction.

### `shutdown(): void`

Flushes all pending write buffers and closes the engine. The handle is no longer usable after this call.

### `flush(): void`

Flushes all pending buffered writes to disk across all collections.

### `configureBuffer(int $maxDocs = 100, int $flushIntervalSecs = 5): void`

Configures the write buffer. `$maxDocs` is the per-collection threshold that triggers an auto-flush. `$flushIntervalSecs` is the background timer interval.

```php
$db->configureBuffer(200, 10); // flush every 200 docs or 10 seconds
```

### `collection(string $name): Collection`

Returns a `Collection` instance for the given name.

### `createCollection(string $name): void`

Creates a new collection. Throws `AnvilDbException` on failure.

### `dropCollection(string $name): void`

Drops a collection and deletes its data file.

### `listCollections(): array`

Returns an array of collection names.

### `encrypt(string $key): void`

Encrypts an unencrypted database. Rewrites all collection and index files with AES-256-GCM. `$key` is a 64-character hex string (32 bytes).

### `decrypt(string $key): void`

Decrypts an encrypted database. Rewrites all files without encryption.

### `clearCache(): void`

Clears the internal LRU cache.

---

## Collection

```php
use AnvilDb\Collection\Collection;
```

### `insert(array $document): array`

Inserts a document. Auto-generates a UUID `id` if not provided. Returns the inserted document with `id`.

```php
$doc = $collection->insert(['name' => 'Alice', 'age' => 25]);
echo $doc['id']; // "550e8400-e29b-41d4-a716-446655440000"
```

### `bulkInsert(array $documents): array`

Inserts multiple documents. Returns array of inserted documents with IDs.

```php
$docs = $collection->bulkInsert([
    ['name' => 'Alice'],
    ['name' => 'Bob'],
]);
```

### `find(string $id): ?array`

Finds a document by ID. Returns `null` if not found.

### `update(string $id, array $data): bool`

Replaces the document with the given data (preserving the ID). Returns `true` on success.

### `delete(string $id): bool`

Deletes a document by ID. Returns `true` on success.

### `flush(): void`

Flushes pending buffered writes for this collection to disk.

### `join(string $collection, string $leftField, string $rightField, string $type = 'inner', ?string $prefix = null): QueryBuilder`

Starts a query chain with a join. Joined fields are prefixed (defaults to `{collection}_`).

```php
$collection->join('users', 'user_id', 'id', 'inner', 'user_')->get();
```

### `leftJoin(string $collection, string $leftField, string $rightField, ?string $prefix = null): QueryBuilder`

Shorthand for `join()` with `$type = 'left'`.

### `where(string $field, string $operator, mixed $value): QueryBuilder`

Starts a query chain with a filter condition.

### `orderBy(string $field, string $direction = 'asc'): QueryBuilder`

Starts a query chain with sorting.

### `all(): array`

Returns all documents in the collection.

### `count(): int`

Returns the total number of documents.

### `exportCsv(string $filePath, ?array $fields = null): int`

Exports the collection to a CSV file. Returns the number of exported documents. If `$fields` is null, infers columns from the first document.

### `importCsv(string $filePath): int`

Imports documents from a CSV file (first row = headers). Returns the number of imported documents. Inserts in batches of 1000.

### `createIndex(string $field, string $type = 'hash'): void`

Creates an index on a field. Types: `hash`, `unique`, `range`.

### `dropIndex(string $field): void`

Drops an index on a field.

### `setSchema(array $schema): void`

Sets a validation schema. Types: `string`, `int`, `float`, `bool`, `array`, `object`.

```php
$collection->setSchema([
    'name' => 'string',
    'age' => 'int',
]);
```

---

## QueryBuilder

```php
use AnvilDb\Query\QueryBuilder;
```

### `join(string $collection, string $leftField, string $rightField, string $type = 'inner', ?string $prefix = null): self`

Adds a join clause. Multiple joins can be chained. `$type` is `inner` or `left`. `$prefix` defaults to `{collection}_`.

```php
$qb->join('users', 'user_id', 'id', 'inner', 'u_')
    ->join('products', 'product_id', 'id', 'inner', 'p_')
    ->where('u_status', '=', 'active')
    ->get();
```

Filters, sorting, and pagination apply **after** all joins, so you can reference prefixed fields.

### `leftJoin(string $collection, string $leftField, string $rightField, ?string $prefix = null): self`

Shorthand for `join()` with `$type = 'left'`. Unmatched left rows are included without right-side fields.

### `where(string $field, string $operator, mixed $value): self`

Adds a filter. Chainable. Operators: `=`, `!=`, `>`, `<`, `>=`, `<=`, `contains`, `between`, `in`, `not_in`.

### `whereBetween(string $field, mixed $min, mixed $max): self`

Shorthand for `where($field, 'between', [$min, $max])`. Inclusive on both ends.

### `whereIn(string $field, array $values): self`

Matches documents where the field value is in the given array.

### `whereNotIn(string $field, array $values): self`

Matches documents where the field value is NOT in the given array.

### `sum(string $field, ?string $alias = null): self`
### `avg(string $field, ?string $alias = null): self`
### `min(string $field, ?string $alias = null): self`
### `max(string $field, ?string $alias = null): self`

Adds an aggregation. When any aggregation is present, `get()` returns a single result object with the computed values instead of document rows.

### `groupBy(string|array $fields, array $aggregations = []): self`

Groups results by the given field(s) and applies aggregations per group. Each aggregation is `['function' => '...', 'field' => '...', 'alias' => '...']`.

### `orderBy(string $field, string $direction = 'asc'): self`

Sets sort order. Direction: `asc` or `desc`.

### `limit(int $limit): self`

Limits the number of results.

### `offset(int $offset): self`

Skips the first N results.

### `get(): array`

Executes the query and returns matching documents.

### `count(): int`

Returns the count of matching documents.

---

## Exceptions

### `AnvilDb\Exception\AnvilDbException`

Base exception for all database errors (validation failures, missing documents, etc.).

### `AnvilDb\Exception\FFIException`

Thrown when the FFI bridge fails to load (missing `.so`, FFI disabled, unsupported platform).
