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

$fmt = "  %-34s %7.1f ms  %s\n";

echo "  CRUD\n";
echo "  ──────────────────────────────────  ──────────  ──────────\n";

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
printf($fmt, "Bulk insert ({$batchSize}/batch)", $insertMs, "~{$insertThroughput} docs/s");

// Read all
$start = hrtime(true);
$all = $collection->all();
$readAllMs = (hrtime(true) - $start) / 1_000_000;
$readThroughput = round(count($all) / ($readAllMs / 1000));
printf($fmt, "Read all ({$records} docs)", $readAllMs, "~{$readThroughput} docs/s");

echo "\n  Queries\n";
echo "  ──────────────────────────────────  ──────────  ──────────\n";

// Filter equality
$start = hrtime(true);
$admins = $collection->where('role', '=', 'admin')->get();
$filterMs = (hrtime(true) - $start) / 1_000_000;
printf($fmt, "Filter (role = admin)", $filterMs, count($admins) . " results");

// Filter + sort + limit
$start = hrtime(true);
$complex = $collection->where('age', '>', 50)->orderBy('name', 'asc')->limit(100)->get();
$complexMs = (hrtime(true) - $start) / 1_000_000;
printf($fmt, "Filter + sort + limit(100)", $complexMs, count($complex) . " results");

// Between
$start = hrtime(true);
$between = $collection->whereBetween('age', 30, 50)->get();
$betweenMs = (hrtime(true) - $start) / 1_000_000;
printf($fmt, "whereBetween(age, 30, 50)", $betweenMs, count($between) . " results");

// In
$start = hrtime(true);
$in = $collection->whereIn('role', ['admin', 'editor'])->get();
$inMs = (hrtime(true) - $start) / 1_000_000;
printf($fmt, "whereIn(role, [admin,editor])", $inMs, count($in) . " results");

// Regex
$start = hrtime(true);
$regex = $collection->whereRegex('name', '^user_1[0-9]{2}$')->get();
$regexMs = (hrtime(true) - $start) / 1_000_000;
printf($fmt, "whereRegex(name, ^user_1xx$)", $regexMs, count($regex) . " results");

// Count with filter
$start = hrtime(true);
$activeCount = $collection->where('active', '=', true)->count();
$countMs = (hrtime(true) - $start) / 1_000_000;
printf($fmt, "Count with filter", $countMs, "{$activeCount} matching");

echo "\n  Aggregations\n";
echo "  ──────────────────────────────────  ──────────  ──────────\n";

// Aggregation (sum, avg, min, max) — via QueryBuilder
$start = hrtime(true);
$agg = $collection->where('active', '=', true)
    ->sum('score')->avg('score')->min('score')->max('score')->get();
$aggMs = (hrtime(true) - $start) / 1_000_000;
printf($fmt, "sum + avg + min + max (score)", $aggMs, "");

// Group by
$start = hrtime(true);
$grouped = $collection->where('active', '=', true)
    ->groupBy('role', [
        ['function' => 'count', 'alias' => 'total'],
        ['function' => 'avg', 'field' => 'age', 'alias' => 'avg_age'],
    ])->get();
$groupMs = (hrtime(true) - $start) / 1_000_000;
printf($fmt, "group_by(role) + count + avg", $groupMs, count($grouped) . " groups");

echo "\n  Indexes\n";
echo "  ──────────────────────────────────  ──────────  ──────────\n";

// Create range index
$start = hrtime(true);
$collection->createIndex('age', 'range');
$createIdxMs = (hrtime(true) - $start) / 1_000_000;
printf($fmt, "Create range index (age)", $createIdxMs, "");

// Query with range index (between)
$start = hrtime(true);
$rangeResult = $collection->whereBetween('age', 30, 50)->get();
$rangeMs = (hrtime(true) - $start) / 1_000_000;
printf($fmt, "whereBetween with range index", $rangeMs, count($rangeResult) . " results");

$db->close();
Bridge::reset();

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
