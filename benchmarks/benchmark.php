<?php

/**
 * AnvilDB Benchmark — measures AnvilDB performance.
 *
 * Usage:
 *   php benchmarks/benchmark.php [records]
 *
 * Default: 10,000 records
 */

declare(strict_types=1);

require __DIR__ . '/../vendor/autoload.php';

use AnvilDb\AnvilDb;
use AnvilDb\FFI\Bridge;

$records = (int) ($argv[1] ?? 10_000);

echo "===========================================\n";
echo " AnvilDB Benchmark — $records records\n";
echo "===========================================\n\n";

$tmpBase = sys_get_temp_dir() . '/anvildb_bench_' . uniqid();

// ─── Generate test data ───────────────────────────────────
$testData = [];
for ($i = 0; $i < $records; $i++) {
    $testData[] = [
        'name'   => "user_$i",
        'email'  => "user_$i@example.com",
        'age'    => ($i % 80) + 18,
        'role'   => ['admin', 'user', 'editor', 'viewer'][$i % 4],
        'active' => $i % 3 !== 0,
        'score'  => round($i * 0.7, 2),
    ];
}

// ─── Benchmark ───────────────────────────────────────────
$anvilPath = $tmpBase . '/anvildb';
mkdir($anvilPath, 0777, true);

$db = new AnvilDb($anvilPath);
$db->createCollection('bench');
$collection = $db->collection('bench');

// Bulk insert
$batchSize = 1000;
$start = hrtime(true);
for ($i = 0; $i < $records; $i += $batchSize) {
    $batch = array_slice($testData, $i, $batchSize);
    $collection->bulkInsert($batch);
}
$db->flush();
$insertMs = (hrtime(true) - $start) / 1_000_000;
$insertThroughput = round($records / ($insertMs / 1000));

// Read all
$start = hrtime(true);
$all = $collection->all();
$readAllMs = (hrtime(true) - $start) / 1_000_000;
$readThroughput = round(count($all) / ($readAllMs / 1000));

// Filter query
$start = hrtime(true);
$admins = $collection->where('role', '=', 'admin')->get();
$filterMs = (hrtime(true) - $start) / 1_000_000;

// Filter + sort + limit
$start = hrtime(true);
$complex = $collection
    ->where('age', '>', 50)
    ->orderBy('name', 'asc')
    ->limit(100)
    ->get();
$complexMs = (hrtime(true) - $start) / 1_000_000;

// Count with filter
$start = hrtime(true);
$activeCount = $collection->where('active', '=', true)->count();
$countMs = (hrtime(true) - $start) / 1_000_000;

$db->close();
Bridge::reset();

// ─── Results ─────────────────────────────────────────────
echo "  Operation                    Time        Throughput\n";
echo "  ───────────────────────────  ──────────  ──────────\n";

$fmt = "  %-29s %7.1f ms  %s\n";

printf($fmt, "Bulk insert ({$batchSize}/batch)", $insertMs, "~{$insertThroughput} docs/s");
printf($fmt, "Read all ({$records} docs)", $readAllMs, "~{$readThroughput} docs/s");
printf($fmt, "Filter (role = admin)", $filterMs, count($admins) . " results");
printf($fmt, "Filter + sort + limit(100)", $complexMs, count($complex) . " results");
printf($fmt, "Count with filter", $countMs, "{$activeCount} matching");

echo "\n  All operations include: compression, atomic writes,\n";
echo "  schema validation, and index enforcement.\n";

// ─── Cleanup ─────────────────────────────────────────────
function rmrf(string $dir): void
{
    if (!is_dir($dir)) return;
    foreach (scandir($dir) as $item) {
        if ($item === '.' || $item === '..') continue;
        $path = $dir . '/' . $item;
        is_dir($path) ? rmrf($path) : unlink($path);
    }
    rmdir($dir);
}
rmrf($tmpBase);

echo "\nDone.\n";
