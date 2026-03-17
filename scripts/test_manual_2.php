<?php

declare(strict_types=1);

require_once __DIR__ . '/../vendor/autoload.php';

use AnvilDb\AnvilDb;

$dataPath = __DIR__ . '/../data';

// Limpiar datos previos
if (is_dir($dataPath)) {
    $files = new RecursiveIteratorIterator(
        new RecursiveDirectoryIterator($dataPath, FilesystemIterator::SKIP_DOTS),
        RecursiveIteratorIterator::CHILD_FIRST
    );
    foreach ($files as $f) {
        $f->isDir() ? rmdir($f->getPathname()) : unlink($f->getPathname());
    }
    rmdir($dataPath);
}

echo "=== AnvilDB Manual Test ===\n";
echo "Data path: {$dataPath}\n\n";

$encryptionKey = 'a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2';
$db = new AnvilDb($dataPath, $encryptionKey);

// ─────────────────────────────────────────────
// 1. CRUD básico
// ─────────────────────────────────────────────
echo "── 1. CRUD básico ──\n";

$db->createCollection('users');
$db->createCollection('roles');

$alice = $db->collection('users')->insert(['name' => 'Alice', 'email' => 'alice@mail.com', 'role_id' => '1']);

echo "Insertados 1 usuario: Alice ({$alice['id']})\n";

// Find
$found = $db->collection('users')->find($alice['id']);
echo "Find Alice: {$found['name']} — {$found['email']}\n";

// Update
$db->collection('users')->update($alice['id'], ['name' => 'Alicia', 'email' => 'alicia@mail.com', 'role_id' => '2']);
$updated = $db->collection('users')->find($alice['id']);
echo "Update Alice → Alicia: {$updated['name']} — {$updated['email']}\n";

// Count
$count = $db->collection('users')->count();
echo "Count users: {$count}\n";

// ─────────────────────────────────────────────
// 2. Índices
// ─────────────────────────────────────────────
echo "\n── 2. Índices ──\n";

// Unique index en email — no permite duplicados
$db->collection('users')->createIndex('email', 'unique');
echo "Creado unique index en users.email\n";

// Hash index en status — acelera búsquedas por igualdad
$db->collection('users')->createIndex('status', 'hash');
echo "Creado hash index en users.status\n";

// Probar que el unique index funciona — intentar duplicar email
try {
    $db->collection('users')->insert(['name' => 'Fake Alice', 'email' => 'alice@mail.com', 'status' => 'active']);
    echo "ERROR: debería haber fallado por email duplicado!\n";
} catch (\Exception $e) {
    echo "Unique index OK — rechazó email duplicado: {$e->getMessage()}\n";
}

// Query usando el campo indexado
$activos = $db->collection('users')->where('status', '=', 'active')->get();
echo "Query con hash index (status=active): " . count($activos) . " resultados\n";

// ─────────────────────────────────────────────
// 3. Queries con filtros
// ─────────────────────────────────────────────
echo "\n── 3. Queries con filtros ──\n";

$activeUsers = $db->collection('users')
    ->where('status', '=', 'active')
    ->orderBy('name', 'asc')
    ->get();

echo "Usuarios activos (asc): ";
echo implode(', ', array_map(fn($u) => $u['name'], $activeUsers)) . "\n";

// ─────────────────────────────────────────────
// 4. Bulk insert
// ─────────────────────────────────────────────
echo "\n── 4. Bulk insert ──\n";

$products = $db->collection('products')->bulkInsert([
    ['name' => 'Laptop', 'price' => 999, 'category' => 'electronics'],
    ['name' => 'Mouse', 'price' => 25, 'category' => 'electronics'],
    ['name' => 'Desk', 'price' => 350, 'category' => 'furniture'],
    ['name' => 'Chair', 'price' => 200, 'category' => 'furniture'],
    ['name' => 'Keyboard', 'price' => 75, 'category' => 'electronics'],
]);

echo "Insertados {$count} productos: " . implode(', ', array_map(fn($p) => $p['name'], $products)) . "\n";

// ─────────────────────────────────────────────
// 5. Orders (para joins)
// ─────────────────────────────────────────────
echo "\n── 5. Creando orders ──\n";

$o1 = $db->collection('orders')->insert(['user_id' => $alice['id'], 'product_id' => $products[0]['id'], 'total' => 999, 'status' => 'completed']);
$o2 = $db->collection('orders')->insert(['user_id' => $alice['id'], 'product_id' => $products[1]['id'], 'total' => 25, 'status' => 'pending']);
$o3 = $db->collection('orders')->insert(['user_id' => $bob['id'], 'product_id' => $products[2]['id'], 'total' => 350, 'status' => 'completed']);

echo "Insertados 3 orders: Alice(2), Bobby(1), Charlie(0)\n";

// ─────────────────────────────────────────────
// 6. INNER JOIN
// ─────────────────────────────────────────────
echo "\n── 6. INNER JOIN: orders + users ──\n";

$results = $db->collection('orders')
    ->join('users', 'user_id', 'id', 'inner', 'user_')
    ->orderBy('total', 'desc')
    ->get();

foreach ($results as $r) {
    printf(
        "  Order %s | %s | total: %d | user: %s\n",
        substr($r['id'], 0, 8),
        $r['status'],
        $r['total'],
        $r['user_name']
    );
}

// ─────────────────────────────────────────────
// 7. LEFT JOIN
// ─────────────────────────────────────────────
echo "\n── 7. LEFT JOIN: users + orders ──\n";

$results = $db->collection('users')
    ->leftJoin('orders', 'id', 'user_id', 'order_')
    ->orderBy('name', 'asc')
    ->get();

foreach ($results as $r) {
    $orderTotal = $r['order_total'] ?? 'sin pedidos';
    printf("  %s — order total: %s\n", $r['name'], $orderTotal);
}

// ─────────────────────────────────────────────
// 8. Multiple joins
// ─────────────────────────────────────────────
echo "\n── 8. MULTI JOIN: orders + users + products ──\n";

$results = $db->collection('orders')
    ->join('users', 'user_id', 'id', 'inner', 'user_')
    ->join('products', 'product_id', 'id', 'inner', 'product_')
    ->orderBy('total', 'desc')
    ->get();

foreach ($results as $r) {
    printf(
        "  %s compró %s por $%d\n",
        $r['user_name'],
        $r['product_name'],
        $r['total']
    );
}

// ─────────────────────────────────────────────
// 9. Join con filtro
// ─────────────────────────────────────────────
echo "\n── 9. JOIN + filtro: solo orders de Alice ──\n";

$results = $db->collection('orders')
    ->join('users', 'user_id', 'id', 'inner', 'user_')
    ->where('user_name', '=', 'Alice')
    ->get();

foreach ($results as $r) {
    printf("  Order de %s — total: $%d\n", $r['user_name'], $r['total']);
}

// ─────────────────────────────────────────────
// 10. Write buffering
// ─────────────────────────────────────────────
echo "\n── 10. Write buffering ──\n";

$db->createCollection('buffered');

// Configurar buffer con threshold alto para que no flushee automáticamente
$db->configureBuffer(1000, 60);

$t0 = microtime(true);
for ($i = 0; $i < 500; $i++) {
    $db->collection('buffered')->insert(['idx' => $i, 'data' => "item_{$i}"]);
}
$insertTime = round((microtime(true) - $t0) * 1000, 2);
echo "500 inserts buffered: {$insertTime}ms\n";

// Los datos son visibles en memoria antes del flush
$countBefore = $db->collection('buffered')->count();
echo "Count antes de flush (en memoria): {$countBefore}\n";

// Flush manual
$t0 = microtime(true);
$db->flush();
$flushTime = round((microtime(true) - $t0) * 1000, 2);
echo "Flush a disco: {$flushTime}ms\n";

// ─────────────────────────────────────────────
// 11. Nuevos operadores
// ─────────────────────────────────────────────
echo "\n── 11. Nuevos operadores ──\n";

$between = $db->collection('products')->whereBetween('price', 50, 400)->get();
echo "whereBetween(50, 400): " . implode(', ', array_map(fn($p) => "{$p['name']}(\${$p['price']})", $between)) . "\n";

$in = $db->collection('products')->whereIn('category', ['furniture'])->get();
echo "whereIn(['furniture']): " . implode(', ', array_map(fn($p) => $p['name'], $in)) . "\n";

$notIn = $db->collection('products')->whereNotIn('category', ['furniture'])->get();
echo "whereNotIn(['furniture']): " . implode(', ', array_map(fn($p) => $p['name'], $notIn)) . "\n";

// ─────────────────────────────────────────────
// 12. Aggregations
// ─────────────────────────────────────────────
echo "\n── 12. Aggregations ──\n";

$agg = $db->collection('products')
    ->sum('price', 'total')
    ->avg('price', 'promedio')
    ->min('price', 'minimo')
    ->max('price', 'maximo')
    ->get();
printf(
    "  sum: $%s | avg: $%s | min: $%s | max: $%s\n",
    $agg[0]['total'],
    $agg[0]['promedio'],
    $agg[0]['minimo'],
    $agg[0]['maximo']
);

// Group by
$grouped = $db->collection('products')
    ->groupBy('category', [
        ['function' => 'count', 'alias' => 'total'],
        ['function' => 'sum', 'field' => 'price', 'alias' => 'revenue'],
    ])
    ->get();
echo "  Group by category:\n";
foreach ($grouped as $g) {
    printf("    %s — %d productos, $%s total\n", $g['category'], $g['total'], $g['revenue']);
}

// ─────────────────────────────────────────────
// 13. Range index
// ─────────────────────────────────────────────
echo "\n── 13. Range index ──\n";

$db->collection('products')->createIndex('price', 'range');
echo "Creado range index en products.price\n";

$rangeResults = $db->collection('products')->whereBetween('price', 100, 500)->get();
echo "Range query (100-500): " . implode(', ', array_map(fn($p) => "{$p['name']}(\${$p['price']})", $rangeResults)) . "\n";

// ─────────────────────────────────────────────
// 14. CSV export/import
// ─────────────────────────────────────────────
echo "\n── 14. CSV export/import ──\n";

$csvPath = $dataPath . '/products_export.csv';
$exported = $db->collection('products')->exportCsv($csvPath);
echo "Exportados {$exported} productos a CSV\n";

$db->createCollection('products_imported');
$imported = $db->collection('products_imported')->importCsv($csvPath);
echo "Importados {$imported} productos desde CSV\n";

$importedCount = $db->collection('products_imported')->count();
echo "Count productos importados: {$importedCount}\n";

// ─────────────────────────────────────────────
// 15. Shutdown limpio
// ─────────────────────────────────────────────
echo "\n── 15. Shutdown ──\n";

// Insertar algo más sin flush manual — shutdown debería flushearlo
$db->collection('buffered')->insert(['idx' => 999, 'data' => 'last_item']);
$db->shutdown();
echo "Shutdown completado (flush automático de buffers pendientes)\n";

// Reabrir y verificar que el último doc sobrevivió
$db2 = new AnvilDb($dataPath, $encryptionKey);
$lastCount = $db2->collection('buffered')->count();
echo "Count después de reopen: {$lastCount} (esperado: 501)\n";

$db2->close();

// // Limpiar
// $files = new RecursiveIteratorIterator(
//     new RecursiveDirectoryIterator($dataPath, FilesystemIterator::SKIP_DOTS),
//     RecursiveIteratorIterator::CHILD_FIRST
// );
// foreach ($files as $f) {
//     $f->isDir() ? rmdir($f->getPathname()) : unlink($f->getPathname());
// }
// rmdir($dataPath);

echo "\n=== Test completado ===\n";
