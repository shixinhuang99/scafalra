use std::sync::OnceLock;

use anyhow::{anyhow, Result};
use camino::{Utf8Path, Utf8PathBuf};
use fs_err as fs;
use regex::Regex;
use remove_dir_all::remove_dir_all;

use crate::{
	debug,
	error::ScafalraError,
	utils::{download, tar_unpack},
};

static REPO_RE: OnceLock<Regex> = OnceLock::new();

fn get_repo_re() -> &'static Regex {
	REPO_RE.get_or_init(|| {
		Regex::new(r"^([^/\s]+)/([^/\s?]+)(?:((?:/[^/\s?]+)+))?(?:\?(branch|tag|commit)=([^\s]+))?$").unwrap()
	})
}

pub struct Repository {
	pub owner: String,
	pub name: String,
	pub subdir: Option<Utf8PathBuf>,
	pub query: Option<Query>,
}

#[derive(PartialEq, Debug)]
pub enum Query {
	Branch(String),
	Tag(String),
	Commit(String),
}

impl Repository {
	pub fn parse(input: &str) -> Result<Self> {
		let caps = get_repo_re().captures(input).ok_or(anyhow!(
			ScafalraError::RepositoryParseError(input.to_string())
		))?;

		let owner = caps[1].to_string();
		let name = caps[2].to_string();

		let subdir = caps.get(3).map(|v| Utf8PathBuf::from(v.as_str()));
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

	pub fn cache(
		&self,
		url: &str,
		cache_dir: &Utf8Path,
	) -> Result<Utf8PathBuf> {
		let temp_dir = cache_dir.join("t");
		let tarball = temp_dir.with_extension("tar.gz");

		download(url, &tarball)?;

		tar_unpack(&tarball, &temp_dir)?;

		let Some(extracted_dir) = temp_dir.read_dir_utf8()?.next() else {
			anyhow::bail!("Empty directory");
		};

		let extracted_dir = extracted_dir?.into_path();

		debug!("extracted directory: {}", extracted_dir);

		let scaffold_dir = Utf8PathBuf::from_iter([
			cache_dir,
			Utf8Path::new(&self.owner),
			Utf8Path::new(&self.name),
		]);

		if scaffold_dir.exists() {
			remove_dir_all(&scaffold_dir)?;
		}

		dircpy::copy_dir(extracted_dir, &scaffold_dir)?;

		fs::remove_file(&tarball)?;
		remove_dir_all(temp_dir)?;

		Ok(scaffold_dir)
	}
}

#[cfg(test)]
mod tests {
	use std::{fs, path::PathBuf};

	use anyhow::Result;
	use camino::Utf8Path;
	use pretty_assertions::assert_eq;

	use super::{get_repo_re, Query, Repository};

	#[test]
	fn test_repo_re_basic() {
		let caps = get_repo_re().captures("foo/bar");
		assert!(caps.is_some());
		let caps = caps.unwrap();
		assert_eq!(&caps[1], "foo");
		assert_eq!(&caps[2], "bar");
	}

	#[test]
	fn test_repo_re_subdir() {
		let caps = get_repo_re().captures("foo/bar/path/to/dir");
		assert!(caps.is_some());
		let caps = caps.unwrap();
		assert_eq!(&caps[1], "foo");
		assert_eq!(&caps[2], "bar");
		assert_eq!(&caps[3], "/path/to/dir");
	}

	#[test]
	fn test_repo_re_branch() {
		let caps = get_repo_re().captures("foo/bar?branch=main");
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
		let caps = get_repo_re().captures("foo/bar?tag=v1.0.0");
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
		let caps = get_repo_re().captures("foo/bar?commit=abc123");
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
		let caps = get_repo_re().captures("foo/bar?commit= ");
		assert!(caps.is_none());
	}

	#[test]
	fn test_repo_re_full() {
		let caps = get_repo_re().captures("foo/bar/path/to/dir?branch=main");
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
		let caps = get_repo_re().captures("foo");
		assert!(caps.is_none());
	}

	#[test]
	fn test_repo_new() -> Result<()> {
		let repo = Repository::parse("foo/bar/path/to/dir?branch=main")?;

		assert_eq!(repo.owner, "foo");
		assert_eq!(repo.name, "bar");
		assert_eq!(repo.subdir.unwrap(), "/path/to/dir");
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
		use std::io::Read;

		let mut server = mockito::Server::new();
		let file_path = PathBuf::from_iter(["assets", "scafalra-test.tar.gz"]);
		let mut file = fs::File::open(file_path)?;
		let mut data = Vec::new();
		file.read_to_end(&mut data)?;

		let mock = server
			.mock("GET", "/")
			.with_status(200)
			.with_header("content-type", "application/x-gzip")
			.with_body(data)
			.create();

		let temp_dir = tempfile::tempdir()?;
		let temp_dir_path = Utf8Path::from_path(temp_dir.path()).unwrap();

		let repo = Repository::parse("shixinhuang99/scafalra")?;
		repo.cache(&server.url(), temp_dir_path)?;

		mock.assert();
		assert!(
			temp_dir_path
				.join("shixinhuang99")
				.join("scafalra")
				.is_dir()
		);
		assert!(!temp_dir_path.join("t").exists());

		Ok(())
	}
}
