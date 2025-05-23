.PHONY: all
all: test doc

test:
	TMPDIR=~/tmp cargo test

doc:
	cargo doc --no-deps --document-private-items

doc-open:
	cargo doc --no-deps --document-private-items --open

clean:
	cargo clean

check:
	cargo check

release:
	cargo build --release

run:
	cargo run

fmt:
	cargo fmt

lint:
	cargo clippy -- -D warnings

pre-publish: fmt lint check test doc
	git diff --exit-code
	cargo publish -p rust-automata-macros --dry-run