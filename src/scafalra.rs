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
	pub path: PathBuf,
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
		scfalra_dir: PathBuf,
		endpoint: Option<&str>,
		token: Option<&str>,
	) -> Result<Self> {
		let cache_dir = scfalra_dir.join(Self::CACHE_DIR_NAME);
		#[cfg(feature = "self_update")]
		let update_dir = scfalra_dir.join(Self::UPDATE_DIR_NAME);

		if !cache_dir.exists() {
			fs::create_dir_all(&cache_dir)?;
		}

		let config = Config::new(&scfalra_dir)?;
		let store = Store::new(&scfalra_dir)?;
		let github_api = GitHubApi::new(endpoint);

		if let Some(token) = token.or_else(|| config.token()) {
			github_api.set_token(token);
		}

		Ok(Self {
			path: scfalra_dir,
			cache_dir,
			config,
			store,
			github_api,
			#[cfg(feature = "self_update")]
			update_dir,
		})
	}

	pub fn token(&mut self, args: TokenArgs) -> Result<()> {
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

		let remote_repo = self.github_api.query_remote_repo(&repo)?;

		let mut template_name = args.name.unwrap_or(repo.name.clone());

		let mut template_dir =
			repo.cache(&remote_repo.tarball_url, &self.cache_dir)?;

		debug!("template_dir: {:?}", template_dir);

		if let Some(subdir) = repo.subdir {
			subdir
				.components()
				.filter(|c| matches!(c, Component::Normal(_)))
				.for_each(|c| {
					template_dir.push(c);
				});

			debug!("template_path: {:?}", template_dir);

			if let Some(name) = template_dir.file_name() {
				template_name = name.to_string_lossy().to_string();
			}
		}

		match args.depth {
			0 => {
				self.store.add(Template::new(
					template_name,
					remote_repo.url,
					template_dir,
				));
			}
			1 => {
				for entry in template_dir.read_dir()? {
					let entry = entry?;
					let file_type = entry.file_type()?;
					let file_name =
						entry.file_name().to_string_lossy().to_string();

					if file_type.is_dir() && !file_name.starts_with('.') {
						let sub_template_dir = entry.path();

						self.store.add(
							TemplateBuilder::new(
								&file_name,
								&remote_repo.url,
								&sub_template_dir,
							)
							.sub_template(true)
							.build(),
						);
					}
				}

				self.copy_on_add(&template_dir);
			}
			_ => anyhow::bail!("The argument `depth` allows only 0 or 1"),
		}

		self.store.save()?;

		Ok(())
	}

	fn copy_on_add(&self, template_dir: &Path) {
		use globwalk::{GlobWalker, GlobWalkerBuilder};

		let template_gw_list = RepositoryConfig::load(template_dir)
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
			glob_walk_and_copy(gw, &template.path);
		}
	}

	pub fn create(&self, args: CreateArgs) -> Result<()> {
		debug!("args: {:#?}", args);

		let Some(template) = self.store.get(&args.name) else {
			let mut msg = format!("No such template `{}`", args.name);
			if let Some(name) = self.store.get_similar_name(&args.name) {
				msg = format!("{}\nA similar template is `{}`", msg, name);
			}
			anyhow::bail!(msg);
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
			glob_walk_and_copy(gw, dest);
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
						.join("sca")
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
			remove_dir_all::remove_dir_all(&self.path)?;
		}

		if cfg!(not(test)) {
			self_replace::self_delete()?;
		}

		Ok(())
	}
}

fn glob_walk_and_copy(gw: globwalk::GlobWalker, dest: &Path) {
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
mod test_utils {
	use std::{fs, path::PathBuf};

	use mockito::{Mock, ServerGuard};
	use tempfile::{tempdir, TempDir};

	use super::Scafalra;
	#[cfg(feature = "self_update")]
	use crate::{
		github_api::mock_release_response_json,
		utils::{get_self_target, get_self_version},
	};
	use crate::{
		github_api::mock_repo_response_json,
		store::{test_utils::StoreJsonMock, Store},
	};

	pub struct ScafalraMock {
		pub scafalra: Scafalra,
		pub tmpdir: TempDir,
		endpoint_cache: Option<String>,
	}

	impl ScafalraMock {
		pub fn new() -> Self {
			let tmpdir = tempdir().unwrap();
			let scafalra = Scafalra::new(
				tmpdir.path().join("scafalra"),
				None,
				Some("token"),
			)
			.unwrap();

			Self {
				scafalra,
				tmpdir,
				endpoint_cache: None,
			}
		}

		pub fn endpoint(self, endpoint: &str) -> Self {
			let scafalra = Scafalra::new(
				self.scafalra.path,
				Some(endpoint),
				Some("token"),
			)
			.unwrap();

			Self {
				scafalra,
				endpoint_cache: Some(endpoint.to_string()),
				..self
			}
		}

		pub fn with_content(self) -> Self {
			use crate::path_ext::*;

			let store_file = self.scafalra.path.join(Store::FILE_NAME);
			let bar_dir = self.scafalra.path.join_iter([
				Scafalra::CACHE_DIR_NAME,
				"foo",
				"bar",
			]);
			fs::create_dir_all(&bar_dir).unwrap();
			fs::write(bar_dir.join("baz.txt"), "").unwrap();
			fs::write(
				store_file,
				StoreJsonMock::new().push("bar", bar_dir).build(),
			)
			.unwrap();
			let scafalra = Scafalra::new(
				self.scafalra.path,
				self.endpoint_cache.as_deref(),
				Some("token"),
			)
			.unwrap();

			Self {
				scafalra,
				..self
			}
		}
	}

	pub struct RepoServerMock {
		pub server: ServerGuard,
		pub query_repo_mock: Mock,
		pub download_mock: Mock,
	}

	impl RepoServerMock {
		pub fn new() -> Self {
			let mut server = mockito::Server::new();

			let query_repo_mock = server
				.mock("POST", "/")
				.with_status(200)
				.with_header("content-type", "application/json")
				.with_body(mock_repo_response_json(&server.url()))
				.create();

			let download_mock = server
				.mock("GET", "/tarball")
				.with_status(200)
				.with_header("content-type", "application/gzip")
				.with_body_from_file("fixtures/scafalra-test.tar.gz")
				.create();

			Self {
				server,
				query_repo_mock,
				download_mock,
			}
		}
	}

	#[cfg(feature = "self_update")]
	pub struct ReleaseServerMock {
		pub server: ServerGuard,
		pub query_release_mock: Mock,
		pub download_mock: Mock,
	}

	#[cfg(feature = "self_update")]
	impl ReleaseServerMock {
		pub fn new() -> Self {
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
				.with_body(mock_release_response_json(
					&server.url(),
					&higher_ver,
				))
				.create();

			let download_mock = server
				.mock(
					"GET",
					format!(
						"/scafalra-{}-{}{}",
						higher_ver,
						get_self_target(),
						if cfg!(windows) {
							".zip"
						} else {
							".tar.gz"
						}
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

			Self {
				server,
				query_release_mock,
				download_mock,
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use std::{fs, path::PathBuf};

	use anyhow::Result;
	use similar_asserts::assert_eq;
	use tempfile::tempdir;

	use super::test_utils::{ReleaseServerMock, RepoServerMock, ScafalraMock};
	#[cfg(feature = "self_update")]
	use crate::cli::{UninstallArgs, UpdateArgs};
	use crate::{
		cli::{AddArgs, CreateArgs},
		path_ext::*,
		store::test_utils::StoreJsonMock,
	};

	#[test]
	fn test_scafalra_new() {
		let scafalra_mock = ScafalraMock::new();

		assert!(scafalra_mock.scafalra.cache_dir.exists());
		assert!(scafalra_mock.scafalra.store.path.exists());
		assert!(scafalra_mock.scafalra.config.path.exists());
	}

	#[test]
	fn test_scafalra_add() -> Result<()> {
		let repo_server_mock = RepoServerMock::new();
		let mut scafalra_mock =
			ScafalraMock::new().endpoint(&repo_server_mock.server.url());

		scafalra_mock.scafalra.add(AddArgs {
			repository: "foo/bar".to_string(),
			depth: 0,
			name: None,
		})?;

		repo_server_mock.query_repo_mock.assert();
		repo_server_mock.download_mock.assert();

		let bar_dir = scafalra_mock.scafalra.cache_dir.join_slash("foo/bar");
		let actual = fs::read_to_string(&scafalra_mock.scafalra.store.path)?;
		let expect = StoreJsonMock::new().push("bar", &bar_dir).build();

		assert!(bar_dir.exists());
		assert_eq!(actual, expect);

		Ok(())
	}

	#[test]
	fn test_scafalra_add_specified_name() -> Result<()> {
		let repo_server_mock = RepoServerMock::new();
		let mut scafalra_mock =
			ScafalraMock::new().endpoint(&repo_server_mock.server.url());

		scafalra_mock.scafalra.add(AddArgs {
			repository: "foo/bar".to_string(),
			depth: 0,
			name: Some("foo".to_string()),
		})?;

		repo_server_mock.query_repo_mock.assert();
		repo_server_mock.download_mock.assert();

		let bar_dir = scafalra_mock.scafalra.cache_dir.join_slash("foo/bar");
		let actual = fs::read_to_string(&scafalra_mock.scafalra.store.path)?;
		let expect = StoreJsonMock::new().push("foo", &bar_dir).build();

		assert!(bar_dir.exists());
		assert_eq!(actual, expect);

		Ok(())
	}

	#[test]
	fn test_scafalra_add_depth_1() -> Result<()> {
		let repo_server_mock = RepoServerMock::new();
		let mut scafalra_mock =
			ScafalraMock::new().endpoint(&repo_server_mock.server.url());

		scafalra_mock.scafalra.add(AddArgs {
			repository: "foo/bar".to_string(),
			depth: 1,
			name: Some("foo".to_string()),
		})?;

		repo_server_mock.query_repo_mock.assert();
		repo_server_mock.download_mock.assert();

		let bar_dir = scafalra_mock.scafalra.cache_dir.join_slash("foo/bar");
		let actual = fs::read_to_string(&scafalra_mock.scafalra.store.path)?;
		let expect = StoreJsonMock::new()
			.push("a", bar_dir.join("a"))
			.push("b", bar_dir.join("b"))
			.push("c", bar_dir.join("c"))
			.push("node_modules", bar_dir.join("node_modules"))
			.all_sub_template()
			.build();

		assert!(bar_dir.exists());
		assert_eq!(actual, expect);

		Ok(())
	}

	#[test]
	fn test_scafalra_add_subdir() -> Result<()> {
		let repo_server_mock = RepoServerMock::new();
		let mut scafalra_mock =
			ScafalraMock::new().endpoint(&repo_server_mock.server.url());

		scafalra_mock.scafalra.add(AddArgs {
			repository: "foo/bar/a/a1".to_string(),
			depth: 0,
			name: None,
		})?;

		repo_server_mock.query_repo_mock.assert();
		repo_server_mock.download_mock.assert();

		let a1_dir =
			scafalra_mock.scafalra.cache_dir.join_slash("foo/bar/a/a1");
		let actual = fs::read_to_string(&scafalra_mock.scafalra.store.path)?;
		let expect = StoreJsonMock::new().push("a1", &a1_dir).build();

		assert!(a1_dir.exists());
		assert_eq!(actual, expect);

		Ok(())
	}

	#[test]
	fn test_scafalra_add_subdir_and_depth_1() -> Result<()> {
		let repo_server_mock = RepoServerMock::new();
		let mut scafalra_mock =
			ScafalraMock::new().endpoint(&repo_server_mock.server.url());

		scafalra_mock.scafalra.add(AddArgs {
			repository: "foo/bar/a".to_string(),
			depth: 1,
			name: None,
		})?;

		repo_server_mock.query_repo_mock.assert();
		repo_server_mock.download_mock.assert();

		let a_dir = scafalra_mock.scafalra.cache_dir.join_slash("foo/bar/a");
		let a1_dir = a_dir.join("a1");
		let a2_dir = a_dir.join("a2");
		let a3_dir = a_dir.join("a3");
		let actual = fs::read_to_string(&scafalra_mock.scafalra.store.path)?;
		let expect = StoreJsonMock::new()
			.push("a1", &a1_dir)
			.push("a2", &a2_dir)
			.push("a3", &a3_dir)
			.all_sub_template()
			.build();

		assert!(a1_dir.exists());
		assert!(a2_dir.exists());
		assert!(a3_dir.exists());
		assert_eq!(actual, expect);

		Ok(())
	}

	#[test]
	fn test_scafalra_create() -> Result<()> {
		let scafalra_mock = ScafalraMock::new().with_content();

		let tmpdir_path = scafalra_mock.tmpdir.path();

		scafalra_mock.scafalra.create(CreateArgs {
			name: "bar".to_string(),
			// Due to chroot restrictions, a directory is specified here to
			// simulate the current working directory
			directory: Some(tmpdir_path.join("bar")),
			with: None,
		})?;

		assert!(tmpdir_path.join_slash("bar/baz.txt").exists());

		Ok(())
	}

	#[test]
	fn test_scafalra_create_not_found() -> Result<()> {
		let scafalra_mock = ScafalraMock::new();

		let ret = scafalra_mock.scafalra.create(CreateArgs {
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
		let release_server_mock = ReleaseServerMock::new();
		let scafalra_mock =
			ScafalraMock::new().endpoint(&release_server_mock.server.url());

		scafalra_mock.scafalra.update(UpdateArgs {
			check: true,
		})?;

		release_server_mock.query_release_mock.assert();
		assert!(!scafalra_mock.scafalra.update_dir.exists());

		Ok(())
	}

	#[test]
	#[cfg(feature = "self_update")]
	fn test_scafalra_update() -> Result<()> {
		let release_server_mock = ReleaseServerMock::new();
		let scafalra_mock =
			ScafalraMock::new().endpoint(&release_server_mock.server.url());

		scafalra_mock.scafalra.update(UpdateArgs {
			check: false,
		})?;

		release_server_mock.query_release_mock.assert();
		release_server_mock.download_mock.assert();
		assert!(!scafalra_mock.scafalra.update_dir.exists());

		Ok(())
	}

	#[test]
	#[cfg(feature = "self_update")]
	fn test_scafalra_uninstall() -> Result<()> {
		let scafalra_mock = ScafalraMock::new();

		scafalra_mock.scafalra.uninstall(UninstallArgs {
			keep_data: false,
		})?;

		assert!(!scafalra_mock.scafalra.path.exists());

		Ok(())
	}

	#[test]
	#[cfg(feature = "self_update")]
	fn test_scafalra_uninstall_keep_data() -> Result<()> {
		let scafalra_mock = ScafalraMock::new();

		scafalra_mock.scafalra.uninstall(UninstallArgs {
			keep_data: true,
		})?;

		assert!(scafalra_mock.scafalra.path.exists());

		Ok(())
	}

	fn is_all_exists(paths: &[PathBuf]) -> bool {
		paths.iter().any(|p| !p.exists())
	}

	#[test]
	fn test_scafalra_copy_on_add() -> Result<()> {
		let repo_server_mock = RepoServerMock::new();
		let mut scafalra_mock =
			ScafalraMock::new().endpoint(&repo_server_mock.server.url());

		scafalra_mock.scafalra.add(AddArgs {
			repository: "foo/bar".to_string(),
			depth: 1,
			name: Some("foo".to_string()),
		})?;

		let template_dir =
			scafalra_mock.scafalra.cache_dir.join_slash("foo/bar");

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
		let repo_server_mock = RepoServerMock::new();
		let mut scafalra_mock =
			ScafalraMock::new().endpoint(&repo_server_mock.server.url());

		scafalra_mock.scafalra.add(AddArgs {
			repository: "foo/bar".to_string(),
			depth: 1,
			name: Some("foo".to_string()),
		})?;

		let tmp_dir = tempdir()?;
		let dest = tmp_dir.path().join("dest");

		scafalra_mock.scafalra.create(CreateArgs {
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
