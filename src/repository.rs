use std::{
	fs, io,
	path::{Path, PathBuf},
	sync::OnceLock,
};

use anyhow::{anyhow, Context, Result};
use flate2::read::GzDecoder;
use regex::Regex;
use remove_dir_all::remove_dir_all;

use crate::{debug, error::ScafalraError, utils::build_proxy_agent};

static REPO_RE: OnceLock<Regex> = OnceLock::new();

fn get_repo_re() -> &'static Regex {
	REPO_RE.get_or_init(|| {
		Regex::new(r"^([^/\s]+)/([^/\s?]+)(?:((?:/[^/\s?]+)+))?(?:\?(branch|tag|commit)=([^\s]+))?$").unwrap()
	})
}

pub struct Repository {
	pub owner: String,
	pub name: String,
	pub subdir: Option<String>,
	pub query: Option<Query>,
}

#[derive(PartialEq, Debug)]
pub enum Query {
	Branch(String),
	Tag(String),
	Commit(String),
}

impl Repository {
	pub fn new(input: &str) -> Result<Self> {
		let caps = get_repo_re().captures(input).ok_or(anyhow!(
			ScafalraError::RepositoryParseError(input.to_string())
		))?;

		let owner = caps[1].to_string();
		let name = caps[2].to_string();

		let subdir = caps.get(3).map(|v| v.as_str().to_string());
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
		cache_dir: &Path,
		oid: &str,
	) -> Result<PathBuf> {
		let scaffold_path = cache_dir.join(format!(
			"{}-{}-{}",
			self.owner,
			self.name,
			&oid[0..7]
		));

		debug!("scaffold directory: {:?}", scaffold_path);

		let tarball_path = scaffold_path.with_extension("tar.gz");

		if tarball_path.exists() {
			fs::remove_file(&tarball_path)?;
		}

		download(url, &tarball_path)
			.context(ScafalraError::IOError(tarball_path.clone()))?;

		let temp_dir_path = cache_dir.join(oid);

		unpack(&tarball_path, &temp_dir_path)?;

		// There will only be one folder in this directory, which is the
		// extracted repository
		let extracted_dir = temp_dir_path.read_dir()?.next().unwrap()?;

		debug!("extracted directory: {:?}", extracted_dir);

		if scaffold_path.exists() {
			remove_dir_all(&scaffold_path)?;
		}

		fs::rename(extracted_dir.path(), &scaffold_path)?;

		fs::remove_file(&tarball_path)?;

		remove_dir_all(temp_dir_path)?;

		Ok(scaffold_path)
	}
}

fn download(url: &str, file_path: &Path) -> Result<()> {
	let agent = build_proxy_agent();
	let response = agent.get(url).call()?;
	let mut file = fs::File::create(file_path)?;

	io::copy(&mut response.into_reader(), &mut file)?;

	Ok(())
}

fn unpack(file_path: &Path, parent_dir: &Path) -> Result<()> {
	let file = fs::File::open(file_path)?;
	let dec = GzDecoder::new(file);
	let mut tar = tar::Archive::new(dec);

	tar.unpack(parent_dir)?;

	Ok(())
}

#[cfg(test)]
mod tests {
	use std::{fs, path::PathBuf};

	use anyhow::Result;
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
		let repo = Repository::new("foo/bar/path/to/dir?branch=main")?;

		assert_eq!(repo.owner, "foo");
		assert_eq!(repo.name, "bar");
		assert_eq!(repo.subdir.unwrap(), "/path/to/dir");
		assert_eq!(repo.query.unwrap(), Query::Branch("main".to_string()));

		Ok(())
	}

	#[test]
	fn test_repo_new_err() {
		let repo = Repository::new("foo");
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

		let dir = tempfile::tempdir()?;
		let oid = "ea7c165bac336140bcf08f84758ab752769799be";

		let repo = Repository::new("shixinhuang99/scafalra")?;
		repo.cache(&server.url(), dir.path(), oid)?;

		let repo_path = dir.path().join("shixinhuang99-scafalra-ea7c165");

		mock.assert();
		assert!(repo_path.exists());
		assert!(repo_path.is_dir());
		assert!(!repo_path.with_extension("tar.gz").exists());

		Ok(())
	}
}
