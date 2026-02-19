.DEFAULT_GOAL := help

LEVEL ?= patch
REGRESSION_THRESHOLD ?= 20

.PHONY: help fix check fmt test build doc bench-baseline bench-check release

help:
	@echo "Usage: make <target> [LEVEL=patch|minor|major] [REGRESSION_THRESHOLD=20]"
	@echo ""
	@echo "  fix             Auto-fix formatting and clippy lints"
	@echo "  check           Fmt check + clippy + tests (validation)"
	@echo "  fmt             Format source code"
	@echo "  test            Run all tests"
	@echo "  build           Build the project"
	@echo "  doc             Build and open API documentation"
	@echo "  bench-baseline  Establish (or reset) the local benchmark baseline"
	@echo "  bench-check     Run benchmarks and report regressions vs baseline"
	@echo "  release         Preview changelog, confirm, then generate CHANGELOG and publish (LEVEL=patch)"

fix:
	cargo fmt
	cargo clippy --fix --allow-dirty --allow-staged --all-targets

check:
	cargo fmt -- --check
	cargo clippy --all-targets -- -D warnings
	cargo test

fmt:
	cargo fmt

test:
	time cargo test

build:
	cargo build

doc:
	cargo doc --no-deps --open

# Run all benchmarks and save results as the new "main" baseline.
# Run this once after cloning, or whenever you want to reset the reference point.
bench-baseline:
	@echo "=== Establishing benchmark baseline ==="
	cargo bench --bench pipeline -- --save-baseline current
	REGRESSION_THRESHOLD=$(REGRESSION_THRESHOLD) python3 scripts/check_benchmarks.py --save
	@echo "Done. Future 'make bench-check' and 'make release' will compare against this baseline."

# Run benchmarks as "current" and compare against the stored baseline.
# Exits non-zero if any benchmark regressed more than REGRESSION_THRESHOLD percent.
bench-check:
	@echo "=== Benchmark regression check (threshold: $(REGRESSION_THRESHOLD)%) ==="
	cargo bench --bench pipeline -- --save-baseline current
	REGRESSION_THRESHOLD=$(REGRESSION_THRESHOLD) python3 scripts/check_benchmarks.py

# bench-check runs before the interactive prompt so the developer sees benchmark
# results alongside the changelog preview before deciding to confirm the release.
# After a successful cargo release the "current" results are promoted to the new
# baseline so the next release compares against the just-shipped version.
release: fix check bench-check
	@printf "\n=== Release Preview (level: $(LEVEL)) ===\n\n"; \
	git-cliff --bump --unreleased 2>/dev/null || true; \
	printf "\nProceed with $(LEVEL) release? [y/N] " > /dev/tty; \
	read confirm < /dev/tty; \
	case "$$confirm" in \
		[yY]*) \
			cargo release $(LEVEL) --execute; \
			TAG=$$(git describe --tags --abbrev=0); \
			awk '/^## \[/{if(found) exit; found=1} found' CHANGELOG.md > /tmp/release-notes.md; \
			gh release create "$$TAG" --title "$$TAG" --notes-file /tmp/release-notes.md; \
			rm -f /tmp/release-notes.md; \
			REGRESSION_THRESHOLD=$(REGRESSION_THRESHOLD) python3 scripts/check_benchmarks.py --save \
		;; \
		*) echo "Aborted."; exit 1 ;; \
	esac
