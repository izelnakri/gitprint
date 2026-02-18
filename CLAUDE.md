# gitprint

Rust CLI that converts git repositories into syntax-highlighted, printer-friendly PDFs.

## Build & Test

- `cargo build` — build the project
- `cargo test` — run all tests (unit + integration)
- `cargo clippy --all-targets -- -D warnings` — lint
- `cargo fmt -- --check` — check formatting
- `nix flake check` — run all CI checks (build, clippy, fmt, tests)
- `nix develop` — enter development shell

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
