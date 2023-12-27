default:
	just --list --unsorted

fmt:
	cargo fmt --all
	taplo fmt

lint: fmt
	cargo clippy --all-targets --all-features

check:
	cargo fmt --all -- --check
	RUST_LOG=trace taplo fmt --check --diff
	cargo check --all-targets --all-features
	cargo clippy --all-targets --all-features -- -D warnings

check-windows:
	cargo check --all-targets --all-features
	cargo clippy --all-targets --all-features -- -D warnings

release-pr tag:
	git checkout -b "release-{{tag}}"
	git cliff --tag {{tag}} -o CHANGELOG.md
	# cargo-edit
	cargo set-version {{tag}}
	git commit -am "chore(release): {{tag}}"
	git push --set-upstream origin release-{{tag}}

push-tag tag:
	git tag {{tag}}
	git push origin {{tag}}
