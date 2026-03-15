<?php

/**
 * AnvilDB Benchmark — compares AnvilDB (Rust FFI) vs pure PHP json_encode/json_decode.
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

// ─── AnvilDB Benchmark ───────────────────────────────────
$anvilPath = $tmpBase . '/anvildb';
mkdir($anvilPath, 0777, true);

$db = new AnvilDb($anvilPath);
$db->createCollection('bench');
$collection = $db->collection('bench');

// Insert
$start = hrtime(true);
$batchSize = 1000;
for ($i = 0; $i < $records; $i += $batchSize) {
    $batch = array_slice($testData, $i, $batchSize);
    $collection->bulkInsert($batch);
}
$anvilInsertMs = (hrtime(true) - $start) / 1_000_000;

// Query: filter
$start = hrtime(true);
$admins = $collection->where('role', '=', 'admin')->get();
$anvilQueryMs = (hrtime(true) - $start) / 1_000_000;

// Query: filter + sort + limit
$start = hrtime(true);
$complex = $collection
    ->where('age', '>', 50)
    ->orderBy('name', 'asc')
    ->limit(100)
    ->get();
$anvilComplexMs = (hrtime(true) - $start) / 1_000_000;

// Count with filter
$start = hrtime(true);
$activeCount = $collection->where('active', '=', true)->count();
$anvilCountMs = (hrtime(true) - $start) / 1_000_000;

// Read all
$start = hrtime(true);
$all = $collection->all();
$anvilReadAllMs = (hrtime(true) - $start) / 1_000_000;

$db->close();
Bridge::reset();

// ─── Pure PHP Benchmark ──────────────────────────────────
$phpPath = $tmpBase . '/purephp';
mkdir($phpPath, 0777, true);
$phpFile = $phpPath . '/bench.json';

// Insert (write all at once — best case for PHP)
$start = hrtime(true);
$phpDocs = [];
foreach ($testData as $i => $doc) {
    $doc['id'] = sprintf('%08d', $i);
    $phpDocs[] = $doc;
}
file_put_contents($phpFile, json_encode($phpDocs, JSON_THROW_ON_ERROR));
$phpInsertMs = (hrtime(true) - $start) / 1_000_000;

// Read all + decode
$start = hrtime(true);
$phpAll = json_decode(file_get_contents($phpFile), true, 512, JSON_THROW_ON_ERROR);
$phpReadAllMs = (hrtime(true) - $start) / 1_000_000;

// Query: filter (manual array_filter)
$start = hrtime(true);
$phpAdmins = array_values(array_filter($phpAll, fn($d) => $d['role'] === 'admin'));
$phpQueryMs = (hrtime(true) - $start) / 1_000_000;

// Query: filter + sort + limit (manual)
$start = hrtime(true);
$phpComplex = array_values(array_filter($phpAll, fn($d) => $d['age'] > 50));
usort($phpComplex, fn($a, $b) => strcmp($a['name'], $b['name']));
$phpComplex = array_slice($phpComplex, 0, 100);
$phpComplexMs = (hrtime(true) - $start) / 1_000_000;

// Count with filter
$start = hrtime(true);
$phpActiveCount = count(array_filter($phpAll, fn($d) => $d['active'] === true));
$phpCountMs = (hrtime(true) - $start) / 1_000_000;

// ─── Results ─────────────────────────────────────────────
$fmt = "  %-22s %10.1f ms  %10.1f ms  %7s\n";

echo "  Operation              AnvilDB (ms)  Pure PHP (ms)  Winner\n";
echo "  ─────────────────────  ────────────  ─────────────  ──────\n";

$ops = [
    ['Bulk insert',          $anvilInsertMs,  $phpInsertMs],
    ['Read all',             $anvilReadAllMs, $phpReadAllMs],
    ['Filter (= admin)',     $anvilQueryMs,   $phpQueryMs],
    ['Filter + sort + limit',$anvilComplexMs, $phpComplexMs],
    ['Count with filter',    $anvilCountMs,   $phpCountMs],
];

foreach ($ops as [$name, $anvil, $php]) {
    $winner = $anvil < $php ? 'AnvilDB' : 'PHP';
    $ratio = $anvil < $php
        ? sprintf('%.1fx', $php / $anvil)
        : sprintf('%.1fx', $anvil / $php);
    printf($fmt, $name, $anvil, $php, $winner);
}

echo "\n  Records: $records | AnvilDB admins: " . count($admins) . " | PHP admins: " . count($phpAdmins) . "\n";

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
