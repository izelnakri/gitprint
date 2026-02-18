# gitprint

Rust CLI that converts git repositories into syntax-highlighted, printer-friendly PDFs.

## Build & Test

- `make check` — fmt check + clippy + tests (run before every commit)
- `make build` — build the project
- `make test` — run all tests (unit + integration)
- `make fmt` — format source code
- `make doc` — build and open API docs
- `make release [LEVEL=patch|minor|major]` — generate CHANGELOG and publish
- `nix flake check` — run all CI checks (build, clippy, fmt, tests)
- `nix develop` — enter development shell (includes git-cliff, cargo-release)

## Architecture

Single-binary CLI. Pipeline: git → filter → highlight → PDF.

Modules:
- `cli.rs` — Clap argument parsing
- `types.rs` — Shared data types (Config, RepoMetadata, PaperSize, etc.)
- `git.rs` — Git operations via `git` CLI subprocess
- `filter.rs` — Glob-based file filtering + binary/minified detection
- `defaults.rs` — Default exclude glob patterns
- `highlight.rs` — Syntax highlighting via syntect
- `pdf/` — PDF generation via printpdf
  - `mod.rs` — Document creation and writing
  - `fonts.rs` — Embedded JetBrains Mono font loading
  - `cover.rs` — Cover page rendering
  - `toc.rs` — Table of contents rendering
  - `tree.rs` — Directory tree visualization
  - `code.rs` — Highlighted source code rendering
- `lib.rs` — Main pipeline orchestration
- `main.rs` — CLI entry point

## Conventions

- Edition 2024
- Error handling: anyhow for ergonomic error propagation throughout
- Tests: inline `#[cfg(test)] mod tests` for unit tests, `tests/` directory for integration tests
- Integration tests use `tempfile` crate to create temporary git repos
- No unsafe code
