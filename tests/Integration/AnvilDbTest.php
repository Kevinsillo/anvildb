<?php

declare(strict_types=1);

namespace AnvilDb\Tests\Integration;

use AnvilDb\AnvilDb;
use AnvilDb\Collection\Collection;
use AnvilDb\Exception\AnvilDbException;
use PHPUnit\Framework\TestCase;

class AnvilDbTest extends TestCase
{
    private string $dataPath;
    private ?AnvilDb $db = null;

    protected function setUp(): void
    {
        $this->dataPath = sys_get_temp_dir() . '/anvildb_test_' . uniqid();
        mkdir($this->dataPath, 0777, true);
        $this->db = new AnvilDb($this->dataPath);
    }

    protected function tearDown(): void
    {
        if ($this->db !== null) {
            $this->db->close();
            $this->db = null;
        }

        $this->removeDirectory($this->dataPath);
    }

    private function removeDirectory(string $dir): void
    {
        if (!is_dir($dir)) {
            return;
        }

        $items = scandir($dir);
        foreach ($items as $item) {
            if ($item === '.' || $item === '..') {
                continue;
            }
            $path = $dir . '/' . $item;
            if (is_dir($path)) {
                $this->removeDirectory($path);
            } else {
                unlink($path);
            }
        }
        rmdir($dir);
    }

    // -------------------------------------------------------
    // Full CRUD flow
    // -------------------------------------------------------

    public function testCreateCollectionAndInsertDocument(): void
    {
        $this->db->createCollection('users');
        $collection = $this->db->collection('users');

        $doc = $collection->insert(['name' => 'Alice', 'age' => 30]);

        $this->assertArrayHasKey('id', $doc);
        $this->assertSame('Alice', $doc['name']);
        $this->assertSame(30, $doc['age']);
    }

    public function testFindById(): void
    {
        $this->db->createCollection('users');
        $collection = $this->db->collection('users');

        $inserted = $collection->insert(['name' => 'Bob', 'email' => 'bob@test.com']);
        $id = $inserted['id'];

        $found = $collection->find($id);

        $this->assertNotNull($found);
        $this->assertSame($id, $found['id']);
        $this->assertSame('Bob', $found['name']);
        $this->assertSame('bob@test.com', $found['email']);
    }

    public function testFindByIdReturnsNullForMissingDocument(): void
    {
        $this->db->createCollection('users');
        $collection = $this->db->collection('users');

        $result = $collection->find('nonexistent-id');

        $this->assertNull($result);
    }

    public function testUpdateDocument(): void
    {
        $this->db->createCollection('users');
        $collection = $this->db->collection('users');

        $inserted = $collection->insert(['name' => 'Charlie', 'age' => 25]);
        $id = $inserted['id'];

        $result = $collection->update($id, ['name' => 'Charlie Updated', 'age' => 26]);
        $this->assertTrue($result);

        $updated = $collection->find($id);
        $this->assertSame('Charlie Updated', $updated['name']);
        $this->assertSame(26, $updated['age']);
    }

    public function testDeleteDocument(): void
    {
        $this->db->createCollection('users');
        $collection = $this->db->collection('users');

        $inserted = $collection->insert(['name' => 'Dave']);
        $id = $inserted['id'];

        $result = $collection->delete($id);
        $this->assertTrue($result);

        $found = $collection->find($id);
        $this->assertNull($found);
    }

    // -------------------------------------------------------
    // Query builder
    // -------------------------------------------------------

    public function testWhereFilter(): void
    {
        $this->db->createCollection('products');
        $collection = $this->db->collection('products');

        $collection->insert(['name' => 'Apple', 'price' => 1.5]);
        $collection->insert(['name' => 'Banana', 'price' => 0.75]);
        $collection->insert(['name' => 'Cherry', 'price' => 3.0]);

        $results = $collection->where('price', '>', 1.0)->get();

        $this->assertCount(2, $results);
        $names = array_column($results, 'name');
        $this->assertContains('Apple', $names);
        $this->assertContains('Cherry', $names);
    }

    public function testWhereEqualsFilter(): void
    {
        $this->db->createCollection('products');
        $collection = $this->db->collection('products');

        $collection->insert(['name' => 'Apple', 'category' => 'fruit']);
        $collection->insert(['name' => 'Carrot', 'category' => 'vegetable']);
        $collection->insert(['name' => 'Banana', 'category' => 'fruit']);

        $results = $collection->where('category', '=', 'fruit')->get();

        $this->assertCount(2, $results);
        $names = array_column($results, 'name');
        $this->assertContains('Apple', $names);
        $this->assertContains('Banana', $names);
    }

    public function testOrderBy(): void
    {
        $this->db->createCollection('items');
        $collection = $this->db->collection('items');

        $collection->insert(['name' => 'C', 'priority' => 3]);
        $collection->insert(['name' => 'A', 'priority' => 1]);
        $collection->insert(['name' => 'B', 'priority' => 2]);

        $results = $collection->orderBy('priority', 'asc')->get();

        $this->assertCount(3, $results);
        $this->assertSame('A', $results[0]['name']);
        $this->assertSame('B', $results[1]['name']);
        $this->assertSame('C', $results[2]['name']);
    }

    public function testOrderByDescending(): void
    {
        $this->db->createCollection('items');
        $collection = $this->db->collection('items');

        $collection->insert(['name' => 'C', 'priority' => 3]);
        $collection->insert(['name' => 'A', 'priority' => 1]);
        $collection->insert(['name' => 'B', 'priority' => 2]);

        $results = $collection->orderBy('priority', 'desc')->get();

        $this->assertCount(3, $results);
        $this->assertSame('C', $results[0]['name']);
        $this->assertSame('B', $results[1]['name']);
        $this->assertSame('A', $results[2]['name']);
    }

    public function testLimit(): void
    {
        $this->db->createCollection('items');
        $collection = $this->db->collection('items');

        $collection->insert(['name' => 'A', 'priority' => 1]);
        $collection->insert(['name' => 'B', 'priority' => 2]);
        $collection->insert(['name' => 'C', 'priority' => 3]);

        $results = $collection->orderBy('priority', 'asc')->limit(2)->get();

        $this->assertCount(2, $results);
        $this->assertSame('A', $results[0]['name']);
        $this->assertSame('B', $results[1]['name']);
    }

    public function testOffset(): void
    {
        $this->db->createCollection('items');
        $collection = $this->db->collection('items');

        $collection->insert(['name' => 'A', 'priority' => 1]);
        $collection->insert(['name' => 'B', 'priority' => 2]);
        $collection->insert(['name' => 'C', 'priority' => 3]);

        $results = $collection->orderBy('priority', 'asc')->offset(1)->limit(2)->get();

        $this->assertCount(2, $results);
        $this->assertSame('B', $results[0]['name']);
        $this->assertSame('C', $results[1]['name']);
    }

    public function testWhereWithOrderByAndLimit(): void
    {
        $this->db->createCollection('products');
        $collection = $this->db->collection('products');

        $collection->insert(['name' => 'A', 'price' => 10]);
        $collection->insert(['name' => 'B', 'price' => 20]);
        $collection->insert(['name' => 'C', 'price' => 30]);
        $collection->insert(['name' => 'D', 'price' => 40]);

        $results = $collection
            ->where('price', '>=', 20)
            ->orderBy('price', 'desc')
            ->limit(2)
            ->get();

        $this->assertCount(2, $results);
        $this->assertSame('D', $results[0]['name']);
        $this->assertSame('C', $results[1]['name']);
    }

    // -------------------------------------------------------
    // Bulk insert
    // -------------------------------------------------------

    public function testBulkInsert(): void
    {
        $this->db->createCollection('logs');
        $collection = $this->db->collection('logs');

        $docs = [
            ['message' => 'Log 1', 'level' => 'info'],
            ['message' => 'Log 2', 'level' => 'warn'],
            ['message' => 'Log 3', 'level' => 'error'],
        ];

        $results = $collection->bulkInsert($docs);

        $this->assertCount(3, $results);
        foreach ($results as $doc) {
            $this->assertArrayHasKey('id', $doc);
        }

        $all = $collection->all();
        $this->assertCount(3, $all);
    }

    // -------------------------------------------------------
    // List collections
    // -------------------------------------------------------

    public function testListCollections(): void
    {
        $this->db->createCollection('alpha');
        $this->db->createCollection('beta');
        $this->db->createCollection('gamma');

        $collections = $this->db->listCollections();

        $this->assertContains('alpha', $collections);
        $this->assertContains('beta', $collections);
        $this->assertContains('gamma', $collections);
    }

    public function testDropCollection(): void
    {
        $this->db->createCollection('temp');
        $collections = $this->db->listCollections();
        $this->assertContains('temp', $collections);

        $this->db->dropCollection('temp');
        $collections = $this->db->listCollections();
        $this->assertNotContains('temp', $collections);
    }

    // -------------------------------------------------------
    // Count
    // -------------------------------------------------------

    public function testCount(): void
    {
        $this->db->createCollection('items');
        $collection = $this->db->collection('items');

        $this->assertSame(0, $collection->count());

        $collection->insert(['name' => 'A']);
        $collection->insert(['name' => 'B']);
        $collection->insert(['name' => 'C']);

        $this->assertSame(3, $collection->count());
    }

    public function testCountAfterDelete(): void
    {
        $this->db->createCollection('items');
        $collection = $this->db->collection('items');

        $doc = $collection->insert(['name' => 'A']);
        $collection->insert(['name' => 'B']);

        $this->assertSame(2, $collection->count());

        $collection->delete($doc['id']);

        $this->assertSame(1, $collection->count());
    }

    // -------------------------------------------------------
    // Indexes
    // -------------------------------------------------------

    public function testCreateAndDropIndex(): void
    {
        $this->db->createCollection('indexed');
        $collection = $this->db->collection('indexed');

        // Should not throw
        $collection->createIndex('email', 'hash');

        // Insert and query should still work with index
        $collection->insert(['email' => 'test@example.com', 'name' => 'Test']);
        $results = $collection->where('email', '=', 'test@example.com')->get();
        $this->assertCount(1, $results);
        $this->assertSame('Test', $results[0]['name']);

        // Drop index should not throw
        $collection->dropIndex('email');
    }

    // -------------------------------------------------------
    // Schema validation
    // -------------------------------------------------------

    public function testSchemaValidationAcceptsValidDocument(): void
    {
        $this->db->createCollection('strict');
        $collection = $this->db->collection('strict');

        $schema = [
            'name' => 'string',
            'age' => 'int',
        ];

        $collection->setSchema($schema);

        $doc = $collection->insert(['name' => 'Valid User', 'age' => 25]);
        $this->assertArrayHasKey('id', $doc);
        $this->assertSame('Valid User', $doc['name']);
    }

    public function testSchemaValidationRejectsInvalidDocument(): void
    {
        $this->db->createCollection('strict');
        $collection = $this->db->collection('strict');

        $schema = [
            'name' => 'string',
            'age' => 'int',
        ];

        $collection->setSchema($schema);

        $this->expectException(AnvilDbException::class);

        // Providing wrong type for 'age' should fail validation
        $collection->insert(['name' => 'Invalid User', 'age' => 'not_a_number']);
    }

    // -------------------------------------------------------
    // All documents
    // -------------------------------------------------------

    public function testAllReturnsAllDocuments(): void
    {
        $this->db->createCollection('data');
        $collection = $this->db->collection('data');

        $collection->insert(['key' => 'val1']);
        $collection->insert(['key' => 'val2']);

        $all = $collection->all();
        $this->assertCount(2, $all);
    }

    // -------------------------------------------------------
    // Edge cases
    // -------------------------------------------------------

    public function testClosePreventsFurtherOperations(): void
    {
        $db = new AnvilDb($this->dataPath);
        $db->createCollection('test_close');
        $db->close();

        $this->expectException(AnvilDbException::class);
        $db->createCollection('should_fail');
    }
}
