export CARGO_BUILD_JOBS ?= 4

.DEFAULT_GOAL := help
.PHONY: help build check fmt lint test test-unit test-integration test-doc \
        gallery doc package verify clean

help: ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN{FS=":.*?## "}{printf "  \033[36m%-18s\033[0m %s\n", $$1, $$2}'

build: ## Debug build (library)
	cargo build

check: ## Fast typecheck of all targets
	cargo check --all-targets

fmt: ## Apply formatting
	cargo fmt

lint: ## fmt-check + clippy with warnings as errors
	cargo fmt --check
	cargo clippy --all-targets -- -D warnings

test-unit: ## Unit tests (in-module #[cfg(test)])
	cargo test --lib

test-integration: ## Integration tests (tests/integration.rs; writes target/test-samples/*.svg)
	cargo test --test '*'

test-doc: ## Doctests (lib.rs examples)
	cargo test --doc

test: test-unit test-integration test-doc ## All tests (unit + integration + doctest)

gallery: ## Render sample boards to target/gallery/*.svg
	cargo run --example gallery

doc: ## Build rustdoc
	cargo doc --no-deps

package: ## Dry-run crates.io packaging
	cargo package --allow-dirty

verify: lint test ## Hard gate: lint + all tests — must pass before pushing

clean: ## Remove build artifacts
	cargo clean
