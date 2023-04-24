#![allow(dead_code)]

use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

use anyhow::Result;

use crate::{
    cli::{AddArgs, ListArgs, TokenArgs},
    config::Config,
    github_api::GitHubApi,
    print_flush,
    repository::Repository,
    store::{Scaffold, Store},
};

pub struct Scafalra {
    root_dir: PathBuf,
    cache_dir: PathBuf,
    config: Config,
    store: Store,
    github_api: GitHubApi,
}

impl Scafalra {
    pub fn new(
        home_dir: &Path,
        endpoint: Option<&str>,
        token: Option<&str>,
    ) -> Result<Self> {
        let root_dir = home_dir.join(".scafalra");
        let cache_dir = root_dir.join("cache");

        if !cache_dir.exists() {
            fs::create_dir_all(&cache_dir)?;
        }

        let config = Config::new(&root_dir)?;
        let store = Store::new(&root_dir)?;
        let mut github_api = GitHubApi::new(endpoint);

        let token = token.or_else(|| config.token());

        if let Some(token) = token {
            github_api.set_token(token);
        }

        Ok(Self {
            root_dir,
            cache_dir,
            config,
            store,
            github_api,
        })
    }

    pub fn config_or_display_token(&mut self, args: TokenArgs) -> Result<()> {
        match args.token {
            Some(token) => {
                self.config.set_token(&token);
                self.config.save()?;
            }
            None => {
                if let Some(token) = self.config.token() {
                    println!("{}", token);
                }
            }
        }

        Ok(())
    }

    pub fn list(&self, args: ListArgs) {
        let res = if args.table {
            self.store.print_table()
        } else {
            self.store.print_grid()
        };

        println!("{}", res);
    }

    pub fn add(&mut self, args: AddArgs) -> Result<()> {
        let repo = Repository::new(&args.repository)?;

        print_flush!("Downloading...");

        let api_result = self.github_api.request(&repo)?;

        let mut scaffold_name = args.name.unwrap_or(repo.name.clone());

        let mut scaffold_path = repo.cache(
            &api_result.tarball_url,
            &self.cache_dir,
            &api_result.oid,
        )?;

        if let Some(subdir) = repo.subdir {
            let subdir = Path::new(&subdir);
            scaffold_path.push(subdir.strip_prefix("/").unwrap_or(subdir));
            if let Some(name) = scaffold_path.file_name() {
                scaffold_name = name.to_string_lossy().to_string();
            }
        }

        if args.depth == 0 {
            self.store.add(
                scaffold_name.clone(),
                Scaffold::new(
                    &scaffold_name,
                    &repo.input,
                    &api_result.url,
                    &api_result.oid,
                    &scaffold_path.to_string_lossy(),
                ),
            )
        }

        if args.depth == 1 {
            for entry in scaffold_path.read_dir()? {
                let entry = entry?;
                let file_type = entry.file_type()?;
                let file_name = entry.file_name().to_string_lossy().to_string();

                if file_type.is_dir() && !file_name.starts_with(".") {
                    self.store.add(
                        file_name.clone(),
                        Scaffold::new(
                            &file_name,
                            &repo.input,
                            &api_result.url,
                            &api_result.oid,
                            &entry.path().to_string_lossy(),
                        ),
                    )
                }
            }
        }

        println!("\r");

        self.store.save()?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::{Path, PathBuf},
    };

    use anyhow::Result;
    use mockito::{Mock, ServerGuard};
    use pretty_assertions::assert_eq;
    use tempfile::{tempdir, TempDir};

    use super::{AddArgs, Scafalra};

    struct Paths {
        root_dir: PathBuf,
        cache_dir: PathBuf,
        store_file: PathBuf,
        config_file: PathBuf,
    }

    fn build_scafalra(
        endpoint: Option<&str>,
        token: Option<&str>,
    ) -> Result<(Scafalra, TempDir, Paths)> {
        let dir = tempdir()?;
        let sca = Scafalra::new(dir.path(), endpoint, token)?;
        let root_dir = PathBuf::from_iter([dir.path(), Path::new(".scafalra")]);
        let cache_dir = PathBuf::from_iter([
            dir.path(),
            Path::new(".scafalra"),
            Path::new("cache"),
        ]);
        let store_file = root_dir.join("store.toml");
        let config_file = root_dir.join("config.toml");

        Ok((
            sca,
            dir,
            Paths {
                root_dir,
                cache_dir,
                store_file,
                config_file,
            },
        ))
    }

    fn build_server() -> Result<(ServerGuard, Mock, Mock)> {
        use std::io::Read;

        let mut server = mockito::Server::new();

        let file_path = PathBuf::from_iter(["assets", "scafalra-test.tar.gz"]);
        let mut file = fs::File::open(&file_path)?;
        let mut tarball_data = Vec::new();
        file.read_to_end(&mut tarball_data)?;

        let tarball_mock = server
            .mock("GET", "/tarball")
            .with_status(200)
            .with_header("content-type", "application/x-gzip")
            .with_body(tarball_data)
            .create();

        let api_data = format!(
            r#"{{
                "data": {{
                    "repository": {{
                        "url": "https://github.com/shixinhuang99/scafalra",
                        "defaultBranchRef": {{
                            "target": {{
                                "oid": "ea7c165bac336140bcf08f84758ab752769799be",
                                "tarballUrl": "{}/tarball"
                            }}
                        }}
                    }}
                }}
            }}"#,
            server.url()
        );

        let api_mock = server
            .mock("POST", "/")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(api_data)
            .create();

        Ok((server, tarball_mock, api_mock))
    }

    #[test]
    fn scafalra_new() -> Result<()> {
        let (sca, _dir, paths) = build_scafalra(None, None)?;

        assert_eq!(sca.root_dir, paths.root_dir);
        assert_eq!(sca.cache_dir, paths.cache_dir);
        assert!(sca.root_dir.exists());
        assert!(sca.cache_dir.exists());
        assert!(paths.store_file.exists());
        assert!(paths.config_file.exists());

        Ok(())
    }

    #[test]
    fn scafalra_add_basic() -> Result<()> {
        let (server, tarball_mock, api_mock) = build_server()?;
        let (mut sca, _dir, paths) =
            build_scafalra(Some(&server.url()), Some("token"))?;

        sca.add(AddArgs {
            repository: "shixinhuang99/scafalra".to_string(),
            depth: 0,
            name: None,
        })?;

        tarball_mock.assert();
        api_mock.assert();

        let repo_dir = paths.cache_dir.join("shixinhuang99-scafalra-ea7c165");

        let store_content = fs::read_to_string(paths.store_file)?;
        let expected_content = format!(
            r#"[[scaffold]]
name = "scafalra"
input = "shixinhuang99/scafalra"
url = "https://github.com/shixinhuang99/scafalra"
commit = "ea7c165bac336140bcf08f84758ab752769799be"
local = "{}"
"#,
            repo_dir.to_string_lossy()
        );

        assert_eq!(store_content, expected_content);
        assert!(repo_dir.exists());

        Ok(())
    }

    #[test]
    fn scafalra_add_specified_name() -> Result<()> {
        let (server, tarball_mock, api_mock) = build_server()?;
        let (mut sca, _dir, paths) =
            build_scafalra(Some(&server.url()), Some("token"))?;

        sca.add(AddArgs {
            repository: "shixinhuang99/scafalra".to_string(),
            depth: 0,
            name: Some("foo".to_string()),
        })?;

        tarball_mock.assert();
        api_mock.assert();

        let repo_dir = paths.cache_dir.join("shixinhuang99-scafalra-ea7c165");

        let store_content = fs::read_to_string(paths.store_file)?;
        let expected_content = format!(
            r#"[[scaffold]]
name = "foo"
input = "shixinhuang99/scafalra"
url = "https://github.com/shixinhuang99/scafalra"
commit = "ea7c165bac336140bcf08f84758ab752769799be"
local = "{}"
"#,
            repo_dir.to_string_lossy()
        );

        assert_eq!(store_content, expected_content);
        assert!(repo_dir.exists());

        Ok(())
    }

    #[test]
    fn scafalra_add_depth_1() -> Result<()> {
        let (server, tarball_mock, api_mock) = build_server()?;
        let (mut sca, _dir, paths) =
            build_scafalra(Some(&server.url()), Some("token"))?;

        sca.add(AddArgs {
            repository: "shixinhuang99/scafalra".to_string(),
            depth: 1,
            name: Some("foo".to_string()),
        })?;

        tarball_mock.assert();
        api_mock.assert();

        let repo_dir = paths.cache_dir.join("shixinhuang99-scafalra-ea7c165");

        let store_content = fs::read_to_string(paths.store_file)?;
        let expected_content = format!(
            r#"[[scaffold]]
name = "a"
input = "shixinhuang99/scafalra"
url = "https://github.com/shixinhuang99/scafalra"
commit = "ea7c165bac336140bcf08f84758ab752769799be"
local = "{}"

[[scaffold]]
name = "b"
input = "shixinhuang99/scafalra"
url = "https://github.com/shixinhuang99/scafalra"
commit = "ea7c165bac336140bcf08f84758ab752769799be"
local = "{}"

[[scaffold]]
name = "c"
input = "shixinhuang99/scafalra"
url = "https://github.com/shixinhuang99/scafalra"
commit = "ea7c165bac336140bcf08f84758ab752769799be"
local = "{}"

[[scaffold]]
name = "node_modules"
input = "shixinhuang99/scafalra"
url = "https://github.com/shixinhuang99/scafalra"
commit = "ea7c165bac336140bcf08f84758ab752769799be"
local = "{}"
"#,
            repo_dir.join("a").to_string_lossy(),
            repo_dir.join("b").to_string_lossy(),
            repo_dir.join("c").to_string_lossy(),
            repo_dir.join("node_modules").to_string_lossy(),
        );

        assert_eq!(store_content, expected_content);
        assert!(repo_dir.exists());

        Ok(())
    }

    #[test]
    fn scafalra_add_subdir() -> Result<()> {
        let (server, tarball_mock, api_mock) = build_server()?;
        let (mut sca, _dir, paths) =
            build_scafalra(Some(&server.url()), Some("token"))?;

        sca.add(AddArgs {
            repository: "shixinhuang99/scafalra/a/a1".to_string(),
            depth: 0,
            name: None,
        })?;

        tarball_mock.assert();
        api_mock.assert();

        let repo_dir = paths.cache_dir.join("shixinhuang99-scafalra-ea7c165");

        let store_content = fs::read_to_string(paths.store_file)?;
        let expected_content = format!(
            r#"[[scaffold]]
name = "a1"
input = "shixinhuang99/scafalra/a/a1"
url = "https://github.com/shixinhuang99/scafalra"
commit = "ea7c165bac336140bcf08f84758ab752769799be"
local = "{}"
"#,
            repo_dir.join("a").join("a1").to_string_lossy()
        );

        assert_eq!(store_content, expected_content);
        assert!(repo_dir.exists());

        Ok(())
    }

    #[test]
    fn scafalra_add_subdir_and_depth_1() -> Result<()> {
        let (server, tarball_mock, api_mock) = build_server()?;
        let (mut sca, _dir, paths) =
            build_scafalra(Some(&server.url()), Some("token"))?;

        sca.add(AddArgs {
            repository: "shixinhuang99/scafalra/a".to_string(),
            depth: 1,
            name: None,
        })?;

        tarball_mock.assert();
        api_mock.assert();

        let repo_dir = paths.cache_dir.join("shixinhuang99-scafalra-ea7c165");

        let store_content = fs::read_to_string(paths.store_file)?;
        let expected_content = format!(
            r#"[[scaffold]]
name = "a1"
input = "shixinhuang99/scafalra/a"
url = "https://github.com/shixinhuang99/scafalra"
commit = "ea7c165bac336140bcf08f84758ab752769799be"
local = "{}"

[[scaffold]]
name = "a2"
input = "shixinhuang99/scafalra/a"
url = "https://github.com/shixinhuang99/scafalra"
commit = "ea7c165bac336140bcf08f84758ab752769799be"
local = "{}"

[[scaffold]]
name = "a3"
input = "shixinhuang99/scafalra/a"
url = "https://github.com/shixinhuang99/scafalra"
commit = "ea7c165bac336140bcf08f84758ab752769799be"
local = "{}"
"#,
            repo_dir.join("a").join("a1").to_string_lossy(),
            repo_dir.join("a").join("a2").to_string_lossy(),
            repo_dir.join("a").join("a3").to_string_lossy(),
        );

        assert_eq!(store_content, expected_content);
        assert!(repo_dir.exists());

        Ok(())
    }
}
