.PHONY: lint test check test-contract test-compat fmt clippy

fmt:
	cargo fmt --all -- --check

clippy:
	cargo clippy --workspace --all-targets --all-features -- -D warnings

lint: fmt clippy

test:
	cargo test --workspace --all-features

test-contract:
	cargo test --workspace --all-features --test '*' contract::

test-compat:
	cargo test --workspace --all-features --test '*' compatibility::

check: lint test
