.PHONY: test fmt clippy

test:
	cargo test --workspace

fmt:
	cargo fmt --all -- --check

clippy:
	cargo clippy --workspace -- -D warnings
