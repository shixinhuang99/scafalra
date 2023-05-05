use std::{
    fs,
    path::{Component, Path, PathBuf},
};

use anyhow::Result;

use crate::{
    cli::{AddArgs, CreateArgs, ListArgs, MvArgs, RemoveArgs, TokenArgs},
    config::Config,
    github_api::{GitHubApi, GitHubApiResult},
    repository::Repository,
    store::{Scaffold, Store},
};

pub struct Scafalra {
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
        if self.store.scaffolds_len() == 0 {
            return;
        }

        let res = if args.table {
            self.store.print_table()
        } else {
            self.store.print_grid()
        };

        println!("{}", res);
    }

    pub fn add(&mut self, args: AddArgs) -> Result<()> {
        let repo = Repository::new(&args.repository)?;

        println!("Downloading `{}`", args.repository);

        let api_result = self.github_api.request(&repo)?;

        let GitHubApiResult {
            url,
            oid,
            tarball_url,
        } = api_result;

        let mut scaffold_name = args.name.unwrap_or(repo.name.clone());

        let mut scaffold_path =
            repo.cache(&tarball_url, &self.cache_dir, &oid)?;

        if let Some(subdir) = repo.subdir {
            Path::new(&subdir)
                .components()
                .filter(|v| matches!(v, Component::Normal(_)))
                .for_each(|v| {
                    scaffold_path.push(v);
                });

            if let Some(name) = scaffold_path.file_name() {
                scaffold_name = name.to_string_lossy().to_string();
            }
        }

        if args.depth == 0 {
            self.store.add(
                scaffold_name.clone(),
                Scaffold::new(
                    scaffold_name,
                    url.clone(),
                    scaffold_path.clone(),
                ),
            )
        }

        if args.depth == 1 {
            for entry in scaffold_path.read_dir()? {
                let entry = entry?;
                let file_type = entry.file_type()?;
                let file_name = entry.file_name().to_string_lossy().to_string();

                if file_type.is_dir() && !file_name.starts_with('.') {
                    self.store.add(
                        file_name.clone(),
                        Scaffold::new(file_name, url.clone(), entry.path()),
                    )
                }
            }
        }

        self.store.save()?;

        Ok(())
    }

    pub fn create(&self, args: CreateArgs) -> Result<()> {
        use std::env::current_dir;

        println!("Creating `{}`", args.name);

        let scaffold = self.store.get(&args.name);

        let Some(scaffold) = scaffold else {
            anyhow::bail!("No such scaffold `{}`", args.name);
        };

        let target_dir = if let Some(dir) = args.directory {
            let dir_path = PathBuf::from(dir);
            if dir_path.is_absolute() {
                dir_path
            } else {
                current_dir()?.join(dir_path)
            }
        } else {
            current_dir()?.join(args.name)
        };

        fs_extra::dir::copy(
            scaffold.local,
            &target_dir,
            &fs_extra::dir::CopyOptions::new().content_only(true),
        )?;

        println!("Created in `{}`", target_dir.display());

        Ok(())
    }

    pub fn mv(&mut self, args: MvArgs) -> Result<()> {
        self.store.rename(&args.name, &args.new_name);

        self.store.save()?;

        Ok(())
    }

    pub fn remove(&mut self, args: RemoveArgs) -> Result<()> {
        use crate::utils::DedupExt;

        let names = args.names.dedup_without_sort();

        for name in names {
            self.store.remove(name)?;
        }

        self.store.save()?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use anyhow::Result;
    use mockito::{Mock, ServerGuard};
    use pretty_assertions::assert_eq;
    use tempfile::{tempdir, TempDir};

    use super::{AddArgs, CreateArgs, Scafalra};
    use crate::utils::scaffold_toml;

    struct Paths {
        cache_dir: PathBuf,
        store_file: PathBuf,
        config_file: PathBuf,
    }

    fn build_scafalra(
        endpoint: Option<&str>,
        token: Option<&str>,
        with_scaffold: bool,
    ) -> Result<(Scafalra, TempDir, Paths)> {
        let dir = tempdir()?;
        let dir_path = dir.path();
        let root_dir = dir_path.join(".scafalra");
        let cache_dir = root_dir.join("cache");
        let store_file = root_dir.join("store.toml");
        let config_file = root_dir.join("config.toml");

        if with_scaffold {
            let scaffold_dir = cache_dir.join("scaffold_dir");
            fs::create_dir_all(&scaffold_dir)?;
            fs::create_dir(scaffold_dir.join("a"))?;
            fs::File::create(scaffold_dir.join("a").join("foo.txt"))?;
            fs::File::create(&store_file)?;

            let content = scaffold_toml("bar", scaffold_dir);

            fs::write(&store_file, content)?;
        }

        let sca = Scafalra::new(dir_path, endpoint, token)?;

        Ok((
            sca,
            dir,
            Paths {
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
        let mut file = fs::File::open(file_path)?;
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
                        "url": "url",
                        "defaultBranchRef": {{
                            "target": {{
                                "oid": "aaaaaaa",
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
        let (sca, _dir, paths) = build_scafalra(None, None, false)?;

        assert_eq!(sca.cache_dir, paths.cache_dir);
        assert!(sca.cache_dir.exists());
        assert!(paths.store_file.exists());
        assert!(paths.config_file.exists());

        Ok(())
    }

    #[test]
    fn scafalra_add_basic() -> Result<()> {
        let (server, tarball_mock, api_mock) = build_server()?;
        let (mut sca, _dir, paths) =
            build_scafalra(Some(&server.url()), Some("token"), false)?;

        sca.add(AddArgs {
            repository: "foo/bar".to_string(),
            depth: 0,
            name: None,
        })?;

        tarball_mock.assert();
        api_mock.assert();

        let scaffold_dir = paths.cache_dir.join("foo-bar-aaaaaaa");

        let store_content = fs::read_to_string(paths.store_file)?;
        let expected = scaffold_toml("bar", &scaffold_dir);

        assert_eq!(store_content, expected);
        assert!(scaffold_dir.exists());

        Ok(())
    }

    #[test]
    fn scafalra_add_specified_name() -> Result<()> {
        let (server, tarball_mock, api_mock) = build_server()?;
        let (mut sca, _dir, paths) =
            build_scafalra(Some(&server.url()), Some("token"), false)?;

        sca.add(AddArgs {
            repository: "foo/bar".to_string(),
            depth: 0,
            name: Some("foo".to_string()),
        })?;

        tarball_mock.assert();
        api_mock.assert();

        let scaffold_dir = paths.cache_dir.join("foo-bar-aaaaaaa");

        let store_content = fs::read_to_string(paths.store_file)?;
        let expected = scaffold_toml("foo", &scaffold_dir);

        assert_eq!(store_content, expected);
        assert!(scaffold_dir.exists());

        Ok(())
    }

    #[test]
    fn scafalra_add_depth_1() -> Result<()> {
        let (server, tarball_mock, api_mock) = build_server()?;
        let (mut sca, _dir, paths) =
            build_scafalra(Some(&server.url()), Some("token"), false)?;

        sca.add(AddArgs {
            repository: "foo/bar".to_string(),
            depth: 1,
            name: Some("foo".to_string()),
        })?;

        tarball_mock.assert();
        api_mock.assert();

        let scaffold_dir = paths.cache_dir.join("foo-bar-aaaaaaa");

        let store_content = fs::read_to_string(paths.store_file)?;
        let expected = format!(
            "{}\n{}\n{}\n{}",
            scaffold_toml("a", scaffold_dir.join("a")),
            scaffold_toml("b", scaffold_dir.join("b")),
            scaffold_toml("c", scaffold_dir.join("c")),
            scaffold_toml("node_modules", scaffold_dir.join("node_modules")),
        );

        assert_eq!(store_content, expected);
        assert!(scaffold_dir.exists());

        Ok(())
    }

    #[test]
    fn scafalra_add_subdir() -> Result<()> {
        let (server, tarball_mock, api_mock) = build_server()?;
        let (mut sca, _dir, paths) =
            build_scafalra(Some(&server.url()), Some("token"), false)?;

        sca.add(AddArgs {
            repository: "foo/bar/a/a1".to_string(),
            depth: 0,
            name: None,
        })?;

        tarball_mock.assert();
        api_mock.assert();

        let scaffold_dir = paths.cache_dir.join("foo-bar-aaaaaaa");

        let store_content = fs::read_to_string(paths.store_file)?;
        let expected = scaffold_toml("a1", scaffold_dir.join("a").join("a1"));

        assert_eq!(store_content, expected);
        assert!(scaffold_dir.exists());

        Ok(())
    }

    #[test]
    fn scafalra_add_subdir_and_depth_1() -> Result<()> {
        let (server, tarball_mock, api_mock) = build_server()?;
        let (mut sca, _dir, paths) =
            build_scafalra(Some(&server.url()), Some("token"), false)?;

        sca.add(AddArgs {
            repository: "foo/bar/a".to_string(),
            depth: 1,
            name: None,
        })?;

        tarball_mock.assert();
        api_mock.assert();

        let scaffold_dir = paths.cache_dir.join("foo-bar-aaaaaaa");

        let store_content = fs::read_to_string(paths.store_file)?;
        let expected = format!(
            "{}\n{}\n{}",
            scaffold_toml("a1", scaffold_dir.join("a").join("a1")),
            scaffold_toml("a2", scaffold_dir.join("a").join("a2")),
            scaffold_toml("a3", scaffold_dir.join("a").join("a3")),
        );

        assert_eq!(store_content, expected);
        assert!(scaffold_dir.exists());

        Ok(())
    }

    #[test]
    fn scafalra_create_basic() -> Result<()> {
        let (sca, dir, _) = build_scafalra(None, None, true)?;

        let dir_path = dir.path();

        sca.create(CreateArgs {
            name: "bar".to_string(),
            // Due to chroot restrictions, a directory is specified here to
            // simulate the current working directory
            directory: Some(dir_path.join("bar").display().to_string()),
        })?;

        assert!(dir_path.exists());
        assert!(dir_path.join("bar").join("a").join("foo.txt").exists());

        Ok(())
    }

    #[test]
    fn scafalra_create_not_found() -> Result<()> {
        let (sca, _dir, _) = build_scafalra(None, None, false)?;

        let res = sca.create(CreateArgs {
            name: "bar".to_string(),
            directory: None,
        });

        assert!(res.is_err());

        Ok(())
    }
}
