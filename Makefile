.PHONY: all
all: test doc

test:
	TMPDIR=~/tmp cargo test

doc:
	RUSTDOCFLAGS="--html-in-header ./examples/assets/graphviz-header.html" cargo doc --no-deps

doc-open:
	RUSTDOCFLAGS="--html-in-header ./examples/assets/graphviz-header.html" cargo doc --no-deps --open

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