<?php

declare(strict_types=1);

namespace AnvilDb\Tests\Stress;

use AnvilDb\AnvilDb;
use AnvilDb\FFI\Bridge;
use PHPUnit\Framework\TestCase;

/**
 * Concurrency test — spawns multiple PHP child processes writing simultaneously
 * to the same AnvilDB instance to verify data integrity under contention.
 */
class ConcurrencyTest extends TestCase
{
    private string $dataPath;

    protected function setUp(): void
    {
        $this->dataPath = sys_get_temp_dir() . '/anvildb_concurrency_' . uniqid();
        mkdir($this->dataPath, 0777, true);

        // Pre-create the collection so child processes can write immediately
        $db = new AnvilDb($this->dataPath);
        $db->createCollection('concurrent');
        $db->close();
        Bridge::reset();
    }

    protected function tearDown(): void
    {
        $this->removeDirectory($this->dataPath);
    }

    public function testMultipleProcessesWritingSimultaneously(): void
    {
        $workerCount = 5;
        $insertsPerWorker = 100;
        $expectedTotal = $workerCount * $insertsPerWorker;

        $workerScript = $this->createWorkerScript($insertsPerWorker);
        $processes = [];
        $pipes = [];

        // Spawn child processes
        for ($i = 0; $i < $workerCount; $i++) {
            $descriptors = [
                0 => ['pipe', 'r'],
                1 => ['pipe', 'w'],
                2 => ['pipe', 'w'],
            ];

            $process = proc_open(
                [PHP_BINARY, '-d', 'ffi.enable=true', $workerScript, $this->dataPath, (string) $i],
                $descriptors,
                $workerPipes
            );

            $this->assertIsResource($process, "Failed to spawn worker $i");
            $processes[] = $process;
            $pipes[] = $workerPipes;
        }

        // Wait for all processes to finish
        $allSucceeded = true;
        foreach ($processes as $i => $process) {
            $stdout = stream_get_contents($pipes[$i][1]);
            $stderr = stream_get_contents($pipes[$i][2]);
            fclose($pipes[$i][0]);
            fclose($pipes[$i][1]);
            fclose($pipes[$i][2]);

            $exitCode = proc_close($process);
            if ($exitCode !== 0) {
                $allSucceeded = false;
                $this->fail("Worker $i failed with exit code $exitCode: $stderr");
            }
        }

        // Verify data integrity — all documents must be present, no corruption
        $db = new AnvilDb($this->dataPath);
        $collection = $db->collection('concurrent');

        $allDocs = $collection->all();
        $count = $collection->count();
        $db->close();
        Bridge::reset();

        $this->assertSame($expectedTotal, $count, "Expected $expectedTotal documents, got $count");
        $this->assertCount($expectedTotal, $allDocs);

        // Verify each document has valid structure
        $workerCounts = array_fill(0, $workerCount, 0);
        foreach ($allDocs as $doc) {
            $this->assertArrayHasKey('id', $doc);
            $this->assertArrayHasKey('worker', $doc);
            $this->assertArrayHasKey('index', $doc);
            $this->assertArrayHasKey('data', $doc);

            $workerCounts[$doc['worker']]++;
        }

        // Each worker should have inserted exactly $insertsPerWorker documents
        foreach ($workerCounts as $i => $wCount) {
            $this->assertSame(
                $insertsPerWorker,
                $wCount,
                "Worker $i inserted $wCount documents, expected $insertsPerWorker"
            );
        }

        // Clean up worker script
        unlink($workerScript);
    }

    public function testConcurrentReadsAndWrites(): void
    {
        $db = new AnvilDb($this->dataPath);
        $collection = $db->collection('concurrent');

        // Pre-populate with some data
        for ($i = 0; $i < 50; $i++) {
            $collection->insert(['name' => "seed_$i", 'value' => $i]);
        }
        $db->close();
        Bridge::reset();

        $writerScript = $this->createWriterScript(50);
        $readerScript = $this->createReaderScript(100);

        $descriptors = [
            0 => ['pipe', 'r'],
            1 => ['pipe', 'w'],
            2 => ['pipe', 'w'],
        ];

        // Start writer and readers simultaneously
        $writerPipes = [];
        $writer = proc_open(
            [PHP_BINARY, '-d', 'ffi.enable=true', $writerScript, $this->dataPath],
            $descriptors,
            $writerPipes
        );

        $readerPipes = [];
        $reader = proc_open(
            [PHP_BINARY, '-d', 'ffi.enable=true', $readerScript, $this->dataPath],
            $descriptors,
            $readerPipes
        );

        // Wait for both
        $writerStderr = stream_get_contents($writerPipes[2]);
        fclose($writerPipes[0]);
        fclose($writerPipes[1]);
        fclose($writerPipes[2]);
        $writerExit = proc_close($writer);

        $readerStderr = stream_get_contents($readerPipes[2]);
        fclose($readerPipes[0]);
        fclose($readerPipes[1]);
        fclose($readerPipes[2]);
        $readerExit = proc_close($reader);

        $this->assertSame(0, $writerExit, "Writer failed: $writerStderr");
        $this->assertSame(0, $readerExit, "Reader failed: $readerStderr");

        // Verify final state
        $db = new AnvilDb($this->dataPath);
        $count = $db->collection('concurrent')->count();
        $db->close();
        Bridge::reset();

        $this->assertSame(100, $count, "Expected 100 documents (50 seed + 50 written)");

        unlink($writerScript);
        unlink($readerScript);
    }

    private function createWorkerScript(int $inserts): string
    {
        $autoload = dirname(__DIR__, 2) . '/vendor/autoload.php';
        $script = sys_get_temp_dir() . '/anvildb_worker_' . uniqid() . '.php';

        file_put_contents($script, <<<PHP
<?php
require '$autoload';

\$dataPath = \$argv[1];
\$workerId = (int) \$argv[2];

// Each insert opens/closes the DB to force re-reading from disk,
// simulating real multi-process usage (e.g. separate PHP-FPM workers).
for (\$i = 0; \$i < $inserts; \$i++) {
    \$db = new AnvilDb\AnvilDb(\$dataPath);
    \$db->collection('concurrent')->insert([
        'worker' => \$workerId,
        'index'  => \$i,
        'data'   => str_repeat('x', 64),
    ]);
    \$db->close();
    AnvilDb\FFI\Bridge::reset();
}
PHP);

        return $script;
    }

    private function createWriterScript(int $inserts): string
    {
        $autoload = dirname(__DIR__, 2) . '/vendor/autoload.php';
        $script = sys_get_temp_dir() . '/anvildb_writer_' . uniqid() . '.php';

        file_put_contents($script, <<<PHP
<?php
require '$autoload';

\$db = new AnvilDb\AnvilDb(\$argv[1]);
\$collection = \$db->collection('concurrent');

for (\$i = 0; \$i < $inserts; \$i++) {
    \$collection->insert(['name' => "written_\$i", 'value' => \$i + 1000]);
}

\$db->close();
AnvilDb\FFI\Bridge::reset();
PHP);

        return $script;
    }

    private function createReaderScript(int $reads): string
    {
        $autoload = dirname(__DIR__, 2) . '/vendor/autoload.php';
        $script = sys_get_temp_dir() . '/anvildb_reader_' . uniqid() . '.php';

        file_put_contents($script, <<<PHP
<?php
require '$autoload';

\$db = new AnvilDb\AnvilDb(\$argv[1]);
\$collection = \$db->collection('concurrent');

for (\$i = 0; \$i < $reads; \$i++) {
    // Reads should never fail or return corrupted data
    \$all = \$collection->all();
    if (!is_array(\$all)) {
        fwrite(STDERR, "Read returned non-array at iteration \$i\n");
        exit(1);
    }
}

\$db->close();
AnvilDb\FFI\Bridge::reset();
PHP);

        return $script;
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
