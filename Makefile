.PHONY: check test lint fmt-check docs-check line-limits-check src-test-separation-check workflow-contract-check

check: fmt-check line-limits-check src-test-separation-check workflow-contract-check lint test docs-check

fmt-check:
	cargo fmt --all -- --check

lint:
	cargo clippy --workspace --all-targets -- -D warnings

test:
	cargo test --workspace

docs-check:
	cargo test --workspace --doc
	cargo test -p tak --test doctest_contract

line-limits-check:
	scripts/check_rust_file_limits.sh

src-test-separation-check:
	scripts/check_no_tests_in_src.sh

workflow-contract-check:
	scripts/check_workflow_binary_matrix.sh
