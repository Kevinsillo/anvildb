# Wrapper Development Guide

[< Back to index](index.md)

How to create an AnvilDB wrapper for a new programming language. The core engine exposes a C-compatible API — any language with FFI support can integrate.

## What a Wrapper Does

1. **Loads** the native shared library (`libanvildb.so` / `.dylib` / `.dll`)
2. **Calls** the C functions (see [C API Reference](c-api.md))
3. **Exposes** an idiomatic API in the target language
4. **Handles** memory management (freeing Rust-allocated strings via `anvildb_free_string`)

The wrapper does NOT need to understand Rust internals — only the C API contract.

## Naming Conventions

- Package name: `anvildb` or `anvildb-<language>`
- Main class/module: `AnvilDb` or `AnvilDB` (follow language conventions)
- Namespace: language-appropriate (e.g. `AnvilDb\` in PHP, `anvildb` in Python)

## Required Features

Every wrapper MUST expose:

- [ ] Open / close / shutdown
- [ ] Create / drop / list collections
- [ ] Insert / find / update / delete / bulk insert
- [ ] Query with filters, sorting, limit, offset
- [ ] Joins (inner, left)
- [ ] Count (with optional filter)
- [ ] Index create / drop
- [ ] Schema validation
- [ ] Flush / configure buffer
- [ ] Encrypt / decrypt
- [ ] Error handling (codes + messages)
- [ ] Automatic string cleanup (`anvildb_free_string`)

## Library Detection

The wrapper should look for the native library in this order:

1. **Environment variable** `ANVILDB_LIB_PATH` (explicit path)
2. **Build output** `target/release/` or `target/debug/` (development)
3. **Bundled binary** `lib/<platform>/` (distributed package)

Platform map:

| Platform | Library |
|----------|---------|
| `x86_64-linux` | `libanvildb.so` |
| `aarch64-linux` | `libanvildb.so` |
| `x86_64-darwin` | `libanvildb.dylib` |
| `aarch64-darwin` | `libanvildb.dylib` |
| `x86_64-windows` | `anvildb.dll` |

## Minimum Tests

- Full CRUD cycle (insert, find, update, delete)
- Query builder (filters, sort, limit, offset)
- Bulk operations
- Collection management (create, drop, list)
- Index operations
- Schema validation
- Connection lifecycle (open, close, reopen)

## Directory Structure

```
wrappers/<language>/
├── src/           # or lib/, pkg/, etc.
├── tests/
├── <manifest>     # composer.json, setup.py, package.json, etc.
└── README.md      # optional
```

## Publishing

Each wrapper is distributed via subtree split (see [CI/CD](ci-cd.md)):

1. Create an empty read-only repo: `kevinsillo/anvildb-<language>`
2. CI splits `wrappers/<language>/` and pushes to that repo
3. CI injects precompiled binaries into `lib/` before pushing
4. Register in the target package manager (Packagist, PyPI, npm, etc.)

## Example

A minimal Python wrapper showing the pattern:

```python
import ctypes, json

class AnvilDb:
    def __init__(self, path, key=None):
        self._lib = ctypes.CDLL("libanvildb.so")
        self._handle = self._lib.anvildb_open(path.encode(), key.encode() if key else None)

    def insert(self, collection, doc):
        result = self._lib.anvildb_insert(self._handle, collection.encode(), json.dumps(doc).encode())
        if not result:
            raise RuntimeError(self._get_error())
        parsed = json.loads(ctypes.string_at(result).decode())
        self._lib.anvildb_free_string(result)
        return parsed

    def close(self):
        self._lib.anvildb_close(self._handle)
```

A production wrapper should define proper `argtypes`/`restype` for all functions and handle all error cases.
