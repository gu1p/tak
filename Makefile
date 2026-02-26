.PHONY: check test lint fmt-check docs-check

check: fmt-check lint test docs-check

fmt-check:
	cargo fmt --all -- --check

lint:
	cargo clippy --workspace --all-targets -- -D warnings

test:
	cargo test --workspace

docs-check:
	cargo test --workspace --doc
	cargo test -p taskcraft --test doctest_contract
