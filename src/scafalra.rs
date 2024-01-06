use std::{
	env,
	path::{Component, Path, PathBuf},
};

use anyhow::Result;
use fs_err as fs;

#[cfg(all(unix, feature = "self_update"))]
use crate::utils::tar_unpack;
#[cfg(all(windows, feature = "self_update"))]
use crate::utils::zip_unpack;
use crate::{
	cli::{AddArgs, CreateArgs, ListArgs, MvArgs, RemoveArgs, TokenArgs},
	config::Config,
	debug,
	github_api::GitHubApi,
	repository::Repository,
	repository_config::RepositoryConfig,
	store::{Store, Template, TemplateBuilder},
};
#[cfg(feature = "self_update")]
use crate::{
	cli::{UninstallArgs, UpdateArgs},
	utils::download,
};

pub struct Scafalra {
	pub proj_dir: PathBuf,
	cache_dir: PathBuf,
	#[cfg(feature = "self_update")]
	update_dir: PathBuf,
	config: Config,
	store: Store,
	github_api: GitHubApi,
}

impl Scafalra {
	const CACHE_DIR_NAME: &'static str = "cache";
	#[cfg(feature = "self_update")]
	const UPDATE_DIR_NAME: &'static str = "update";

	pub fn new(
		proj_dir: PathBuf,
		endpoint: Option<&str>,
		token: Option<&str>,
	) -> Result<Self> {
		let cache_dir = proj_dir.join(Self::CACHE_DIR_NAME);
		#[cfg(feature = "self_update")]
		let update_dir = proj_dir.join(Self::UPDATE_DIR_NAME);

		if !cache_dir.exists() {
			fs::create_dir_all(&cache_dir)?;
		}

		let config = Config::new(&proj_dir)?;
		let store = Store::new(&proj_dir)?;
		let github_api = GitHubApi::new(endpoint);

		if let Some(token) = token.or_else(|| config.token()) {
			github_api.set_token(token);
		}

		Ok(Self {
			proj_dir,
			cache_dir,
			config,
			store,
			github_api,
			#[cfg(feature = "self_update")]
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

		let mut template_name = args.name.unwrap_or(repo.name.clone());

		let mut template_dir =
			repo.cache(&repo_info.tarball_url, &self.cache_dir)?;

		debug!("template_dir: {:?}", template_dir);

		if let Some(subdir) = repo.subdir {
			subdir
				.components()
				.filter(|c| matches!(c, Component::Normal(_)))
				.for_each(|c| {
					template_dir.push(c);
				});

			debug!("template_path: {:?}", template_dir);
		}

		if args.depth == 0 {
			if let Some(name) = template_dir.file_name() {
				template_name = name.to_string_lossy().to_string();
			}

			self.store.add(Template::new(
				template_name,
				repo_info.url,
				template_dir,
			))
		} else if args.depth == 1 {
			for entry in template_dir.read_dir()? {
				let entry = entry?;
				let file_type = entry.file_type()?;
				let file_name = entry.file_name().to_string_lossy().to_string();

				if file_type.is_dir() && !file_name.starts_with('.') {
					let sub_template_dir = entry.path();

					self.store.add(
						TemplateBuilder::new(
							&file_name,
							&repo_info.url,
							&sub_template_dir,
						)
						.sub_template(true)
						.build(),
					);
				}
			}

			self.copy_on_add(&template_dir);
		}

		self.store.save()?;

		Ok(())
	}

	fn copy_on_add(&self, template_dir: &Path) {
		use globwalk::{GlobWalker, GlobWalkerBuilder};

		if let Ok(repo_cfg) = RepositoryConfig::load(template_dir) {
			let template_gw_list = repo_cfg
				.copy_on_add
				.iter()
				.filter_map(|(name, globs)| {
					if let Some(template) = self.store.get(name) {
						if let Ok(gw) = GlobWalkerBuilder::from_patterns(
							template_dir.join(RepositoryConfig::DIR_NAME),
							globs,
						)
						.case_insensitive(true)
						.build()
						{
							return Some((template, gw));
						}
					};

					None
				})
				.collect::<Vec<(&Template, GlobWalker)>>();

			for (template, gw) in template_gw_list {
				copy_by_glob_walker(gw, &template.path);
			}
		}
	}

	pub fn create(&self, args: CreateArgs) -> Result<()> {
		debug!("args: {:#?}", args);

		let Some(template) = self.store.get(&args.name) else {
			anyhow::bail!("No such template `{}`", args.name);
		};

		let cwd = env::current_dir()?;

		debug!("cwd: {:?}", cwd);

		let dest = if let Some(arg_dir) = args.directory {
			if arg_dir.is_absolute() {
				arg_dir
			} else {
				cwd.join(arg_dir)
			}
		} else {
			cwd.join(args.name)
		};

		debug!("dest: {:?}", dest);

		let dest_display = dest.to_string_lossy();

		if dest.exists() {
			anyhow::bail!("`{}` is already exists", dest_display);
		}

		dircpy::copy_dir(&template.path, &dest)?;

		if let Some(with) = args.with {
			if template.is_sub_template.is_some_and(|v| v) {
				if let Some(parent) = template.path.parent() {
					self.copy_on_create(parent, &dest, with);
				};
			}
		}

		println!("Created in `{}`", dest_display);

		Ok(())
	}

	fn copy_on_create(&self, from: &Path, dest: &Path, with: String) {
		let mut globs = with
			.split(',')
			.filter(|v| !v.is_empty())
			.collect::<Vec<_>>();
		globs.dedup();

		if let Ok(gw) = globwalk::GlobWalkerBuilder::from_patterns(from, &globs)
			.case_insensitive(true)
			.build()
		{
			copy_by_glob_walker(gw, dest);
		}
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

	#[cfg(feature = "self_update")]
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

		let mut new_executable: Option<PathBuf> = None;

		for entry in self.update_dir.read_dir()? {
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

	#[cfg(feature = "self_update")]
	pub fn uninstall(&self, args: UninstallArgs) -> Result<()> {
		debug!("args: {:#?}", args);

		if !args.keep_data {
			remove_dir_all::remove_dir_all(&self.proj_dir)?;
		}

		if cfg!(not(test)) {
			self_replace::self_delete()?;
		}

		Ok(())
	}
}

fn copy_by_glob_walker(gw: globwalk::GlobWalker, dest: &Path) {
	for matching in gw.filter_map(|ret| ret.ok()) {
		let matching_path = matching.path();
		debug!("matching_path: {:?}", matching_path);
		let entry_type = matching.file_type();
		let entry_name = matching.file_name();
		let curr_dest = dest.join(entry_name);
		debug!("dst: {:?}", curr_dest);
		if entry_type.is_dir() {
			let _ = dircpy::CopyBuilder::new(matching_path, curr_dest)
				.overwrite(true)
				.run();
		} else if entry_type.is_file() {
			let _ = fs::copy(matching_path, curr_dest);
		}
	}
}

#[cfg(test)]
mod tests {
	use std::{fs, path::PathBuf};

	use anyhow::Result;
	use mockito::{Mock, ServerGuard};
	use tempfile::{tempdir, TempDir};

	use super::Scafalra;
	use crate::{
		cli::{AddArgs, CreateArgs},
		github_api::mock_repo_response_json,
		path_ext::*,
		store::{mock_store_json, Store},
	};
	#[cfg(feature = "self_update")]
	use crate::{
		cli::{UninstallArgs, UpdateArgs},
		github_api::mock_release_response_json,
		utils::{get_self_target, get_self_version},
	};

	fn mock_scafalra(
		endpoint: &str,
		init_content: bool,
	) -> Result<(Scafalra, TempDir)> {
		let temp_dir = tempdir()?;
		let proj_dir = temp_dir.path().join("scafalra");

		if init_content {
			let store_file = proj_dir.join(Store::FILE_NAME);
			let bar_dir =
				proj_dir.join_iter([Scafalra::CACHE_DIR_NAME, "foo", "bar"]);
			fs::create_dir_all(&bar_dir)?;
			fs::write(bar_dir.join("baz.txt"), "")?;
			fs::write(store_file, mock_store_json([("bar", bar_dir)]))?;
		}

		let scafalra = Scafalra::new(proj_dir, Some(endpoint), Some("token"))?;

		Ok((scafalra, temp_dir))
	}

	fn mock_repo_server() -> Result<(ServerGuard, Mock, Mock)> {
		let mut server = mockito::Server::new();

		let query_repo_mock = server
			.mock("POST", "/")
			.with_status(200)
			.with_header("content-type", "application/json")
			.with_body(mock_repo_response_json(&server.url()))
			.create();

		let downlowd_mock = server
			.mock("GET", "/tarball")
			.with_status(200)
			.with_header("content-type", "application/gzip")
			.with_body_from_file("fixtures/scafalra-test.tar.gz")
			.create();

		Ok((server, query_repo_mock, downlowd_mock))
	}

	#[cfg(feature = "self_update")]
	fn mock_release_server() -> Result<(ServerGuard, Mock, Mock)> {
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
				"fixtures",
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
		let (server, query_repo_mock, download_mock) = mock_repo_server()?;
		let (mut scafalra, _dir) = mock_scafalra(&server.url(), false)?;

		scafalra.add(AddArgs {
			repository: "foo/bar".to_string(),
			depth: 0,
			name: None,
		})?;

		query_repo_mock.assert();
		download_mock.assert();

		let store_content = fs::read_to_string(&scafalra.store.path)?;
		let bar_dir = scafalra.cache_dir.join_slash("foo/bar");
		let expected = mock_store_json([("bar", &bar_dir)]);

		assert!(bar_dir.exists());
		assert_eq!(store_content, expected);

		Ok(())
	}

	#[test]
	fn test_scafalra_add_specified_name() -> Result<()> {
		let (server, query_repo_mock, download_mock) = mock_repo_server()?;
		let (mut scafalra, _dir) = mock_scafalra(&server.url(), false)?;

		scafalra.add(AddArgs {
			repository: "foo/bar".to_string(),
			depth: 0,
			name: Some("foo".to_string()),
		})?;

		query_repo_mock.assert();
		download_mock.assert();

		let store_content = fs::read_to_string(&scafalra.store.path)?;
		let bar_dir = scafalra.cache_dir.join_slash("foo/bar");
		let expected = mock_store_json([("foo", &bar_dir)]);

		assert!(bar_dir.exists());
		assert_eq!(store_content, expected);

		Ok(())
	}

	#[test]
	fn test_scafalra_add_depth_1() -> Result<()> {
		let (server, query_repo_mock, download_mock) = mock_repo_server()?;
		let (mut scafalra, _dir) = mock_scafalra(&server.url(), false)?;

		scafalra.add(AddArgs {
			repository: "foo/bar".to_string(),
			depth: 1,
			name: Some("foo".to_string()),
		})?;

		query_repo_mock.assert();
		download_mock.assert();

		let store_content = fs::read_to_string(&scafalra.store.path)?;
		let bar_dir = scafalra.cache_dir.join_slash("foo/bar");
		let expected = mock_store_json([
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
		let (server, query_repo_mock, download_mock) = mock_repo_server()?;
		let (mut scafalra, _dir) = mock_scafalra(&server.url(), false)?;

		scafalra.add(AddArgs {
			repository: "foo/bar/a/a1".to_string(),
			depth: 0,
			name: None,
		})?;

		query_repo_mock.assert();
		download_mock.assert();

		let store_content = fs::read_to_string(&scafalra.store.path)?;
		let a1_dir = scafalra.cache_dir.join_slash("foo/bar/a/a1");
		let expected = mock_store_json([("a1", &a1_dir)]);

		assert!(a1_dir.exists());
		assert_eq!(store_content, expected);

		Ok(())
	}

	#[test]
	fn test_scafalra_add_subdir_and_depth_1() -> Result<()> {
		let (server, query_repo_mock, download_mock) = mock_repo_server()?;
		let (mut scafalra, _dir) = mock_scafalra(&server.url(), false)?;

		scafalra.add(AddArgs {
			repository: "foo/bar/a".to_string(),
			depth: 1,
			name: None,
		})?;

		query_repo_mock.assert();
		download_mock.assert();

		let store_content = fs::read_to_string(&scafalra.store.path)?;
		let a_dir = scafalra.cache_dir.join_slash("foo/bar/a");
		let a1_dir = a_dir.join("a1");
		let a2_dir = a_dir.join("a2");
		let a3_dir = a_dir.join("a3");
		let expected = mock_store_json([
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

		let temp_dir_path = temp_dir.path();

		scafalra.create(CreateArgs {
			name: "bar".to_string(),
			// Due to chroot restrictions, a directory is specified here to
			// simulate the current working directory
			directory: Some(temp_dir_path.join("bar")),
			with: None,
		})?;

		assert!(temp_dir_path.join_slash("bar/baz.txt").exists());

		Ok(())
	}

	#[test]
	fn test_scafalra_create_not_found() -> Result<()> {
		let (scafalra, _dir) = mock_scafalra("", false)?;

		let ret = scafalra.create(CreateArgs {
			name: "bar".to_string(),
			directory: None,
			with: None,
		});

		assert!(ret.is_err());

		Ok(())
	}

	#[test]
	#[cfg(feature = "self_update")]
	fn test_scafalra_update_check() -> Result<()> {
		let (server, query_release_mock, _) = mock_release_server()?;
		let (scafalra, _temp_dir) = mock_scafalra(&server.url(), false)?;

		scafalra.update(UpdateArgs { check: true })?;

		query_release_mock.assert();
		assert!(!scafalra.update_dir.exists());

		Ok(())
	}

	#[test]
	#[cfg(feature = "self_update")]
	fn test_scafalra_update() -> Result<()> {
		let (server, query_release_mock, download_mock) =
			mock_release_server()?;
		let (scafalra, _temp_dir) = mock_scafalra(&server.url(), false)?;

		scafalra.update(UpdateArgs { check: false })?;

		query_release_mock.assert();
		download_mock.assert();
		assert!(!scafalra.update_dir.exists());

		Ok(())
	}

	#[test]
	#[cfg(feature = "self_update")]
	fn test_scafalra_uninstall() -> Result<()> {
		let (scafalra, _temp_dir) = mock_scafalra("", false)?;

		scafalra.uninstall(UninstallArgs { keep_data: false })?;

		assert!(!scafalra.proj_dir.exists());

		Ok(())
	}

	#[test]
	#[cfg(feature = "self_update")]
	fn test_scafalra_uninstall_keep_data() -> Result<()> {
		let (scafalra, _temp_dir) = mock_scafalra("", false)?;

		scafalra.uninstall(UninstallArgs { keep_data: true })?;

		assert!(scafalra.proj_dir.exists());

		Ok(())
	}

	fn is_all_exists(paths: &[PathBuf]) -> bool {
		paths.iter().any(|p| !p.exists())
	}

	#[test]
	fn test_scafalra_copy_on_add() -> Result<()> {
		let (server, _query_repo_mock, _download_mock) = mock_repo_server()?;
		let (mut scafalra, _dir) = mock_scafalra(&server.url(), false)?;

		scafalra.add(AddArgs {
			repository: "foo/bar".to_string(),
			depth: 1,
			name: Some("foo".to_string()),
		})?;

		let template_dir = scafalra.cache_dir.join_slash("foo/bar");

		let a_dir = template_dir.join("a");
		assert!(is_all_exists(&[
			a_dir.join("common.txt"),
			a_dir.join_slash("copy-dir/copy-dir.txt"),
			a_dir.join_slash("copy-dir/copy-dir-2/copy-dir-2.txt"),
			a_dir.join("copy-all-in-dir.txt"),
			a_dir.join_slash("copy-all-in-dir-2/copy-all-in-dir-2.txt"),
			a_dir.join_slash("shared-a/shared-a.txt")
		]));

		let b_dir = template_dir.join("b");
		assert!(b_dir.join_slash("shared-b/shared-b.txt").exists());

		let c_dir = template_dir.join("c");
		assert!(c_dir.join_slash("shared-c/shared-c.txt").exists());

		Ok(())
	}

	#[test]
	fn test_scafalra_copy_on_create() -> Result<()> {
		let (server, _query_repo_mock, _download_mock) = mock_repo_server()?;
		let (mut scafalra, _dir) = mock_scafalra(&server.url(), false)?;

		scafalra.add(AddArgs {
			repository: "foo/bar".to_string(),
			depth: 1,
			name: Some("foo".to_string()),
		})?;

		let tmp_dir = tempdir()?;
		let dest = tmp_dir.path().join("dest");

		scafalra.create(CreateArgs {
			name: "b".to_string(),
			directory: Some(dest.clone()),
			with: Some("common.txt,copy-dir,copy-all-in-dir/**".to_string()),
		})?;

		assert!(is_all_exists(&[
			dest.join("common.txt"),
			dest.join_slash("copy-dir/copy-dir.txt"),
			dest.join_slash("copy-dir/copy-dir-2/copy-dir-2.txt"),
			dest.join("copy-all-in-dir.txt"),
			dest.join_slash("copy-all-in-dir-2/copy-all-in-dir-2.txt"),
		]));

		Ok(())
	}
}
