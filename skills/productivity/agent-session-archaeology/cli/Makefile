PREFIX ?= $(HOME)/.local

.PHONY: check install-local
check:
	cargo fmt --check
	cargo clippy --all-targets -- -D warnings
	cargo test

install-local:
	cargo build --release --locked
	install -d "$(PREFIX)/bin"
	install -m 755 target/release/sarch "$(PREFIX)/bin/sarch"
