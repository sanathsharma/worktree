release:
	cargo build --release
	cp target/release/worktree ~/.local/bin/

check:
	cargo check && cargo clippy

fmt:
	cargo fmt
