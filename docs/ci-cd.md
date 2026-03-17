# CI/CD

[< Back to index](index.md)

AnvilDB uses GitHub Actions for continuous integration, releases, and wrapper distribution.

## Workflows

| Workflow | File | Trigger | Purpose |
|----------|------|---------|---------|
| Tests | `.github/workflows/tests.yml` | Push to `main`, PRs | Run Rust + PHP tests |
| Release | `.github/workflows/release.yml` | Tag `v*` | Cross-compile and create GitHub Release |
| Miri | `.github/workflows/miri.yml` | Push to `main`, PRs | Memory safety checks with Miri |

## Continuous Integration (`tests.yml`)

Every push to `main` and every pull request triggers:

1. **Rust tests** — `cargo test` on `ubuntu-latest`
2. **PHP tests** — PHPUnit on PHP 8.1, 8.2, 8.3, and 8.4

PHP tests depend on Rust tests passing first. The native library is built automatically during CI.

```
Push / PR
    │
    ▼
Rust tests (cargo test)
    │ pass
    ▼
PHP tests (4 PHP versions in parallel)
    │ all pass
    ▼
✓ Green
```

### What happens in the PHP test job

1. Checkout the repository
2. Install Rust toolchain and build the native library (`cargo build`)
3. Setup PHP with FFI extension enabled
4. Install Composer dependencies from `wrappers/php/`
5. Run PHPUnit from `wrappers/php/`

## Memory Safety (`miri.yml`)

Runs [Miri](https://github.com/rust-lang/miri) (Rust's undefined behavior detector) on unit tests using the nightly toolchain. This catches memory safety issues that normal tests might miss.

## Releasing a New Version (`release.yml`)

Releases are fully automated. To publish a new version:

1. Make sure `main` is stable — all tests passing, changes merged
2. Update the version in `core/Cargo.toml` if needed
3. Create and push a tag:
   ```bash
   git tag v0.5.0
   git push origin v0.5.0
   ```

That's it. The workflow automatically:

1. **Cross-compiles** for all 5 target platforms
2. **Packages** each binary as a `.tar.gz` archive
3. **Creates a GitHub Release** with auto-generated notes
4. **Attaches** all archives to the release

### Target Platforms

| Target | OS | Artifact |
|--------|-----|----------|
| `x86_64-unknown-linux-gnu` | Ubuntu | `libanvildb.so` |
| `aarch64-unknown-linux-gnu` | Ubuntu (cross) | `libanvildb.so` |
| `x86_64-apple-darwin` | macOS 13 | `libanvildb.dylib` |
| `aarch64-apple-darwin` | macOS 14 (Apple Silicon) | `libanvildb.dylib` |
| `x86_64-pc-windows-msvc` | Windows | `anvildb.dll` |

### Cross-compilation notes

- **aarch64-linux** uses `gcc-aarch64-linux-gnu` as a cross-linker
- **macOS** targets run on native runners (x86 on macos-13, ARM on macos-14)
- **Windows** builds use MSVC toolchain

## Versioning

Use [Semantic Versioning](https://semver.org/):

- **Patch** (`v0.1.1`): bug fixes, no API changes
- **Minor** (`v0.2.0`): new features, backwards compatible
- **Major** (`v1.0.0`): breaking API changes

## Subtree Split (Wrapper Distribution)

The monorepo publishes each wrapper to its own read-only repository via `git subtree split`. This allows language-specific package managers to install the wrapper without pulling the entire monorepo.

### How it works

```
Monorepo (kevinsillo/anvildb)
    │
    │  git subtree split --prefix=wrappers/php
    │
    ▼
Read-only repo (kevinsillo/anvildb-php)
    │
    │  + precompiled binaries from GitHub Release
    │
    ▼
Packagist (composer require kevinsillo/anvildb)
```

### Wrapper repositories

| Wrapper | Repository | Package Manager |
|---------|-----------|-----------------|
| PHP | [kevinsillo/anvildb-php](https://github.com/kevinsillo/anvildb-php) | Packagist |

### Adding a new wrapper

1. Create the wrapper code in `wrappers/<language>/`
2. Create an empty read-only repository on GitHub (`kevinsillo/anvildb-<language>`)
3. Add a subtree split step to the CI workflow for the new prefix
4. Configure the target package manager (PyPI, npm, etc.) to pull from the read-only repo

## Full Workflow

```
Feature/fix branch
    │
    ▼
Pull Request → CI runs tests (Rust + PHP 8.1–8.4) + Miri
    │
    ▼
Merge to main
    │
    ▼
git tag v0.x.x → push tag
    │
    ▼
Release workflow → cross-compile → GitHub Release with binaries
    │
    ▼
Subtree split → push to wrapper repos → package managers
```
