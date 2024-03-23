default:
	just --list --unsorted

alias rp := release-pr
alias pt := push-tag

fmt:
	cargo fmt
	taplo fmt

lint: fmt
	cargo clippy --no-deps

check:
	cargo fmt -- --check
	taplo fmt --check
	cargo clippy --no-deps -- -D warnings

release-pr tag:
	git checkout -b "release-{{tag}}"
	git cliff --tag {{tag}} -o CHANGELOG.md
	cargo set-version {{tag}}
	git commit -am "chore(release): {{tag}}"
	git push --set-upstream origin release-{{tag}}

push-tag tag:
	git tag {{tag}}
	git push origin {{tag}}

run *args:
	cargo run -F _try -- {{args}}
