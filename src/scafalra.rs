use std::env;

use anyhow::Result;
use camino::{Utf8Component, Utf8Path, Utf8PathBuf};
use fs_err as fs;

#[cfg(windows)]
use crate::utils::zip_unpack;
use crate::{
	cli::{
		AddArgs, CreateArgs, ListArgs, MvArgs, RemoveArgs, TokenArgs,
		UpdateArgs,
	},
	config::Config,
	debug,
	error::ScafalraError,
	github_api::GitHubApi,
	repository::Repository,
	store::{Scaffold, Store},
	utils::{download, tar_unpack},
};

pub struct Scafalra {
	pub root_dir: Utf8PathBuf,
	cache_dir: Utf8PathBuf,
	config: Config,
	store: Store,
	github_api: GitHubApi,
	update_dir: Utf8PathBuf,
}

impl Scafalra {
	pub fn new(
		home_dir: &Utf8Path,
		endpoint: Option<&str>,
		token: Option<&str>,
	) -> Result<Self> {
		let root_dir = home_dir.join(".scafalra");
		let cache_dir = root_dir.join("cache");
		let update_dir = root_dir.join("update");

		if !cache_dir.exists() {
			fs::create_dir_all(&cache_dir)?;
		}

		let config = Config::new(&root_dir)?;
		let store = Store::new(&root_dir)?;
		let github_api = GitHubApi::new(endpoint);

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
			update_dir,
		})
	}

	pub fn set_or_display_token(&mut self, args: TokenArgs) -> Result<()> {
		debug!("args: {:#?}", args);

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
		debug!("args: {:#?}", args);

		let may_output = if args.table {
			self.store.print_table()
		} else {
			self.store.print_grid()
		};

		if let Some(output) = may_output {
			println!("{}", output);
		}
	}

	pub fn add(&mut self, args: AddArgs) -> Result<()> {
		debug!("args: {:#?}", args);

		let repo = Repository::parse(&args.repository)?;

		println!("Downloading `{}` ...", args.repository);

		let repo_info = self.github_api.query_repository(&repo)?;

		let mut scaffold_name = args.name.unwrap_or(repo.name.clone());

		let mut scaffold_path =
			repo.cache(&repo_info.tarball_url, &self.cache_dir)?;

		debug!("scaffold_path: {}", scaffold_path);

		if let Some(ref subdir) = repo.subdir {
			subdir
				.components()
				.filter(|c| matches!(c, Utf8Component::Normal(_)))
				.for_each(|c| {
					scaffold_path.push(c);
				});

			debug!("scaffold_path: {}", scaffold_path);

			if let Some(name) = scaffold_path.file_name() {
				scaffold_name = name.to_string();
			}
		}

		if args.depth == 0 {
			self.store.add(Scaffold::new(
				scaffold_name,
				repo_info.url.clone(),
				scaffold_path.clone(),
			))
		}

		if args.depth == 1 {
			for entry in scaffold_path.read_dir_utf8()? {
				let entry = entry?;
				let file_type = entry.file_type()?;
				let file_name = entry.file_name();

				if file_type.is_dir() && !file_name.starts_with('.') {
					self.store.add(Scaffold::new(
						file_name,
						repo_info.url.clone(),
						entry.path(),
					))
				}
			}
		}

		self.store.save()?;

		Ok(())
	}

	pub fn create(&self, args: CreateArgs) -> Result<()> {
		debug!("args: {:#?}", args);

		let scaffold = self.store.get(&args.name);

		let Some(scaffold) = scaffold else {
			anyhow::bail!("No such scaffold `{}`", args.name);
		};

		let cwd = Utf8PathBuf::from_path_buf(env::current_dir()?)
			.map_err(ScafalraError::NonUtf8Path)?;

		debug!("current directory: {}", cwd);

		let dst = if let Some(arg_dir) = args.directory {
			if arg_dir.is_absolute() {
				arg_dir
			} else {
				cwd.join(arg_dir)
			}
		} else {
			cwd.join(args.name)
		};

		debug!("target directory: {}", dst);

		if dst.exists() {
			anyhow::bail!("`{}` is already exists", dst);
		}

		dircpy::copy_dir(&scaffold.local, &dst)?;

		println!("Created in `{}`", dst);

		Ok(())
	}

	pub fn mv(&mut self, args: MvArgs) -> Result<()> {
		debug!("args: {:#?}", args);

		self.store.rename(&args.name, &args.new_name);

		self.store.save()?;

		Ok(())
	}

	pub fn remove(&mut self, args: RemoveArgs) -> Result<()> {
		debug!("args: {:#?}", args);

		for name in args.names {
			self.store.remove(&name)?;
		}

		self.store.save()?;

		Ok(())
	}

	pub fn update(&self, args: UpdateArgs) -> Result<()> {
		debug!("args: {:#?}", args);

		let release = self.github_api.query_release()?;

		debug!("release: {:#?}", release);

		if !release.can_update {
			println!("It's already the latest version");
			return Ok(());
		}

		if args.check {
			println!("Scafalra {} available", release.version);
			return Ok(());
		}

		if !self.update_dir.exists() {
			fs::create_dir_all(&self.update_dir)?;
		}

		let mut archive = self.update_dir.join("t");

		#[cfg(unix)]
		{
			archive.set_extension("tar.gz");
			download(&release.assets_url, &archive)?;
			tar_unpack(&archive, &self.update_dir)?;
		}

		#[cfg(windows)]
		{
			archive.set_extension("zip");
			download(&release.assets_url, &archive)?;
			zip_unpack(&archive, &self.update_dir)?;
		}

		let mut new_executable: Option<Utf8PathBuf> = None;

		for entry in self.update_dir.read_dir_utf8()? {
			let entry = entry?;
			if entry.file_type()?.is_dir() {
				new_executable = Some(
					entry
						.path()
						.join("scafalra")
						.with_extension(env::consts::EXE_EXTENSION),
				);
				break;
			}
		}

		let Some(new_executable) = new_executable else {
			anyhow::bail!("Excutable not found for update");
		};

		if !new_executable.is_file() {
			anyhow::bail!("Invalid executable for update");
		}

		#[cfg(not(test))]
		{
			self_replace::self_replace(new_executable)?;
		}

		remove_dir_all::remove_dir_all(&self.update_dir)?;

		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use std::{fs, path::PathBuf};

	use anyhow::Result;
	use camino::{Utf8Path, Utf8PathBuf};
	use mockito::{Mock, ServerGuard};
	use pretty_assertions::assert_eq;
	use tempfile::{tempdir, TempDir};

	use super::Scafalra;
	use crate::{
		cli::{AddArgs, CreateArgs, UpdateArgs},
		github_api::mock_release_response_json,
		store::Scaffold,
		utils::{get_self_target, get_self_version},
	};

	struct Paths {
		cache_dir: Utf8PathBuf,
		store_file: Utf8PathBuf,
		config_file: Utf8PathBuf,
	}

	fn mock_scafalra(
		endpoint: Option<&str>,
		token: Option<&str>,
		with_scaffold: bool,
	) -> Result<(Scafalra, TempDir, Paths)> {
		let temp_dir = tempdir()?;
		let tempd_dir_path = Utf8Path::from_path(temp_dir.path()).unwrap();
		let root_dir = tempd_dir_path.join(".scafalra");
		let cache_dir = root_dir.join("cache");
		let store_file = root_dir.join("store.toml");
		let config_file = root_dir.join("config.toml");

		if with_scaffold {
			let scaffold_dir = cache_dir.join("scaffold_dir");
			fs::create_dir_all(&scaffold_dir)?;
			fs::create_dir(scaffold_dir.join("a"))?;
			fs::File::create(scaffold_dir.join("a").join("foo.txt"))?;
			fs::File::create(&store_file)?;

			let content = Scaffold::build_toml_str("bar", scaffold_dir);

			fs::write(&store_file, content)?;
		}

		let scafalra = Scafalra::new(tempd_dir_path, endpoint, token)?;

		Ok((
			scafalra,
			temp_dir,
			Paths {
				cache_dir,
				store_file,
				config_file,
			},
		))
	}

	fn mock_server() -> Result<(ServerGuard, Mock, Mock)> {
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

	fn mock_server_for_update() -> Result<(ServerGuard, Mock, Mock)> {
		let mut server = mockito::Server::new();

		let higher_ver = {
			let mut v = semver::Version::parse(get_self_version()).unwrap();
			v.major += 1;
			v.to_string()
		};

		let query_release_mock = server
			.mock("POST", "/")
			.with_status(200)
			.with_header("content-type", "application/json")
			.with_body(mock_release_response_json(&server.url(), &higher_ver))
			.create();

		let download_mock = server
			.mock(
				"GET",
				format!(
					"/scafalra-{}-{}{}",
					higher_ver,
					get_self_target(),
					if cfg!(windows) { ".zip" } else { ".tar.gz" }
				)
				.as_str(),
			)
			.with_status(200)
			.with_header(
				"content-type",
				if cfg!(windows) {
					"application/zip"
				} else {
					"application/gzip"
				},
			)
			.with_body_from_file(PathBuf::from_iter([
				"assets",
				if cfg!(windows) {
					"scafalra-update-windows.zip"
				} else {
					"scafalra-update-unix.tar.gz"
				},
			]))
			.create();

		Ok((server, query_release_mock, download_mock))
	}

	#[test]
	fn test_scafalra_new() -> Result<()> {
		let (scafalra, _dir, paths) = mock_scafalra(None, None, false)?;

		assert_eq!(scafalra.cache_dir, paths.cache_dir);
		assert!(scafalra.cache_dir.exists());
		assert!(paths.store_file.exists());
		assert!(paths.config_file.exists());

		Ok(())
	}

	#[test]
	fn test_scafalra_add() -> Result<()> {
		let (server, tarball_mock, api_mock) = mock_server()?;
		let (mut scafalra, _dir, paths) =
			mock_scafalra(Some(&server.url()), Some("token"), false)?;

		scafalra.add(AddArgs {
			repository: "foo/bar".to_string(),
			depth: 0,
			name: None,
		})?;

		tarball_mock.assert();
		api_mock.assert();

		let scaffold_dir = paths.cache_dir.join("foo").join("bar");

		let store_content = fs::read_to_string(paths.store_file)?;
		let expected = Scaffold::build_toml_str("bar", &scaffold_dir);

		assert_eq!(store_content, expected);
		assert!(scaffold_dir.exists());

		Ok(())
	}

	#[test]
	fn test_scafalra_add_specified_name() -> Result<()> {
		let (server, tarball_mock, api_mock) = mock_server()?;
		let (mut scafalra, _dir, paths) =
			mock_scafalra(Some(&server.url()), Some("token"), false)?;

		scafalra.add(AddArgs {
			repository: "foo/bar".to_string(),
			depth: 0,
			name: Some("foo".to_string()),
		})?;

		tarball_mock.assert();
		api_mock.assert();

		let scaffold_dir = paths.cache_dir.join("foo").join("bar");

		let store_content = fs::read_to_string(paths.store_file)?;
		let expected = Scaffold::build_toml_str("foo", &scaffold_dir);

		assert_eq!(store_content, expected);
		assert!(scaffold_dir.exists());

		Ok(())
	}

	#[test]
	fn test_scafalra_add_depth_1() -> Result<()> {
		let (server, tarball_mock, api_mock) = mock_server()?;
		let (mut scafalra, _dir, paths) =
			mock_scafalra(Some(&server.url()), Some("token"), false)?;

		scafalra.add(AddArgs {
			repository: "foo/bar".to_string(),
			depth: 1,
			name: Some("foo".to_string()),
		})?;

		tarball_mock.assert();
		api_mock.assert();

		let scaffold_dir = paths.cache_dir.join("foo").join("bar");

		let store_content = fs::read_to_string(paths.store_file)?;
		let expected = format!(
			"{}\n{}\n{}\n{}",
			Scaffold::build_toml_str("a", scaffold_dir.join("a")),
			Scaffold::build_toml_str("b", scaffold_dir.join("b")),
			Scaffold::build_toml_str("c", scaffold_dir.join("c")),
			Scaffold::build_toml_str(
				"node_modules",
				scaffold_dir.join("node_modules")
			),
		);

		assert_eq!(store_content, expected);
		assert!(scaffold_dir.exists());

		Ok(())
	}

	#[test]
	fn test_scafalra_add_subdir() -> Result<()> {
		let (server, tarball_mock, api_mock) = mock_server()?;
		let (mut scafalra, _dir, paths) =
			mock_scafalra(Some(&server.url()), Some("token"), false)?;

		scafalra.add(AddArgs {
			repository: "foo/bar/a/a1".to_string(),
			depth: 0,
			name: None,
		})?;

		tarball_mock.assert();
		api_mock.assert();

		let scaffold_dir = paths.cache_dir.join("foo").join("bar");

		let store_content = fs::read_to_string(paths.store_file)?;
		let expected =
			Scaffold::build_toml_str("a1", scaffold_dir.join("a").join("a1"));

		assert_eq!(store_content, expected);
		assert!(scaffold_dir.exists());

		Ok(())
	}

	#[test]
	fn test_scafalra_add_subdir_and_depth_1() -> Result<()> {
		let (server, tarball_mock, api_mock) = mock_server()?;
		let (mut scafalra, _dir, paths) =
			mock_scafalra(Some(&server.url()), Some("token"), false)?;

		scafalra.add(AddArgs {
			repository: "foo/bar/a".to_string(),
			depth: 1,
			name: None,
		})?;

		tarball_mock.assert();
		api_mock.assert();

		let scaffold_dir = paths.cache_dir.join("foo").join("bar");

		let store_content = fs::read_to_string(paths.store_file)?;
		let expected = format!(
			"{}\n{}\n{}",
			Scaffold::build_toml_str("a1", scaffold_dir.join("a").join("a1")),
			Scaffold::build_toml_str("a2", scaffold_dir.join("a").join("a2")),
			Scaffold::build_toml_str("a3", scaffold_dir.join("a").join("a3")),
		);

		assert_eq!(store_content, expected);
		assert!(scaffold_dir.exists());

		Ok(())
	}

	#[test]
	fn test_scafalra_create() -> Result<()> {
		let (scafalra, dir, _) = mock_scafalra(None, None, true)?;

		let dir_path = Utf8Path::from_path(dir.path()).unwrap();

		scafalra.create(CreateArgs {
			name: "bar".to_string(),
			// Due to chroot restrictions, a directory is specified here to
			// simulate the current working directory
			directory: Some(dir_path.join("bar")),
		})?;

		assert!(dir_path.exists());
		assert!(dir_path.join("bar").join("a").join("foo.txt").exists());

		Ok(())
	}

	#[test]
	fn test_scafalra_create_not_found() -> Result<()> {
		let (scafalra, _dir, _) = mock_scafalra(None, None, false)?;

		let res = scafalra.create(CreateArgs {
			name: "bar".to_string(),
			directory: None,
		});

		assert!(res.is_err());

		Ok(())
	}

	#[test]
	fn test_scafalra_update_check() -> Result<()> {
		let (server, query_release_mock, _) = mock_server_for_update()?;
		let (scafalra, _tmpdir, _) =
			mock_scafalra(Some(&server.url()), Some("token"), false)?;

		scafalra.update(UpdateArgs { check: true })?;

		query_release_mock.assert();
		assert!(!scafalra.update_dir.exists());

		Ok(())
	}

	#[test]
	fn test_scafalra_update() -> Result<()> {
		let (server, query_release_mock, download_mock) =
			mock_server_for_update()?;
		let (scafalra, _tmpdir, _) =
			mock_scafalra(Some(&server.url()), Some("token"), false)?;

		scafalra.update(UpdateArgs { check: false })?;

		query_release_mock.assert();
		download_mock.assert();
		assert!(!scafalra.update_dir.exists());

		Ok(())
	}
}
