use std::sync::OnceLock;

use anyhow::Result;
use regex::Regex;

fn repo_re() -> &'static Regex {
	static REPO_RE: OnceLock<Regex> = OnceLock::new();

	REPO_RE.get_or_init(|| {
		let re = r"^(?:https://github\.com/)?([^/\s]+)/([^/\s]+)$";
		Regex::new(re).unwrap()
	})
}

#[derive(Default)]
pub struct Repository {
	pub owner: String,
	pub name: String,
}

impl Repository {
	pub fn parse(input: &str) -> Result<Self> {
		let caps = repo_re()
			.captures(input)
			.ok_or(anyhow::anyhow!("Could not parse the input: `{}`", input))?;

		let owner = caps[1].to_string();
		let mut name = caps[2].to_string();

		if name.ends_with(".git") {
			name.truncate(name.len() - 4);
		}

		Ok(Self {
			owner,
			name,
		})
	}

	pub fn url(&self) -> String {
		if cfg!(test) {
			"url".to_string()
		} else {
			format!("https://github.com/{}/{}", &self.owner, &self.name)
		}
	}
}

#[cfg(test)]
mod tests {
	use anyhow::Result;
	use test_case::test_case;

	use super::Repository;

	#[test_case("foo/bar"; "basic")]
	#[test_case("https://github.com/foo/bar.git"; "complete url")]
	#[test_case("foo/bar.git"; "url but no header")]
	#[test_case("https://github.com/foo/bar"; "url but no extension")]
	fn test_repo_parse_basic(input: &str) -> Result<()> {
		let repo = Repository::parse(input)?;

		assert_eq!(repo.owner, "foo");
		assert_eq!(repo.name, "bar");

		Ok(())
	}

	#[test_case(""; "empty")]
	#[test_case("foo"; "incomplete")]
	#[test_case("foo/bar/baz"; "paths exceeded")]
	fn test_repo_parse_err(input: &str) {
		let repo = Repository::parse(input);
		assert!(repo.is_err());
	}
}
