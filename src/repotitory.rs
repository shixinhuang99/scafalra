#![allow(dead_code)]

use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Result};
use regex::Regex;
use remove_dir_all::remove_dir_all;
use zip::ZipArchive;

use crate::utils::build_proxy_agent;

fn build_repo_re() -> Regex {
    let re = r"^([^/\s]+)/([^/\s?]+)(?:((?:/[^/\s?]+)+))?(?:\?(branch|tag|commit)=([^\s]+))?$";

    Regex::new(&re).unwrap()
}

pub struct Repository {
    pub owner: String,
    pub name: String,
    pub subdir: Option<PathBuf>,
    pub query: Option<Query>,
}

#[derive(PartialEq, Debug)]
pub enum Query {
    BRANCH(String),
    TAG(String),
    COMMIT(String),
}

impl Repository {
    fn new(input: &str) -> Result<Self> {
        let repo_re = build_repo_re();

        let caps = repo_re
            .captures(input)
            .ok_or(anyhow!("Could not parse the input: '{}'", input))?;

        let owner = (&caps[1]).to_string();
        let name = (&caps[2]).to_string();

        let subdir = caps
            .get(3)
            .map_or(None, |v| Some(PathBuf::from(v.as_str())));

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
        })
    }

    pub fn cache(
        &self,
        zipball_url: &str,
        parent_dir: &Path,
        oid: &str,
    ) -> Result<()> {
        let repo_dir_path = PathBuf::from(format!(
            "{}-{}-{}",
            self.owner,
            self.name,
            &oid[0..7]
        ));

        let zip_file_path = repo_dir_path.with_extension("zip");

        download(zipball_url, &zip_file_path)?;

        if repo_dir_path.exists() {
            remove_dir_all(repo_dir_path)?;
        }

        unzip(&zip_file_path, parent_dir)?;

        fs::remove_file(zip_file_path)?;

        Ok(())
    }

    pub fn is_repo(input: &str) -> bool {
        build_repo_re().is_match(input)
    }
}

fn download(url: &str, zip_file_path: &Path) -> Result<()> {
    let agent = build_proxy_agent();
    let response = agent.get(url).call()?;
    let mut file = File::create(zip_file_path)?;

    io::copy(&mut response.into_reader(), &mut file)?;

    Ok(())
}

fn unzip(zip_file_path: &Path, parent_dir: &Path) -> Result<()> {
    let file = File::open(zip_file_path)?;
    let mut archive = ZipArchive::new(file)?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let outpath = Path::new(&parent_dir).join(file.name());

        if file.is_dir() {
            fs::create_dir_all(&outpath)?;
        } else {
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    fs::create_dir_all(&p)?;
                }
            }
            let mut outfile = File::create(&outpath)?;
            io::copy(&mut file, &mut outfile)?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{build_repo_re, Query, Repository};

    #[test]
    fn repo_re_basic() {
        let caps = build_repo_re().captures("test/repository");
        assert!(caps.is_some());
        let caps = caps.unwrap();
        assert_eq!("test", &caps[1]);
        assert_eq!("repository", &caps[2]);
    }

    #[test]
    fn repo_re_subdir() {
        let caps = build_repo_re().captures("test/repository/path/to/file");
        assert!(caps.is_some());
        let caps = caps.unwrap();
        assert_eq!("test", &caps[1]);
        assert_eq!("repository", &caps[2]);
        assert_eq!("/path/to/file", &caps[3]);
    }

    #[test]
    fn repo_re_query_branch() {
        let caps = build_repo_re().captures("test/repository?branch=main");
        assert!(caps.is_some());
        let caps = caps.unwrap();
        assert_eq!("test", &caps[1]);
        assert_eq!("repository", &caps[2]);
        assert_eq!(None, caps.get(3));
        assert_eq!("branch", &caps[4]);
        assert_eq!("main", &caps[5]);
    }

    #[test]
    fn repo_re_query_tag() {
        let caps = build_repo_re().captures("test/repository?tag=v1.0.0");
        assert!(caps.is_some());
        let caps = caps.unwrap();
        assert_eq!("test", &caps[1]);
        assert_eq!("repository", &caps[2]);
        assert_eq!(None, caps.get(3));
        assert_eq!("tag", &caps[4]);
        assert_eq!("v1.0.0", &caps[5]);
    }

    #[test]
    fn repo_re_query_commit() {
        let caps = build_repo_re().captures("test/repository?commit=abc123");
        assert!(caps.is_some());
        let caps = caps.unwrap();
        assert_eq!("test", &caps[1]);
        assert_eq!("repository", &caps[2]);
        assert_eq!(None, caps.get(3));
        assert_eq!("commit", &caps[4]);
        assert_eq!("abc123", &caps[5]);
    }

    #[test]
    fn repo_re_query_empty() {
        let caps = build_repo_re().captures("test/repository?commit= ");
        assert!(caps.is_none());
    }

    #[test]
    fn repo_re_all() {
        let caps = build_repo_re()
            .captures("test/repository/path/to/file?branch=main");
        assert!(caps.is_some());
        let caps = caps.unwrap();
        assert_eq!("test", &caps[1]);
        assert_eq!("repository", &caps[2]);
        assert_eq!("/path/to/file", &caps[3]);
        assert_eq!("branch", &caps[4]);
        assert_eq!("main", &caps[5]);
    }

    #[test]
    fn repo_re_none_match() {
        let caps = build_repo_re().captures("test");
        assert!(caps.is_none());
    }

    #[test]
    fn repo_new_basic() {
        let repo = Repository::new("test/repository");
        assert!(repo.is_ok());
        let repo = repo.unwrap();
        assert_eq!("test", &repo.owner);
        assert_eq!("repository", &repo.name);
    }

    #[test]
    fn repo_new_with_all() {
        let repo = Repository::new("test/repository/path/to/file?branch=main");
        assert!(repo.is_ok());
        let repo = repo.unwrap();
        assert_eq!("test", &repo.owner);
        assert_eq!("repository", &repo.name);
        assert_eq!("/path/to/file", repo.subdir.unwrap().to_str().unwrap());
        assert_eq!(Query::BRANCH("main".to_string()), repo.query.unwrap());
    }

    #[test]
    fn repo_new_err() {
        let repo = Repository::new("test");
        assert!(repo.is_err());
    }

    #[test]
    fn is_repo_ok() {
        assert_eq!(true, Repository::is_repo("foo/bar"));
        assert_eq!(false, Repository::is_repo("foo"));
    }

    #[ignore]
    #[test]
    fn cache_repo() {
        use std::path::Path;

        use super::Repository;

        let url = "https://codeload.github.com/shixinhuang99/scafalra/legacy.zip/ea7c165bac336140bcf08f84758ab752769799be";
        let dir = Path::new("something");
        let oid = "ea7c165bac336140bcf08f84758ab752769799be";

        let repo = Repository::new("shixinhuang99/scafalra");
        assert!(repo.is_ok());
        let repo = repo.unwrap();
        assert!(repo.cache(url, dir, oid).is_ok());
    }
}
