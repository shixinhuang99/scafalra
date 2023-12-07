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
	cargo check --all-targets --all-features
	cargo clippy --all-targets --all-features -- -D warnings

check-windows:
	cargo check --all-targets --all-features
	cargo clippy --all-targets --all-features -- -D warnings

alias br := build-release

build-release:
	cargo build --release

release tag:
	git cliff --tag {{tag}} -o CHANGELOG.md
	git commit -am "chore(release): {{tag}}"
	git tag {{tag}}
