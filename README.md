# gitprint

Convert git repositories into beautifully formatted, printer-friendly PDFs.

## Features

- Syntax-highlighted source code with 100+ languages supported
- Configurable color themes (InspiredGitHub, Solarized, base16, and more)
- Table of contents and directory tree visualization
- Automatic binary and minified file detection and exclusion
- Glob-based include/exclude filtering
- Multiple paper sizes (A4, Letter, Legal) and landscape mode
- Branch and commit selection for printing specific revisions
- Embedded JetBrains Mono font for crisp code rendering
- Memory-efficient streaming — processes one file at a time

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
# Generate PDF from current directory
gitprint

# Generate PDF from a specific repository
gitprint /path/to/repo

# Output to a specific file
gitprint -o output.pdf

# Include only Rust and TOML files
gitprint --include "*.rs" --include "*.toml"

# Exclude test files
gitprint --exclude "test_*.rs"

# Use a different theme
gitprint --theme "Solarized (dark)"

# List available themes
gitprint --list-themes

# Use Letter paper in landscape
gitprint --paper-size letter --landscape

# Print a specific branch or commit
gitprint --branch feature-x
gitprint --commit abc1234

# Minimal output: no TOC, no file tree, no line numbers
gitprint --no-toc --no-file-tree --no-line-numbers
```

## CLI Reference

```
Convert git repositories into beautifully formatted PDFs

Usage: gitprint [OPTIONS] [PATH]

Arguments:
  [PATH]  Path to git repository [default: .]

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
nix develop   # Enter dev shell with Rust toolchain and tools
cargo build   # Build
cargo test    # Run all tests
```

## Running CI Locally

```sh
nix flake check
```

This runs the full check suite: package build, clippy lints, rustfmt, and tests.

## License

MIT
