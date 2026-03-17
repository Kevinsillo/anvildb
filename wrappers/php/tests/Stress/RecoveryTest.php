<?php

declare(strict_types=1);

namespace AnvilDb\Tests\Stress;

use AnvilDb\AnvilDb;
use AnvilDb\FFI\Bridge;
use PHPUnit\Framework\TestCase;

/**
 * Recovery test — verifies that AnvilDB handles abrupt process termination
 * without corrupting data, thanks to atomic writes (temp file + rename).
 *
 * @group stress
 */
class RecoveryTest extends TestCase
{
    private string $dataPath;

    protected function setUp(): void
    {
        $this->dataPath = sys_get_temp_dir() . '/anvildb_recovery_' . uniqid();
        mkdir($this->dataPath, 0777, true);
    }

    protected function tearDown(): void
    {
        $this->removeDirectory($this->dataPath);
    }

    /**
     * Simulates a process that dies mid-batch by killing the child process.
     * Verifies that the collection file is not left in a corrupted state.
     */
    public function testDataIntegrityAfterProcessKill(): void
    {
        // Step 1: Create collection and insert some baseline data
        $db = new AnvilDb($this->dataPath);
        $db->createCollection('recovery');
        $collection = $db->collection('recovery');

        for ($i = 0; $i < 100; $i++) {
            $collection->insert(['name' => "baseline_$i", 'value' => $i]);
        }
        $db->close();
        Bridge::reset();

        // Step 2: Spawn a child process that writes continuously and gets killed
        $autoload = dirname(__DIR__, 2) . '/vendor/autoload.php';
        $script = sys_get_temp_dir() . '/anvildb_crasher_' . uniqid() . '.php';

        file_put_contents($script, <<<PHP
<?php
require '$autoload';

\$db = new AnvilDb\AnvilDb(\$argv[1]);
\$collection = \$db->collection('recovery');

// Write as fast as possible — parent will kill us mid-flight
for (\$i = 0; \$i < 100000; \$i++) {
    \$collection->insert(['name' => "crash_\$i", 'value' => \$i]);
}

\$db->close();
AnvilDb\FFI\Bridge::reset();
PHP);

        $descriptors = [
            0 => ['pipe', 'r'],
            1 => ['pipe', 'w'],
            2 => ['pipe', 'w'],
        ];

        $process = proc_open(
            [PHP_BINARY, '-d', 'ffi.enable=true', $script, $this->dataPath],
            $descriptors,
            $pipes
        );

        $this->assertIsResource($process);

        // Let the child run briefly, then kill it
        usleep(50_000); // 50ms — enough for some inserts

        $status = proc_get_status($process);
        if ($status['running']) {
            // Send SIGKILL — abrupt termination, no cleanup
            proc_terminate($process, 9);
        }

        fclose($pipes[0]);
        fclose($pipes[1]);
        fclose($pipes[2]);
        proc_close($process);

        // Step 3: Reopen and verify the data is not corrupted
        $db = new AnvilDb($this->dataPath);
        $collection = $db->collection('recovery');

        // The collection should be readable — no corrupt JSON
        $allDocs = $collection->all();
        $count = $collection->count();

        // At minimum the baseline 100 should be intact
        $this->assertGreaterThanOrEqual(100, $count, 'Baseline data should survive process kill');
        $this->assertCount($count, $allDocs, 'Count and all() should agree');

        // Verify documents have valid structure
        foreach ($allDocs as $doc) {
            $this->assertArrayHasKey('id', $doc);
            $this->assertArrayHasKey('name', $doc);
            $this->assertArrayHasKey('value', $doc);
        }

        // Verify baseline data is all there
        $baselineDocs = $collection->where('name', 'contains', 'baseline_')->get();
        $this->assertSame(100, count($baselineDocs), 'All 100 baseline documents should be intact');

        $db->close();
        Bridge::reset();

        fwrite(STDERR, sprintf(
            "\n--- Recovery Test Report ---\n" .
            "  Baseline docs:  100\n" .
            "  Total after kill: %d (baseline + partial writes)\n" .
            "  Data integrity:   OK\n" .
            "----------------------------\n",
            $count
        ));

        unlink($script);
    }

    /**
     * Verifies that reopening the database after a normal close
     * preserves all data across multiple open/close cycles.
     */
    public function testDataPersistsAcrossMultipleReopens(): void
    {
        // Cycle 1: create and populate
        $db = new AnvilDb($this->dataPath);
        $db->createCollection('persist');
        $db->collection('persist')->insert(['cycle' => 1, 'data' => 'first']);
        $db->close();
        Bridge::reset();

        // Cycle 2: add more
        $db = new AnvilDb($this->dataPath);
        $db->collection('persist')->insert(['cycle' => 2, 'data' => 'second']);
        $db->close();
        Bridge::reset();

        // Cycle 3: add more
        $db = new AnvilDb($this->dataPath);
        $db->collection('persist')->insert(['cycle' => 3, 'data' => 'third']);
        $db->close();
        Bridge::reset();

        // Cycle 4: verify everything
        $db = new AnvilDb($this->dataPath);
        $all = $db->collection('persist')->all();
        $db->close();
        Bridge::reset();

        $this->assertCount(3, $all);
        $cycles = array_column($all, 'cycle');
        sort($cycles);
        $this->assertSame([1, 2, 3], $cycles);
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
