use std::{
    fs, io,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, Result};
use flate2::read::GzDecoder;
use once_cell::sync::Lazy;
use regex::Regex;
use remove_dir_all::remove_dir_all;

use crate::utils::build_proxy_agent;

static REPO_RE: Lazy<Regex> = Lazy::new(|| {
    let re = r"^([^/\s]+)/([^/\s?]+)(?:((?:/[^/\s?]+)+))?(?:\?(branch|tag|commit)=([^\s]+))?$";

    Regex::new(&re).unwrap()
});

pub struct Repository {
    pub owner: String,
    pub name: String,
    pub subdir: Option<String>,
    pub query: Option<Query>,
    pub input: String,
}

#[derive(PartialEq, Debug)]
pub enum Query {
    BRANCH(String),
    TAG(String),
    COMMIT(String),
}

impl Repository {
    pub fn new(input: &str) -> Result<Self> {
        let caps = REPO_RE
            .captures(input)
            .ok_or(anyhow!("Could not parse the input: '{}'", input))?;

        let owner = (&caps[1]).to_string();
        let name = (&caps[2]).to_string();

        let subdir = caps.get(3).map_or(None, |v| Some(v.as_str().to_string()));
        let query_type = caps.get(4).map_or(None, |v| Some(v.as_str()));
        let query_val =
            caps.get(5).map_or(None, |v| Some(v.as_str().to_string()));

        let query = match (query_type, query_val) {
            (Some("branch"), Some(val)) => Some(Query::BRANCH(val)),
            (Some("tag"), Some(val)) => Some(Query::TAG(val)),
            (Some("commit"), Some(val)) => Some(Query::COMMIT(val)),
            _ => None,
        };

        Ok(Self {
            owner,
            name,
            subdir,
            query,
            input: input.to_string(),
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

        let tarball_path = scaffold_path.with_extension("tar.gz");

        if tarball_path.exists() {
            fs::remove_file(&tarball_path)?;
        }

        download(url, &tarball_path)?;

        let temp_dir_path = cache_dir.join(oid);

        unpack(&tarball_path, &temp_dir_path)?;

        // There will only be one folder in this directory, which is the
        // extracted repository
        let extracted_dir = temp_dir_path.read_dir()?.next().unwrap()?;

        if scaffold_path.exists() {
            remove_dir_all(&scaffold_path)?;
        }

        fs::rename(extracted_dir.path(), &scaffold_path)?;

        fs::remove_file(&tarball_path)?;

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

    use super::{Query, Repository, REPO_RE};

    #[test]
    fn repo_re_basic() {
        let caps = REPO_RE.captures("test/repository");
        assert!(caps.is_some());
        let caps = caps.unwrap();
        assert_eq!(&caps[1], "test");
        assert_eq!(&caps[2], "repository");
    }

    #[test]
    fn repo_re_subdir() {
        let caps = REPO_RE.captures("test/repository/path/to/dir");
        assert!(caps.is_some());
        let caps = caps.unwrap();
        assert_eq!(&caps[1], "test");
        assert_eq!(&caps[2], "repository");
        assert_eq!(&caps[3], "/path/to/dir");
    }

    #[test]
    fn repo_re_query_branch() {
        let caps = REPO_RE.captures("test/repository?branch=main");
        assert!(caps.is_some());
        let caps = caps.unwrap();
        assert_eq!(&caps[1], "test");
        assert_eq!(&caps[2], "repository");
        assert_eq!(caps.get(3), None);
        assert_eq!(&caps[4], "branch");
        assert_eq!(&caps[5], "main");
    }

    #[test]
    fn repo_re_query_tag() {
        let caps = REPO_RE.captures("test/repository?tag=v1.0.0");
        assert!(caps.is_some());
        let caps = caps.unwrap();
        assert_eq!(&caps[1], "test");
        assert_eq!(&caps[2], "repository");
        assert_eq!(caps.get(3), None);
        assert_eq!(&caps[4], "tag");
        assert_eq!(&caps[5], "v1.0.0");
    }

    #[test]
    fn repo_re_query_commit() {
        let caps = REPO_RE.captures("test/repository?commit=abc123");
        assert!(caps.is_some());
        let caps = caps.unwrap();
        assert_eq!(&caps[1], "test");
        assert_eq!(&caps[2], "repository");
        assert_eq!(caps.get(3), None);
        assert_eq!(&caps[4], "commit");
        assert_eq!(&caps[5], "abc123");
    }

    #[test]
    fn repo_re_query_empty() {
        let caps = REPO_RE.captures("test/repository?commit= ");
        assert!(caps.is_none());
    }

    #[test]
    fn repo_re_all() {
        let caps = REPO_RE.captures("test/repository/path/to/dir?branch=main");
        assert!(caps.is_some());
        let caps = caps.unwrap();
        assert_eq!(&caps[1], "test");
        assert_eq!(&caps[2], "repository");
        assert_eq!(caps.get(3).unwrap().as_str(), "/path/to/dir");
        assert_eq!(&caps[4], "branch");
        assert_eq!(&caps[5], "main");
    }

    #[test]
    fn repo_re_none_match() {
        let caps = REPO_RE.captures("test");
        assert!(caps.is_none());
    }

    #[test]
    fn repo_new_basic() -> Result<()> {
        let repo = Repository::new("test/repository")?;

        assert_eq!(repo.owner, "test");
        assert_eq!(repo.name, "repository");
        assert_eq!(repo.input, "test/repository");

        Ok(())
    }

    #[test]
    fn repo_new_with_all() -> Result<()> {
        let repo = Repository::new("test/repository/path/to/dir?branch=main")?;

        assert_eq!(repo.owner, "test");
        assert_eq!(repo.name, "repository");
        assert_eq!(repo.subdir.unwrap(), "/path/to/dir");
        assert_eq!(repo.query.unwrap(), Query::BRANCH("main".to_string()));
        assert_eq!(repo.input, "test/repository/path/to/dir?branch=main");

        Ok(())
    }

    #[test]
    fn repo_new_err() {
        let repo = Repository::new("test");
        assert!(repo.is_err());
    }

    #[test]
    fn repo_cache_ok() -> Result<()> {
        use std::io::Read;

        let mut server = mockito::Server::new();
        let file_path = PathBuf::from_iter(["assets", "scafalra-test.tar.gz"]);
        let mut file = fs::File::open(&file_path)?;
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
