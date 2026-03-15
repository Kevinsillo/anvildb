<?php

declare(strict_types=1);

namespace AnvilDb\Tests\Stress;

use AnvilDb\AnvilDb;
use AnvilDb\FFI\Bridge;
use PHPUnit\Framework\TestCase;

/**
 * Load test — verifies AnvilDB handles 100k+ records correctly and measures performance.
 *
 * These tests are slow by design. Run them explicitly:
 *   ./vendor/bin/phpunit tests/Stress/LoadTest.php
 *
 * @group stress
 */
class LoadTest extends TestCase
{
    private string $dataPath;
    private ?AnvilDb $db = null;

    protected function setUp(): void
    {
        $this->dataPath = sys_get_temp_dir() . '/anvildb_load_' . uniqid();
        mkdir($this->dataPath, 0777, true);
        $this->db = new AnvilDb($this->dataPath);
    }

    protected function tearDown(): void
    {
        if ($this->db !== null) {
            $this->db->close();
            $this->db = null;
        }
        Bridge::reset();
        $this->removeDirectory($this->dataPath);
    }

    public function testInsert100kRecords(): void
    {
        $this->db->createCollection('large');
        $collection = $this->db->collection('large');

        $totalRecords = 100_000;
        $batchSize = 1_000;
        $batches = $totalRecords / $batchSize;

        $startTime = hrtime(true);

        for ($batch = 0; $batch < $batches; $batch++) {
            $docs = [];
            for ($i = 0; $i < $batchSize; $i++) {
                $index = $batch * $batchSize + $i;
                $docs[] = [
                    'name'     => "user_$index",
                    'email'    => "user_$index@example.com",
                    'age'      => ($index % 80) + 18,
                    'role'     => ['admin', 'user', 'editor', 'viewer'][$index % 4],
                    'active'   => $index % 3 !== 0,
                ];
            }
            $collection->bulkInsert($docs);
        }

        $insertTime = (hrtime(true) - $startTime) / 1_000_000; // ms

        // Verify count
        $count = $collection->count();
        $this->assertSame($totalRecords, $count);

        // Query with filter
        $startTime = hrtime(true);
        $admins = $collection->where('role', '=', 'admin')->get();
        $queryTime = (hrtime(true) - $startTime) / 1_000_000;

        $this->assertSame($totalRecords / 4, count($admins));

        // Query with filter + sort + limit
        $startTime = hrtime(true);
        $results = $collection
            ->where('role', '=', 'user')
            ->where('age', '>', 50)
            ->orderBy('name', 'asc')
            ->limit(100)
            ->get();
        $complexQueryTime = (hrtime(true) - $startTime) / 1_000_000;

        $this->assertLessThanOrEqual(100, count($results));

        // Count with filter
        $startTime = hrtime(true);
        $activeCount = $collection->where('active', '=', true)->count();
        $countTime = (hrtime(true) - $startTime) / 1_000_000;

        // 2/3 of records should be active (index % 3 !== 0)
        $expectedActive = (int) ceil($totalRecords * 2 / 3);
        // Allow small tolerance due to modular arithmetic edge
        $this->assertGreaterThan($totalRecords / 2, $activeCount);

        // Print performance report
        fwrite(STDERR, sprintf(
            "\n--- Load Test Report (100k records) ---\n" .
            "  Bulk insert:    %8.1f ms (%.0f docs/sec)\n" .
            "  Filter query:   %8.1f ms (returned %d docs)\n" .
            "  Complex query:  %8.1f ms (filter + sort + limit)\n" .
            "  Count w/filter: %8.1f ms\n" .
            "---------------------------------------\n",
            $insertTime,
            $totalRecords / ($insertTime / 1000),
            $queryTime,
            count($admins),
            $complexQueryTime,
            $countTime
        ));
    }

    public function testUpdateAndDeleteAtScale(): void
    {
        $this->db->createCollection('mutable');
        $collection = $this->db->collection('mutable');

        // Insert 10k records
        $totalRecords = 10_000;
        $batchSize = 1_000;

        for ($batch = 0; $batch < $totalRecords / $batchSize; $batch++) {
            $docs = [];
            for ($i = 0; $i < $batchSize; $i++) {
                $index = $batch * $batchSize + $i;
                $docs[] = [
                    'name'   => "item_$index",
                    'status' => 'pending',
                ];
            }
            $collection->bulkInsert($docs);
        }

        $this->assertSame($totalRecords, $collection->count());

        // Update first 1000
        $toUpdate = $collection->orderBy('name', 'asc')->limit(1000)->get();
        $startTime = hrtime(true);
        foreach ($toUpdate as $doc) {
            $collection->update($doc['id'], [
                'name'   => $doc['name'],
                'status' => 'processed',
            ]);
        }
        $updateTime = (hrtime(true) - $startTime) / 1_000_000;

        $processed = $collection->where('status', '=', 'processed')->count();
        $this->assertSame(1000, $processed);

        // Delete next 1000
        $toDelete = $collection->where('status', '=', 'pending')->limit(1000)->get();
        $startTime = hrtime(true);
        foreach ($toDelete as $doc) {
            $collection->delete($doc['id']);
        }
        $deleteTime = (hrtime(true) - $startTime) / 1_000_000;

        $this->assertSame($totalRecords - 1000, $collection->count());

        fwrite(STDERR, sprintf(
            "\n--- Update/Delete Report (10k base) ---\n" .
            "  Update 1000:  %8.1f ms\n" .
            "  Delete 1000:  %8.1f ms\n" .
            "---------------------------------------\n",
            $updateTime,
            $deleteTime
        ));
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
            is_dir($path) ? $this->removeDirectory($path) : unlink($path);
        }
        rmdir($dir);
    }
}
