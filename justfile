default:
	just --list --unsorted

fmt:
	cargo fmt --all
	taplo fmt

lint: fmt
	cargo clippy --all-targets --all-features

check:
	cargo fmt --all -- --check
	taplo fmt --check
	cargo clippy --all-targets --all-features -- -D warnings
	cargo check --all-targets --all-features

alias br := build-release

build-release:
	cargo build --release
