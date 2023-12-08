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

release tag:
	git checkout -b "release-{{tag}}"
	git cliff --tag {{tag}} -o CHANGELOG.md
	git commit -am "chore(release): {{tag}}"
	git tag {{tag}}

push-tag tag:
	git push orirgin {{tag}}
