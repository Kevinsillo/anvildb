# API Reference

## AnvilDb (Main Facade)

```php
use AnvilDb\AnvilDb;
```

### `__construct(string $dataPath)`

Opens the database engine. Creates the data directory if it doesn't exist.

```php
$db = new AnvilDb('/path/to/data');
```

### `close(): void`

Closes the engine and frees resources. Called automatically on destruction.

### `shutdown(): void`

Flushes all pending write buffers. Call before `close()` in long-running processes.

### `collection(string $name): Collection`

Returns a `Collection` instance for the given name.

### `createCollection(string $name): void`

Creates a new collection. Throws `AnvilDbException` on failure.

### `dropCollection(string $name): void`

Drops a collection and deletes its data file.

### `listCollections(): array`

Returns an array of collection names.

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

### `where(string $field, string $operator, mixed $value): QueryBuilder`

Starts a query chain with a filter condition.

### `orderBy(string $field, string $direction = 'asc'): QueryBuilder`

Starts a query chain with sorting.

### `all(): array`

Returns all documents in the collection.

### `count(): int`

Returns the total number of documents.

### `createIndex(string $field, string $type = 'hash'): void`

Creates an index on a field. Types: `hash`, `unique`.

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

### `where(string $field, string $operator, mixed $value): self`

Adds a filter. Chainable. Operators: `=`, `!=`, `>`, `<`, `>=`, `<=`, `contains`.

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
