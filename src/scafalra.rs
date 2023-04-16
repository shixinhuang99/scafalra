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
    repotitory::Repository,
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
    pub fn new(home_dir: &Path) -> Result<Self> {
        let root_dir = home_dir.join(".scafalra");
        let cache_dir = root_dir.join("cache");

        if !cache_dir.exists() {
            fs::create_dir_all(&cache_dir)?;
        }

        let config = Config::new(&root_dir)?;
        let store = Store::new(&root_dir)?;
        let mut github_api = GitHubApi::new(None);

        if let Some(token) = config.token() {
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
                Scaffold {
                    name: scaffold_name,
                    input: repo.input.clone(),
                    url: api_result.url.clone(),
                    commit: api_result.oid.clone(),
                    local: scaffold_path.to_string_lossy().to_string(),
                },
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
                        Scaffold {
                            name: file_name,
                            input: repo.input.clone(),
                            url: api_result.url.clone(),
                            commit: api_result.oid.clone(),
                            local: entry.path().to_string_lossy().to_string(),
                        },
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
    use std::path::{Path, PathBuf};

    use anyhow::Result;
    use pretty_assertions::assert_eq;
    use tempfile::{tempdir_in, TempDir};

    use super::Scafalra;

    fn build_scafalra() -> Result<(Scafalra, TempDir)> {
        let dir = tempdir_in("something")?;
        let sca = Scafalra::new(dir.path())?;

        Ok((sca, dir))
    }

    #[test]
    fn scafalra_new() -> Result<()> {
        let (sca, dir) = build_scafalra()?;

        assert_eq!(
            sca.root_dir,
            PathBuf::from_iter([dir.path(), Path::new(".scafalra")])
        );
        assert_eq!(
            sca.cache_dir,
            PathBuf::from_iter([
                dir.path(),
                Path::new(".scafalra"),
                Path::new("cache")
            ])
        );

        Ok(())
    }
}
