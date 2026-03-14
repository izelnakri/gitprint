.DEFAULT_GOAL := help

LEVEL ?= patch
REGRESSION_THRESHOLD ?= 20

.PHONY: help fix check fmt test build doc bench-baseline bench-check release coverage

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
	@echo "  coverage        Run tests with coverage and fail if line coverage < 85%"
	@echo "  release         Preview changelog, confirm, then generate CHANGELOG and publish (LEVEL=patch)"

fix:
	cargo fmt
	cargo clippy --fix --allow-dirty --allow-staged --all-targets

check:
	cargo fmt -- --check
	cargo clippy --all-targets -- -D warnings
	@cargo nextest --version >/dev/null 2>&1 && cargo nextest run || cargo test

fmt:
	cargo fmt

test:
	@cargo nextest --version >/dev/null 2>&1 && cargo nextest run || cargo test

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

coverage:
	@cargo nextest --version >/dev/null 2>&1 && cargo llvm-cov nextest --html --open --fail-under-lines 85 || cargo llvm-cov --html --open --fail-under-lines 85

# Benchmarks run in the background while the developer reads the changelog
# preview — reading time is free CPU time. Results are checked after the
# developer confirms, before cargo-release runs. Aborts kill the background
# process cleanly. `fix` is intentionally excluded: auto-modifying code at
# release time is unsafe; run `make fix` manually if check fails.
# If not already inside a nix shell, re-enters via `nix develop` so git-cliff
# and cargo-release are available. `exec` replaces the shell process, preventing
# any subsequent commands from running outside nix.
release:
	@if [ -z "$$IN_NIX_SHELL" ]; then \
		echo "==> Entering nix develop (git-cliff, cargo-release)..."; \
		exec nix develop --command $(MAKE) release LEVEL=$(LEVEL) REGRESSION_THRESHOLD=$(REGRESSION_THRESHOLD); \
	fi; \
	$(MAKE) check; \
	cargo bench --bench pipeline -- --save-baseline current > /tmp/gitprint-bench.log 2>&1 & BENCH_PID=$$!; \
	printf "\n=== Release Preview (level: $(LEVEL)) ===\n\n"; \
	git-cliff --bump --unreleased 2>/dev/null || true; \
	printf "\nProceed with $(LEVEL) release? [y/N] " > /dev/tty; \
	read confirm < /dev/tty; \
	case "$$confirm" in \
		[yY]*) \
			printf "\n=== Benchmark results ===\n"; \
			tail -f /tmp/gitprint-bench.log & TAIL_PID=$$!; \
			wait $$BENCH_PID; BENCH_EXIT=$$?; \
			kill $$TAIL_PID 2>/dev/null; wait $$TAIL_PID 2>/dev/null; \
			[ $$BENCH_EXIT -eq 0 ] || { printf "\nBenchmark run failed.\n"; rm -f /tmp/gitprint-bench.log; exit 1; }; \
			REGRESSION_THRESHOLD=$(REGRESSION_THRESHOLD) python3 scripts/check_benchmarks.py || { rm -f /tmp/gitprint-bench.log; exit 1; }; \
			cargo release $(LEVEL) --execute; \
			TAG=$$(git describe --tags --abbrev=0); \
			awk '/^## \[/{if(found) exit; found=1} found' CHANGELOG.md > /tmp/release-notes.md; \
			gh release create "$$TAG" --title "$$TAG" --notes-file /tmp/release-notes.md; \
			rm -f /tmp/release-notes.md /tmp/gitprint-bench.log; \
			REGRESSION_THRESHOLD=$(REGRESSION_THRESHOLD) python3 scripts/check_benchmarks.py --save \
		;; \
		*) printf "Aborted.\n"; kill $$BENCH_PID 2>/dev/null; rm -f /tmp/gitprint-bench.log; exit 1 ;; \
	esac

demo:
	vhs demo/demo.tape

