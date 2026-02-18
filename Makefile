.DEFAULT_GOAL := help

LEVEL ?= patch

.PHONY: help fix check fmt test build doc release

help:
	@echo "Usage: make <target> [LEVEL=patch|minor|major]"
	@echo ""
	@echo "  fix      Auto-fix formatting and clippy lints"
	@echo "  check    Fmt check + clippy + tests (validation)"
	@echo "  fmt      Format source code"
	@echo "  test     Run all tests"
	@echo "  build    Build the project"
	@echo "  doc      Build and open API documentation"
	@echo "  release  Preview changelog, confirm, then generate CHANGELOG and publish (LEVEL=patch)"

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

release: fix check
	@printf "\n=== Release Preview (level: $(LEVEL)) ===\n\n"; \
	git-cliff --bump --unreleased 2>/dev/null || true; \
	printf "\nProceed with $(LEVEL) release? [y/N] "; \
	read confirm; \
	if [ "$$confirm" = "y" ] || [ "$$confirm" = "Y" ]; then \
		git-cliff --bump -o CHANGELOG.md && \
		git add CHANGELOG.md && \
		cargo release $(LEVEL) --execute; \
	else \
		echo "Aborted."; \
		exit 1; \
	fi
