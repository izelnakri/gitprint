# gitprint

[![CI](https://github.com/izelnakri/gitprint/actions/workflows/ci.yml/badge.svg)](https://github.com/izelnakri/gitprint/actions/workflows/ci.yml)
[![Crate](https://img.shields.io/crates/v/gitprint)](https://crates.io/crates/gitprint)
[![Downloads](https://img.shields.io/crates/d/gitprint)](https://crates.io/crates/gitprint)
[![Docs](https://img.shields.io/badge/docs-online-blue)](https://izelnakri.github.io/gitprint/docs/gitprint/)
[![Sponsor](https://img.shields.io/badge/sponsor-%E2%99%A5-pink)](https://github.com/sponsors/izelnakri)

Convert git repositories into beautifully formatted, printer-friendly PDFs.

## Features

- Syntax-highlighted source code with 100+ languages supported
- Configurable color themes (InspiredGitHub, Solarized, base16, and more)
- Table of contents and directory tree visualization
- Single-file mode — print just one file, no cover page or TOC overhead
- Plain directory support — works on any folder, not just git repos
- Automatic binary and minified file detection and exclusion
- Glob-based include/exclude filtering
- Multiple paper sizes (A4, Letter, Legal) and landscape mode
- Branch and commit selection for printing specific revisions
- Embedded JetBrains Mono font for crisp code rendering
- Async pipeline — metadata, file reads, and highlighting run concurrently

## Installation

### With Nix

```sh
nix profile install github:izelnakri/gitprint
```

### With Cargo

```sh
cargo install --git https://github.com/izelnakri/gitprint
```

## Usage

```sh
# Generate PDF from current directory (or git repo)
gitprint .

# Print a single file
gitprint src/main.rs

# Print any directory (no git required)
gitprint /path/to/dir

# Output to a specific file
gitprint . -o output.pdf

# Include only Rust and TOML files
gitprint . --include "*.rs" --include "*.toml"

# Exclude test files
gitprint . --exclude "test_*.rs"

# Use a different theme
gitprint . --theme "Solarized (dark)"

# List available themes
gitprint . --list-themes

# Use Letter paper in landscape
gitprint . --paper-size letter --landscape

# Print a specific branch or commit
gitprint . --branch feature-x
gitprint . --commit abc1234

# Minimal output: no TOC, no file tree, no line numbers
gitprint . --no-toc --no-file-tree --no-line-numbers
```

## CLI Reference

```
Convert git repositories into beautifully formatted PDFs

Usage: gitprint [OPTIONS] <PATH>

Arguments:
  <PATH>  Path to git repository, directory, or single file

Options:
  -o, --output <OUTPUT>          Output PDF file path
      --include <INCLUDE>        Glob patterns for files to include (repeatable)
      --exclude <EXCLUDE>        Glob patterns for files to exclude (repeatable)
      --theme <THEME>            Syntax highlighting theme [default: InspiredGitHub]
      --font-size <FONT_SIZE>    Code font size in points [default: 8]
      --no-line-numbers          Disable line numbers
      --no-toc                   Disable table of contents
      --no-file-tree             Disable directory tree visualization
      --branch <BRANCH>          Use a specific branch
      --commit <COMMIT>          Use a specific commit
      --paper-size <PAPER_SIZE>  Paper size [default: a4] [possible values: a4, letter, legal]
      --landscape                Use landscape orientation
      --list-themes              List available syntax themes and exit
  -h, --help                     Print help
  -V, --version                  Print version
```

## Development

```sh
nix develop        # Enter dev shell (Rust toolchain, git-cliff, cargo-release)
make check         # Fmt check + clippy + tests (run before every commit)
make build         # Build
make test          # Run tests
make doc           # Build and open API docs
make release       # Bump CHANGELOG + publish (LEVEL=patch|minor|major)
nix flake check    # Full CI suite: build, clippy, fmt, tests
```

## Donate

If gitprint saves you time, consider sponsoring development:

**[github.com/sponsors/izelnakri](https://github.com/sponsors/izelnakri)**

GitHub Sponsors has zero platform fees — 100% goes to the developer.

## License

MIT
