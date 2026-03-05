# gitprint

[![CI](https://github.com/izelnakri/gitprint/actions/workflows/ci.yml/badge.svg)](https://github.com/izelnakri/gitprint/actions/workflows/ci.yml)
[![Crate](https://img.shields.io/crates/v/gitprint)](https://crates.io/crates/gitprint)
[![Downloads](https://img.shields.io/crates/d/gitprint)](https://crates.io/crates/gitprint)
[![Docs](https://img.shields.io/badge/docs-online-blue)](https://izelnakri.github.io/gitprint/docs/gitprint/)
[![Sponsor](https://img.shields.io/badge/sponsor-%E2%99%A5-pink)](https://github.com/sponsors/izelnakri)

Convert git repositories into beautifully formatted, printer-friendly PDFs — or preview them directly in the terminal.

![gitprint demo](demo/demo.gif)

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
- **Terminal preview mode** — inspect repo or user data in the terminal without generating a PDF
- **GitHub user report mode** — generate a PDF (or preview) of a user's activity, repos, and recent commits

## Installation

### With Nix

```sh
nix profile install github:izelnakri/gitprint
```

### With Cargo

```sh
cargo install --git https://github.com/izelnakri/gitprint
```

### With Docker

No install needed — pull the latest nightly image from GitHub Container Registry and mount your repository:

```sh
docker run --rm -w /repo -v "$(pwd):/repo" ghcr.io/izelnakri/gitprint:nightly . -o output.pdf
```

- `--rm` removes the container after use
- `-w /repo` sets the working directory inside the container so `.` resolves correctly
- `-v "$(pwd):/repo"` mounts the current directory; the output PDF is written back to it

## Usage

### Repository Mode (Default)

```sh
# Generate PDF from current directory (or git repo)
gitprint .

# Print a single file
gitprint src/main.rs

# Print any directory (no git required)
gitprint /path/to/dir

# Print a remote repository
gitprint https://github.com/user/repo

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

### User Report Mode

```sh
# Generate a GitHub user activity report PDF
gitprint --user torvalds

# Limit the activity date range
gitprint --user torvalds --since "last month"
gitprint --user torvalds --since 2024-01-01 --until 2024-12-31

# Show only push events (commits), not issues/PRs/stars
gitprint --user torvalds --activity commits

# Include more repos and commits in the report
gitprint --user torvalds --last-repos 10 --last-commits 10

# Skip commit diffs for a faster, lighter report
gitprint --user torvalds --no-diffs

# Increase GitHub API rate limits with a personal access token
GITHUB_TOKEN=ghp_... gitprint --user torvalds
```

### Preview Mode

Preview shows all the same data as the PDF — metadata, directory tree, file list with LOC/sizes, or GitHub user activity — directly in the terminal without writing any file.

```sh
# Preview a repository in the terminal
gitprint . --preview

# Preview a remote repository
gitprint https://github.com/user/repo --preview

# Preview a GitHub user report
gitprint --user torvalds --preview

# Combine with other flags — filters and date ranges all apply
gitprint . --preview --include "*.rs"
gitprint --user torvalds --preview --since "last month" --activity commits
```

## CLI Reference

```
Convert git repositories into beautifully formatted PDFs.

MODES

  gitprint <PATH> [OPTIONS]
    Local path, file, or remote URL (https://, git@, ssh://) → PDF

  gitprint --user <USERNAME> [OPTIONS]
    GitHub user activity report → PDF

  gitprint <PATH|--user USERNAME> --preview
    Preview output in the terminal — no PDF generated

Usage: gitprint [OPTIONS] [PATH]

Arguments:
  [PATH]
    Local path, file, or remote URL (https://, git@, ssh://)

Options:
      --preview          Preview output in the terminal instead of generating a PDF
  -o, --output <PATH>    Output PDF file path
  -h, --help             Print help
  -V, --version          Print version

Repository Mode (Default):
      --include <PATTERN>      Glob patterns for files to include (repeatable)
      --exclude <PATTERN>      Glob patterns for files to exclude (repeatable)
      --theme <NAME>           Syntax highlighting theme [default: InspiredGitHub]
      --font-size <SIZE>       Code font size in points [default: 8]
      --no-line-numbers        Disable line numbers
      --no-toc                 Disable table of contents
      --no-file-tree           Disable directory tree visualization
      --branch <NAME>          Use a specific branch
      --commit <HASH>          Use a specific commit
      --paper-size <SIZE>      Paper size [default: a4] [possible values: a4, letter, legal]
      --landscape              Use landscape orientation
      --list-themes            List available syntax themes and exit
      --list-tags              List version tags of the repository and exit

User Report Mode:
  -u, --user <USERNAME>        GitHub username — generate a user activity report
      --last-repos <N>         Most-recently-pushed repos to include [default: 5]
      --last-commits <N>       Recent commits with diffs to render [default: 5]
      --no-diffs               Skip commit diff rendering (faster)
      --since <DATE>           Show events from this date forward
      --until <DATE>           Show events up to and including this date
      --activity <TYPE>        Event types: all (default) or commits
      --events <N>             Max events shown in activity feed [default: 30]
```

### Date formats for `--since` / `--until`

| Format | Example |
|--------|---------|
| ISO date | `2024-01-15` or `2024-01-15T00:00:00Z` |
| Keywords | `today`, `yesterday` |
| Named | `last week`, `last month`, `last year` |
| Relative | `30 days ago`, `2 weeks ago`, `1 month ago` |

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
