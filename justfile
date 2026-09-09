release:
	cargo build --release
	cp target/release/worktree ~/.local/bin/
