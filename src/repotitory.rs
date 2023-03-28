#![allow(dead_code)]

use std::fs::{self, File};
use std::io;
use std::path::Path;

use once_cell::sync::Lazy;
use regex::Regex;
use zip::ZipArchive;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

static REPO_RE: Lazy<Regex> = Lazy::new(|| {
    let re = r"^([^/\s]+)/([^/\s?]+)(?:((?:/[^/\s?]+)+))?(?:\?(branch|tag|commit)=([^\s]+))?$";

    Regex::new(&re).unwrap()
});

pub struct Repository {
    pub owner: String,
    pub name: String,
    pub subdir: Option<String>,
    pub query: Option<Query>,
}

pub enum Query {
    BRANCH(String),
    TAG(String),
    COMMIT(String),
}

impl Repository {
    fn new(input: &String) -> Result<Self> {
        let caps = REPO_RE
            .captures(input)
            .ok_or(format!("Could not parse the input: '{}'.", input))?;

        let owner = (&caps[1]).to_string();
        let name = (&caps[2]).to_string();
        let subdir = caps.get(3).map_or(None, |v| Some(v.as_str().to_string()));
        let query_type = caps.get(4).map_or(None, |v| Some(v.as_str()));
        let query_val = (&caps[5]).to_string();
        let query = match query_type {
            Some("branch") => Some(Query::BRANCH(query_val)),
            Some("tag") => Some(Query::TAG(query_val)),
            Some("commit") => Some(Query::COMMIT(query_val)),
            _ => None,
        };

        Ok(Self {
            owner,
            name,
            subdir,
            query,
        })
    }
}

fn download(url: &str, zip_file_path: &Path) -> Result<()> {
    let response = ureq::get(&url).call()?;
    let mut file = File::create(zip_file_path)?;
    io::copy(&mut response.into_reader(), &mut file)?;

    Ok(())
}

fn unzip(zip_file_path: &Path, target_dir_path: &Path) -> Result<()> {
    let file = File::open(zip_file_path)?;
    let mut archive = ZipArchive::new(file)?;
    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let outpath = Path::new(target_dir_path).join(file.name());
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
    use super::{Query, Repository, REPO_RE};

    #[test]
    fn repo_re_basic() {
        let caps = REPO_RE.captures("test/repository");
        assert!(caps.is_some());
        let caps = caps.unwrap();
        assert_eq!("test", &caps[1]);
        assert_eq!("repository", &caps[2]);
    }

    #[test]
    fn repo_re_subdir() {
        let caps = REPO_RE.captures("test/repository/path/to/file");
        assert!(caps.is_some());
        let caps = caps.unwrap();
        assert_eq!("test", &caps[1]);
        assert_eq!("repository", &caps[2]);
        assert_eq!("/path/to/file", &caps[3]);
    }

    #[test]
    fn repo_re_query_branch() {
        let caps = REPO_RE.captures("test/repository?branch=main");
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
        let caps = REPO_RE.captures("test/repository?tag=v1.0.0");
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
        let caps = REPO_RE.captures("test/repository?commit=abc123");
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
        let caps = REPO_RE.captures("test/repository?commit= ");
        assert!(caps.is_none());
    }

    #[test]
    fn repo_re_all() {
        let caps = REPO_RE.captures("test/repository/path/to/file?branch=main");
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
        let caps = REPO_RE.captures("test");
        assert!(caps.is_none());
    }

    #[test]
    fn repo_new_ok() {
        let repo = Repository::new(
            &"test/repository/path/to/file?branch=main".to_string(),
        );
        assert!(repo.is_ok());
        let repo = repo.unwrap();
        assert_eq!("test", &repo.owner);
        assert_eq!("repository", &repo.name);
        assert_eq!("/path/to/file", &repo.subdir.unwrap());
        if let Query::BRANCH(val) = &repo.query.unwrap() {
            assert_eq!("main", val.as_str());
        } else {
            panic!("");
        };
    }

    #[test]
    fn repo_new_err() {
        let repo = Repository::new(&"test".to_string());
        assert!(repo.is_err());
    }

    #[ignore]
    #[test]
    fn download() {
        use std::fs;
        use std::path::Path;

        let url = "https://codeload.github.com/shixinhuang99/scafalra/legacy.zip/ ea7c165bac336140bcf08f84758ab752769799be";

        let zip_file_name = "shixinhuang99+scafalra+ea7c165bac336140bcf08f84758ab752769799be.zip";

        let path = Path::new(zip_file_name);

        super::download(url, &path).unwrap();

        assert!(path.exists());
        assert_eq!(zip_file_name, path.file_name().unwrap());

        fs::remove_file(&path).unwrap();
    }
}
