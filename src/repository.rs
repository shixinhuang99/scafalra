use std::{
	path::{Path, PathBuf},
	sync::OnceLock,
};

use anyhow::Result;
use regex::Regex;
use remove_dir_all::remove_dir_all;

use crate::{debug, path_ext::*, utils::Downloader};

fn repo_re() -> &'static Regex {
	static REPO_RE: OnceLock<Regex> = OnceLock::new();

	REPO_RE.get_or_init(|| {
		Regex::new(
			r"^(?:https://github\.com/)?([^/\s]+)/([^/\s?]+)(?:((?:/[^/\s?]+)+))?(?:\.git)?$",
		)
		.unwrap()
	})
}

#[derive(Default)]
pub struct Repository {
	pub owner: String,
	pub name: String,
	pub subdir: Option<PathBuf>,
}

impl Repository {
	pub const TMP_DIR_NAME: &'static str = "t";

	pub fn parse(input: &str) -> Result<Self> {
		let caps = repo_re()
			.captures(input)
			.ok_or(anyhow::anyhow!("Could not parse the input: `{}`", input))?;

		let owner = caps[1].to_string();
		let name = caps[2].to_string();
		let subdir = caps.get(3).map(|v| PathBuf::from(v.as_str()));

		Ok(Self {
			owner,
			name,
			subdir,
		})
	}

	pub fn cache(&self, url: &str, cache_dir: &Path) -> Result<PathBuf> {
		let tmp_dir = cache_dir.join(Self::TMP_DIR_NAME);

		Downloader::new(url, &tmp_dir, "tar.gz")
			.download()?
			.tar_unpack(&tmp_dir)?;

		let first_dir = tmp_dir
			.read_dir()?
			.next()
			.ok_or(anyhow::anyhow!("Empty directory"))??
			.path();

		debug!("first_dir: {:?}", first_dir);

		let template_dir = cache_dir.join_iter([&self.owner, &self.name]);

		if template_dir.exists() {
			remove_dir_all(&template_dir)?;
		}

		dircpy::copy_dir(first_dir, &template_dir)?;

		remove_dir_all(tmp_dir)?;

		Ok(template_dir)
	}
}

#[cfg(test)]
pub mod test_utils {
	use super::Repository;

	pub struct RepositoryMock {
		owner: String,
		name: String,
	}

	impl RepositoryMock {
		pub fn new() -> Self {
			Self {
				owner: "shixinhuang99".to_string(),
				name: "scafalra".to_string(),
			}
		}

		pub fn build(self) -> Repository {
			Repository {
				owner: self.owner,
				name: self.name,
				..Repository::default()
			}
		}

		pub fn owner(self, owner: &str) -> Self {
			Self {
				owner: owner.to_string(),
				..self
			}
		}

		pub fn name(self, name: &str) -> Self {
			Self {
				name: name.to_string(),
				..self
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use std::path::PathBuf;

	use anyhow::Result;

	use super::{repo_re, Repository};
	use crate::path_ext::*;

	#[test]
	fn test_repo_re_basic() {
		let caps = repo_re().captures("foo/bar");
		assert!(caps.is_some());
		let caps = caps.unwrap();
		assert_eq!(&caps[1], "foo");
		assert_eq!(&caps[2], "bar");
	}

	#[test]
	fn test_repo_re_subdir() {
		let caps = repo_re().captures("foo/bar/path/to/dir");
		assert!(caps.is_some());
		let caps = caps.unwrap();
		assert_eq!(&caps[1], "foo");
		assert_eq!(&caps[2], "bar");
		assert_eq!(&caps[3], "/path/to/dir");
	}

	#[test]
	fn test_repo_re_none_match() {
		let caps = repo_re().captures("foo");
		assert!(caps.is_none());
	}

	#[test]
	fn test_repo_new() -> Result<()> {
		let repo = Repository::parse("foo/bar")?;

		assert_eq!(repo.owner, "foo");
		assert_eq!(repo.name, "bar");

		Ok(())
	}

	#[test]
	fn test_repo_new_subdir() -> Result<()> {
		let repo = Repository::parse("foo/bar/path/to/dir")?;
		assert_eq!(repo.owner, "foo");
		assert_eq!(repo.name, "bar");
		assert_eq!(repo.subdir, Some(PathBuf::from("/path/to/dir")));

		Ok(())
	}

	#[test]
	fn test_repo_new_git_url() -> Result<()> {
		let repo =
			Repository::parse("https://github.com/foo/bar/path/to/dir.git")?;
		assert_eq!(repo.owner, "foo");
		assert_eq!(repo.name, "bar");
		assert_eq!(repo.subdir, Some(PathBuf::from("/path/to/dir")));

		Ok(())
	}

	#[test]
	fn test_repo_new_err() {
		let repo = Repository::parse("foo");
		assert!(repo.is_err());
	}

	#[test]
	fn test_repo_cache() -> Result<()> {
		let mut server = mockito::Server::new();

		let mock = server
			.mock("GET", "/")
			.with_status(200)
			.with_header("content-type", "application/x-gzip")
			.with_body_from_file("fixtures/scafalra-test.tar.gz")
			.create();

		let temp_dir = tempfile::tempdir()?;
		let temp_dir_path = temp_dir.path();

		let repo = Repository::parse("shixinhuang99/scafalra")?;
		repo.cache(&server.url(), temp_dir_path)?;

		let tmp_repo_dir = temp_dir_path.join(Repository::TMP_DIR_NAME);
		let tarball = tmp_repo_dir.with_extension("tar.gz");

		mock.assert();
		assert!(temp_dir_path.join_slash("shixinhuang99/scafalra").is_dir());
		assert!(!tmp_repo_dir.exists());
		assert!(!tarball.exists());

		Ok(())
	}
}
