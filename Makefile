.DEFAULT_GOAL := help

LEVEL ?= patch

.PHONY: help check fmt test build doc release

help:
	@echo "Usage: make <target> [LEVEL=patch|minor|major]"
	@echo ""
	@echo "  check    Fmt check + clippy + tests (run before every commit)"
	@echo "  fmt      Format source code"
	@echo "  test     Run all tests"
	@echo "  build    Build the project"
	@echo "  doc      Build and open API documentation"
	@echo "  release  Generate CHANGELOG and publish (LEVEL=patch)"

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

release:
	git-cliff --bump -o CHANGELOG.md
	cargo release $(LEVEL) --execute
