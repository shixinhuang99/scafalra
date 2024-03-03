use std::sync::OnceLock;

use anyhow::Result;
use regex::Regex;

fn repo_re() -> &'static Regex {
	static REPO_RE: OnceLock<Regex> = OnceLock::new();

	REPO_RE.get_or_init(|| {
		Regex::new(
			r"^(?:https://github\.com/)?([^/\s]+)/([^/\s.git]+)(?:\.git)?$",
		)
		.unwrap()
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
		let name = caps[2].to_string();

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

	use super::Repository;

	#[test]
	fn test_repo_parse() -> Result<()> {
		let repo = Repository::parse("foo/bar")?;

		assert_eq!(repo.owner, "foo");
		assert_eq!(repo.name, "bar");

		Ok(())
	}

	#[test]
	fn test_repo_parse_git_url() -> Result<()> {
		let repo = Repository::parse("https://github.com/foo/bar.git")?;
		assert_eq!(repo.owner, "foo");
		assert_eq!(repo.name, "bar");

		Ok(())
	}

	#[test]
	fn test_repo_parse_err() {
		let repo = Repository::parse("foo");
		assert!(repo.is_err());
	}
}
