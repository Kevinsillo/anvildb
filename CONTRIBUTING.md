# Contributing to AnvilDB

Thank you for your interest in contributing to AnvilDB. This guide explains how to get involved.

## Before You Start

**Every contribution must start with an issue, and the issue must be accepted before any work begins.**

1. Check existing [issues](https://github.com/Kevinsillo/anvildb/issues) to avoid duplicates
2. Open a new issue describing what you want to do (bug fix, feature, improvement)
3. **Wait for the issue to be accepted** — a maintainer will review and approve the proposal before any code is written
4. Once accepted, you can start working and submit a PR linked to that issue

PRs without an associated accepted issue will be closed. This applies to bug fixes, features, refactors, and documentation changes.

## Getting Started

1. Fork and clone the repository:
   ```bash
   git clone https://github.com/<your-user>/anvildb.git
   cd anvildb
   ```
2. Install PHP dependencies:
   ```bash
   composer install
   ```
3. Install Rust toolchain: https://rustup.rs
4. Build the native library:
   ```bash
   cargo build
   ```
5. Verify everything works:
   ```bash
   cargo test && ./vendor/bin/phpunit
   ```

## Development Workflow

1. Create a branch from `main` with a descriptive name:
   ```
   fix/schema-validation-float
   feat/between-operator
   docs/query-builder-examples
   ```
2. Make your changes
3. Run **all** tests before pushing:
   ```bash
   cargo test && ./vendor/bin/phpunit
   ```
4. Submit a pull request referencing the issue (e.g. `Closes #12`)

## Pull Requests

- **One PR per issue.** Keep changes focused and atomic
- **Reference the issue** in the PR description (`Closes #N` or `Fixes #N`)
- **All tests must pass.** PRs with failing tests will not be reviewed
- **Include tests** for new features or bug fixes
- **Keep the scope small.** Large PRs are harder to review — split them if possible

## Commit Messages

Use clear, descriptive commit messages:

```
feat: add between operator to query builder

Closes #15
```

Prefixes:

| Prefix | Use |
|---|---|
| `feat:` | New feature |
| `fix:` | Bug fix |
| `docs:` | Documentation only |
| `test:` | Adding or updating tests |
| `refactor:` | Code change that neither fixes a bug nor adds a feature |
| `build:` | Changes to build system or dependencies |

## Code Style

- **Rust**: Follow standard `rustfmt` formatting. Run `cargo fmt` before committing
- **PHP**: PSR-12. Strict types enabled in all files

## Project Structure

Changes typically touch both Rust and PHP sides:

| What you're changing | Rust files | PHP files |
|---|---|---|
| New FFI function | `rust/src/lib.rs` + business logic module | `src/FFI/anvildb.h` + wrapper class |
| New query operator | `rust/src/query/engine.rs` | `src/Query/QueryBuilder.php` |
| New index type | `rust/src/index/` | `src/Collection/Collection.php` |
| Bug fix | Depends on the bug | Depends on the bug |

When adding a new FFI function:

1. Add the function signature to `src/FFI/anvildb.h`
2. Implement the `extern "C"` function in `rust/src/lib.rs`
3. Add the business logic in the appropriate Rust module
4. Expose via the PHP wrapper classes
5. Add tests in both Rust (`rust/tests/`) and PHP (`tests/`)

## Reporting Issues

Open an issue with:

- **Bug reports**: description, steps to reproduce, expected vs actual behavior, PHP version, OS, architecture
- **Feature requests**: description, use case, proposed API (if applicable)
- **Questions**: check existing issues and docs first

## Code of Conduct

Be respectful. We are all here to build something useful. Harassment, discrimination, or toxic behavior will not be tolerated. Keep discussions constructive and focused on the work.

## License

By contributing, you agree that your contributions will be licensed under the [MIT License](LICENSE).
