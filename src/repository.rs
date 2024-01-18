use std::{
	path::{Path, PathBuf},
	sync::OnceLock,
};

use anyhow::Result;
use fs_err as fs;
use regex::Regex;
use remove_dir_all::remove_dir_all;

use crate::{
	debug,
	path_ext::*,
	utils::{download, tar_unpack},
};

static REPO_RE: OnceLock<Regex> = OnceLock::new();

fn repo_re() -> &'static Regex {
	REPO_RE.get_or_init(|| {
		Regex::new(
			r"^([^/\s]+)/([^/\s?]+)(?:((?:/[^/\s?]+)+))?(?:\?(branch|tag|commit)=([^\s]+))?$",
		)
		.unwrap()
	})
}

#[derive(Default)]
pub struct Repository {
	pub owner: String,
	pub name: String,
	pub subdir: Option<PathBuf>,
	pub query: Option<Query>,
}

#[derive(PartialEq, Debug)]
pub enum Query {
	Branch(String),
	Tag(String),
	Commit(String),
}

impl Repository {
	pub const TMP_REPO_DIR_NAME: &'static str = "t";
	pub const TARBALL_EXT: &'static str = "tar.gz";

	pub fn parse(input: &str) -> Result<Self> {
		let caps = repo_re()
			.captures(input)
			.ok_or(anyhow::anyhow!("Could not parse the input: `{}`", input))?;

		let owner = caps[1].to_string();
		let name = caps[2].to_string();

		let subdir = caps.get(3).map(|v| PathBuf::from(v.as_str()));
		let query_kind = caps.get(4).map(|v| v.as_str());
		let query_val = caps.get(5).map(|v| v.as_str().to_string());

		let query = match (query_kind, query_val) {
			(Some("branch"), Some(val)) => Some(Query::Branch(val)),
			(Some("tag"), Some(val)) => Some(Query::Tag(val)),
			(Some("commit"), Some(val)) => Some(Query::Commit(val)),
			_ => None,
		};

		Ok(Self {
			owner,
			name,
			subdir,
			query,
		})
	}

	pub fn cache(&self, url: &str, cache_dir: &Path) -> Result<PathBuf> {
		let temp_dir = cache_dir.join(Self::TMP_REPO_DIR_NAME);
		let tarball = temp_dir.with_extension(Self::TARBALL_EXT);

		download(url, &tarball)?;

		tar_unpack(&tarball, &temp_dir)?;

		let first_inner_dir = temp_dir
			.read_dir()?
			.next()
			.ok_or(anyhow::anyhow!("Empty directory"))??
			.path();

		debug!("first_inner_dir: {:?}", first_inner_dir);

		let template_dir = cache_dir.join_iter([&self.owner, &self.name]);

		if template_dir.exists() {
			remove_dir_all(&template_dir)?;
		}

		dircpy::copy_dir(first_inner_dir, &template_dir)?;

		remove_dir_all(temp_dir)?;

		fs::remove_file(tarball)?;

		Ok(template_dir)
	}
}

#[cfg(test)]
mod tests {
	use anyhow::Result;

	use super::{repo_re, Query, Repository};
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
	fn test_repo_re_branch() {
		let caps = repo_re().captures("foo/bar?branch=main");
		assert!(caps.is_some());
		let caps = caps.unwrap();
		assert_eq!(&caps[1], "foo");
		assert_eq!(&caps[2], "bar");
		assert_eq!(caps.get(3), None);
		assert_eq!(&caps[4], "branch");
		assert_eq!(&caps[5], "main");
	}

	#[test]
	fn test_repo_re_tag() {
		let caps = repo_re().captures("foo/bar?tag=v1.0.0");
		assert!(caps.is_some());
		let caps = caps.unwrap();
		assert_eq!(&caps[1], "foo");
		assert_eq!(&caps[2], "bar");
		assert_eq!(caps.get(3), None);
		assert_eq!(&caps[4], "tag");
		assert_eq!(&caps[5], "v1.0.0");
	}

	#[test]
	fn test_repo_re_commit() {
		let caps = repo_re().captures("foo/bar?commit=abc123");
		assert!(caps.is_some());
		let caps = caps.unwrap();
		assert_eq!(&caps[1], "foo");
		assert_eq!(&caps[2], "bar");
		assert_eq!(caps.get(3), None);
		assert_eq!(&caps[4], "commit");
		assert_eq!(&caps[5], "abc123");
	}

	#[test]
	fn test_repo_re_query_empty() {
		let caps = repo_re().captures("foo/bar?commit= ");
		assert!(caps.is_none());
	}

	#[test]
	fn test_repo_re_full() {
		let caps = repo_re().captures("foo/bar/path/to/dir?branch=main");
		assert!(caps.is_some());
		let caps = caps.unwrap();
		assert_eq!(&caps[1], "foo");
		assert_eq!(&caps[2], "bar");
		assert_eq!(caps.get(3).unwrap().as_str(), "/path/to/dir");
		assert_eq!(&caps[4], "branch");
		assert_eq!(&caps[5], "main");
	}

	#[test]
	fn test_repo_re_none_match() {
		let caps = repo_re().captures("foo");
		assert!(caps.is_none());
	}

	#[test]
	fn test_repo_new() -> Result<()> {
		let repo = Repository::parse("foo/bar/path/to/dir?branch=main")?;

		assert_eq!(repo.owner, "foo");
		assert_eq!(repo.name, "bar");
		assert_eq!(
			repo.subdir.unwrap().to_string_lossy().to_string(),
			"/path/to/dir"
		);
		assert_eq!(repo.query.unwrap(), Query::Branch("main".to_string()));

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

		let tmp_repo_dir = temp_dir_path.join(Repository::TMP_REPO_DIR_NAME);
		let tarball = tmp_repo_dir.with_extension(Repository::TARBALL_EXT);

		mock.assert();
		assert!(temp_dir_path.join_slash("shixinhuang99/scafalra").is_dir());
		assert!(!tmp_repo_dir.exists());
		assert!(!tarball.exists());

		Ok(())
	}
}
