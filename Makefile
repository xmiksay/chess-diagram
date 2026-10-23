export CARGO_BUILD_JOBS ?= 4

.DEFAULT_GOAL := help
.PHONY: help build check fmt lint test test-unit test-integration test-doc \
        gallery doc package verify clean golden-update

help: ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN{FS=":.*?## "}{printf "  \033[36m%-18s\033[0m %s\n", $$1, $$2}'

build: ## Debug build (library)
	cargo build

check: ## Fast typecheck of all targets (all features)
	cargo check --all-targets --all-features

fmt: ## Apply formatting
	cargo fmt

lint: ## fmt-check + clippy (default and all features) with warnings as errors
	cargo fmt --check
	cargo clippy --all-targets -- -D warnings
	cargo clippy --all-targets --all-features -- -D warnings

test-unit: ## Unit tests (in-module #[cfg(test)], all features)
	cargo test --lib --all-features

test-integration: ## Integration tests (tests/integration.rs, all features; writes target/test-samples/*.svg)
	cargo test --test '*' --all-features

test-doc: ## Doctests (all features)
	cargo test --doc --all-features

test: test-unit test-integration test-doc ## All tests (unit + integration + doctest)

golden-update: ## Rewrite tests/golden/*.svg fixtures from the current renderer output
	UPDATE_GOLDEN=1 cargo test --test '*'

gallery: ## Regenerate the README gallery under assets/gallery/*.svg
	cargo run --example gallery

doc: ## Build rustdoc (all features, warnings as errors — same as CI and docs.rs)
	RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --lib --all-features

package: ## Dry-run crates.io packaging
	cargo package --allow-dirty

verify: lint test ## Hard gate: lint + all tests — must pass before pushing

clean: ## Remove build artifacts
	cargo clean
