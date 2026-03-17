<?php

declare(strict_types=1);

namespace AnvilDb\FFI;

use AnvilDb\Exception\FFIException;

/**
 * FFI bridge that loads and provides access to the native AnvilDB shared library.
 */
class Bridge
{
    /** @var AnvilDbFFI|\FFI|null */
    private static ?\FFI $ffi = null;

    /**
     * Get the singleton FFI instance, initializing it on first call.
     *
     * Returns `\FFI` at runtime, typed as {@see AnvilDbFFI} for IDE autocompletion.
     *
     * @return AnvilDbFFI
     *
     * @throws FFIException If the FFI extension is missing, the header is unreadable, or the library is not found
     */
    public static function get(): \FFI
    {
        if (self::$ffi === null) {
            if (!extension_loaded('ffi')) {
                throw new FFIException('PHP FFI extension is not loaded. Enable it in php.ini (ffi.enable=true)');
            }

            $header = file_get_contents(__DIR__ . '/anvildb.h');
            if ($header === false) {
                throw new FFIException('Cannot read anvildb.h header file');
            }

            $soPath = self::detectLibraryPath();
            if (!file_exists($soPath)) {
                throw new FFIException("Shared library not found at: {$soPath}");
            }

            self::$ffi = \FFI::cdef($header, $soPath);
        }

        return self::$ffi;
    }

    /**
     * Reset the cached FFI instance (useful for testing).
     *
     * @return void
     */
    public static function reset(): void
    {
        self::$ffi = null;
    }

    private static function detectLibraryPath(): string
    {
        $envPath = getenv('ANVILDB_LIB_PATH');
        if ($envPath !== false && file_exists($envPath)) {
            return $envPath;
        }

        $wrapperDir = dirname(__DIR__, 2);      // wrappers/php/
        $monorepoRoot = dirname($wrapperDir, 2); // project root

        $os = PHP_OS_FAMILY;
        $arch = php_uname('m');

        $ext = match ($os) {
            'Darwin' => 'dylib',
            'Windows' => 'dll',
            default => 'so',
        };

        $libName = $os === 'Windows' ? "anvildb.{$ext}" : "libanvildb.{$ext}";

        // 1. Monorepo dev: target/release or target/debug (cargo build from root)
        $releaseLib = $monorepoRoot . "/target/release/{$libName}";
        if (file_exists($releaseLib)) {
            return $releaseLib;
        }

        $debugLib = $monorepoRoot . "/target/debug/{$libName}";
        if (file_exists($debugLib)) {
            return $debugLib;
        }

        // 2. Distributed wrapper: lib/<platform>/ (injected by CI on subtree split)
        $platform = match (true) {
            $os === 'Linux' && $arch === 'x86_64' => 'x86_64-linux',
            $os === 'Linux' && str_contains($arch, 'aarch64') => 'aarch64-linux',
            $os === 'Darwin' && $arch === 'x86_64' => 'x86_64-darwin',
            $os === 'Darwin' && str_contains($arch, 'arm64') => 'aarch64-darwin',
            $os === 'Windows' && $arch === 'AMD64' => 'x86_64-windows',
            default => throw new FFIException("Unsupported platform: {$os} {$arch}"),
        };

        return $wrapperDir . "/lib/{$platform}/{$libName}";
    }
}
