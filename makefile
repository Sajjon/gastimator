run:
	cargo run --release --locked

help:
	cargo run --locked -- --help

utest:
	cargo nextest run --locked --workspace --lib

itest:
	cargo nextest run --locked --workspace --test "integration*"

test:
	$(MAKE) utest
	$(MAKE) itest

lint:
	cargo clippy --locked --all -- -D warnings

format:
	cargo fmt -- --check