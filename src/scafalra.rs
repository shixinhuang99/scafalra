use std::env;

use anyhow::Result;
use camino::{Utf8Component, Utf8Path, Utf8PathBuf};
use fs_err as fs;

#[cfg(unix)]
use crate::utils::tar_unpack;
#[cfg(windows)]
use crate::utils::zip_unpack;
use crate::{
	cli::{
		AddArgs, CreateArgs, ListArgs, MvArgs, RemoveArgs, TokenArgs,
		UninstallArgs, UpdateArgs,
	},
	config::Config,
	debug,
	github_api::GitHubApi,
	repository::Repository,
	store::{Scaffold, Store},
	utils::download,
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
	const ROOT_DIR_NAME: &'static str = ".scafalra";
	const CACHE_DIR_NAME: &'static str = "cache";
	const UPDATE_DIR_NAME: &'static str = "update";

	pub fn new(
		home_dir: &Utf8Path,
		endpoint: Option<&str>,
		token: Option<&str>,
	) -> Result<Self> {
		let root_dir = home_dir.join(Self::ROOT_DIR_NAME);
		let cache_dir = root_dir.join(Self::CACHE_DIR_NAME);
		let update_dir = root_dir.join(Self::UPDATE_DIR_NAME);

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

		let cwd = Utf8PathBuf::from_path_buf(env::current_dir()?).map_err(
			|err_path| {
				anyhow::anyhow!(
					"Current working directory `{}` it is not valid UTF-8 path",
					err_path.display()
				)
			},
		)?;

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

		dircpy::copy_dir(&scaffold.path, &dst)?;

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

		if cfg!(not(test)) {
			self_replace::self_replace(new_executable)?;
		}

		remove_dir_all::remove_dir_all(&self.update_dir)?;

		Ok(())
	}

	pub fn uninstall(&self, args: UninstallArgs) -> Result<()> {
		debug!("args: {:#?}", args);

		if !args.keep_data {
			remove_dir_all::remove_dir_all(&self.root_dir)?;
		}

		if cfg!(not(test)) {
			self_replace::self_delete()?;
		}

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

	use super::Scafalra;
	use crate::{
		cli::{AddArgs, CreateArgs, UninstallArgs, UpdateArgs},
		github_api::mock_release_response_json,
		store::{mock_store_json, Store},
		utf8_path::Utf8PathBufExt,
		utils::{get_self_target, get_self_version},
	};

	fn mock_scafalra(
		endpoint: &str,
		init_content: bool,
	) -> Result<(Scafalra, TempDir)> {
		let temp_dir = tempdir()?;
		let temp_dir_path = temp_dir.path().into_utf8_path_buf()?;

		if init_content {
			let root_dir = temp_dir_path.join(Scafalra::ROOT_DIR_NAME);
			let cache_dir = root_dir.join(Scafalra::CACHE_DIR_NAME);
			let store_file = root_dir.join(Store::FILE_NAME);
			let foo_dir = cache_dir.join("foo");
			fs::create_dir_all(foo_dir.join("bar"))?;
			fs::write(foo_dir.join("bar").join("baz.txt"), "")?;
			fs::write(store_file, mock_store_json(vec![("bar", foo_dir)]))?;
		}

		let scafalra =
			Scafalra::new(&temp_dir_path, Some(endpoint), Some("token"))?;

		Ok((scafalra, temp_dir))
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
		let (scafalra, _dir) = mock_scafalra("", false)?;

		assert!(scafalra.cache_dir.exists());
		assert!(scafalra.store.path.exists());
		assert!(scafalra.config.path.exists());

		Ok(())
	}

	#[test]
	fn test_scafalra_add() -> Result<()> {
		let (server, tarball_mock, api_mock) = mock_server()?;
		let (mut scafalra, _dir) = mock_scafalra(&server.url(), false)?;

		scafalra.add(AddArgs {
			repository: "foo/bar".to_string(),
			depth: 0,
			name: None,
		})?;

		tarball_mock.assert();
		api_mock.assert();

		let store_content = fs::read_to_string(&scafalra.store.path)?;
		let bar_dir = scafalra.cache_dir.join("foo").join("bar");
		let expected = mock_store_json(vec![("bar", &bar_dir)]);

		assert!(bar_dir.exists());
		assert_eq!(store_content, expected);

		Ok(())
	}

	#[test]
	fn test_scafalra_add_specified_name() -> Result<()> {
		let (server, tarball_mock, api_mock) = mock_server()?;
		let (mut scafalra, _dir) = mock_scafalra(&server.url(), false)?;

		scafalra.add(AddArgs {
			repository: "foo/bar".to_string(),
			depth: 0,
			name: Some("foo".to_string()),
		})?;

		tarball_mock.assert();
		api_mock.assert();

		let store_content = fs::read_to_string(&scafalra.store.path)?;
		let bar_dir = scafalra.cache_dir.join("foo").join("bar");
		let expected = mock_store_json(vec![("foo", &bar_dir)]);

		assert!(bar_dir.exists());
		assert_eq!(store_content, expected);

		Ok(())
	}

	#[test]
	fn test_scafalra_add_depth_1() -> Result<()> {
		let (server, tarball_mock, api_mock) = mock_server()?;
		let (mut scafalra, _dir) = mock_scafalra(&server.url(), false)?;

		scafalra.add(AddArgs {
			repository: "foo/bar".to_string(),
			depth: 1,
			name: Some("foo".to_string()),
		})?;

		tarball_mock.assert();
		api_mock.assert();

		let store_content = fs::read_to_string(&scafalra.store.path)?;
		let bar_dir = scafalra.cache_dir.join("foo").join("bar");
		let expected = mock_store_json(vec![
			("a", bar_dir.join("a")),
			("b", bar_dir.join("b")),
			("c", bar_dir.join("c")),
			("node_modules", bar_dir.join("node_modules")),
		]);

		assert!(bar_dir.exists());
		assert_eq!(store_content, expected);

		Ok(())
	}

	#[test]
	fn test_scafalra_add_subdir() -> Result<()> {
		let (server, tarball_mock, api_mock) = mock_server()?;
		let (mut scafalra, _dir) = mock_scafalra(&server.url(), false)?;

		scafalra.add(AddArgs {
			repository: "foo/bar/a/a1".to_string(),
			depth: 0,
			name: None,
		})?;

		tarball_mock.assert();
		api_mock.assert();

		let store_content = fs::read_to_string(&scafalra.store.path)?;
		let a1_dir = scafalra
			.cache_dir
			.join("foo")
			.join("bar")
			.join("a")
			.join("a1");
		let expected = mock_store_json(vec![("a1", &a1_dir)]);

		assert!(a1_dir.exists());
		assert_eq!(store_content, expected);

		Ok(())
	}

	#[test]
	fn test_scafalra_add_subdir_and_depth_1() -> Result<()> {
		let (server, tarball_mock, api_mock) = mock_server()?;
		let (mut scafalra, _dir) = mock_scafalra(&server.url(), false)?;

		scafalra.add(AddArgs {
			repository: "foo/bar/a".to_string(),
			depth: 1,
			name: None,
		})?;

		tarball_mock.assert();
		api_mock.assert();

		let store_content = fs::read_to_string(&scafalra.store.path)?;
		let a_dir = scafalra.cache_dir.join("foo").join("bar").join("a");
		let a1_dir = a_dir.join("a1");
		let a2_dir = a_dir.join("a2");
		let a3_dir = a1_dir.join("a3");
		let expected = mock_store_json(vec![
			("a1", &a1_dir),
			("a2", &a2_dir),
			("a3", &a3_dir),
		]);

		assert!(a1_dir.exists());
		assert!(a2_dir.exists());
		assert!(a3_dir.exists());
		assert_eq!(store_content, expected);

		Ok(())
	}

	#[test]
	fn test_scafalra_create() -> Result<()> {
		let (scafalra, temp_dir) = mock_scafalra("", true)?;

		let temp_dir_path = temp_dir.path().into_utf8_path_buf()?;

		scafalra.create(CreateArgs {
			name: "bar".to_string(),
			// Due to chroot restrictions, a directory is specified here to
			// simulate the current working directory
			directory: Some(temp_dir_path.join("bar")),
		})?;

		assert!(
			temp_dir_path
				.join("foo")
				.join("bar")
				.join("baz.txt")
				.exists()
		);

		Ok(())
	}

	#[test]
	fn test_scafalra_create_not_found() -> Result<()> {
		let (scafalra, _dir) = mock_scafalra("", false)?;

		let ret = scafalra.create(CreateArgs {
			name: "bar".to_string(),
			directory: None,
		});

		assert!(ret.is_err());

		Ok(())
	}

	#[test]
	fn test_scafalra_update_check() -> Result<()> {
		let (server, query_release_mock, _) = mock_server_for_update()?;
		let (scafalra, _temp_dir) = mock_scafalra(&server.url(), false)?;

		scafalra.update(UpdateArgs { check: true })?;

		query_release_mock.assert();
		assert!(!scafalra.update_dir.exists());

		Ok(())
	}

	#[test]
	fn test_scafalra_update() -> Result<()> {
		let (server, query_release_mock, download_mock) =
			mock_server_for_update()?;
		let (scafalra, _temp_dir) = mock_scafalra(&server.url(), false)?;

		scafalra.update(UpdateArgs { check: false })?;

		query_release_mock.assert();
		download_mock.assert();
		assert!(!scafalra.update_dir.exists());

		Ok(())
	}

	#[test]
	fn test_scafalra_uninstall() -> Result<()> {
		let (scafalra, _temp_dir) = mock_scafalra("", false)?;

		scafalra.uninstall(UninstallArgs { keep_data: false })?;

		assert!(!scafalra.root_dir.exists());

		Ok(())
	}

	#[test]
	fn test_scafalra_uninstall_keep_data() -> Result<()> {
		let (scafalra, _temp_dir) = mock_scafalra("", false)?;

		scafalra.uninstall(UninstallArgs { keep_data: true })?;

		assert!(scafalra.root_dir.exists());

		Ok(())
	}
}
