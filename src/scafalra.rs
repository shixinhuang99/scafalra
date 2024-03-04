use std::{
	env,
	path::{Component, Path, PathBuf},
};

use anyhow::Result;
use fs_err as fs;
use remove_dir_all::remove_dir_all;

use crate::{
	api::GitHubApi,
	cli::{AddArgs, CreateArgs, ListArgs, RemoveArgs, RenameArgs, TokenArgs},
	config::Config,
	debug,
	path_ext::*,
	repository::Repository,
	repository_config::RepositoryConfig,
	store::{Store, Template, TemplateBuilder},
};

pub struct Scafalra {
	pub path: PathBuf,
	cache_dir: PathBuf,
	config: Config,
	store: Store,
	github_api: GitHubApi,
}

impl Scafalra {
	const CACHE_DIR_NAME: &'static str = "cache";
	const TMP_DIR_NAME: &'static str = "t";

	pub fn new(
		scfalra_dir: PathBuf,
		endpoint: Option<&str>,
		token: Option<&str>,
	) -> Result<Self> {
		let cache_dir = scfalra_dir.join(Self::CACHE_DIR_NAME);

		if !cache_dir.exists() {
			fs::create_dir_all(&cache_dir)?;
		}

		let config = Config::new(&scfalra_dir)?;
		let store = Store::new(&scfalra_dir)?;
		let mut github_api = GitHubApi::new(endpoint);

		if let Some(token) = token.or_else(|| config.token()) {
			github_api.set_token(token);
		}

		Ok(Self {
			path: scfalra_dir,
			cache_dir,
			config,
			store,
			github_api,
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

	fn cache_template(
		&self,
		repo: &Repository,
		args: &AddArgs,
	) -> Result<PathBuf> {
		let tmp_dir = self.cache_dir.join(Self::TMP_DIR_NAME);
		let zipball_path = self.github_api.download(repo, args, &tmp_dir)?;
		let zipball = fs::File::open(&zipball_path)?;

		let mut archive = zip::ZipArchive::new(&zipball)?;
		archive.extract(&tmp_dir)?;

		let first_dir = tmp_dir
			.read_dir()?
			.next()
			.ok_or(anyhow::anyhow!("Empty directory"))??
			.path();

		debug!("first_dir: {:?}", first_dir);

		let template_dir = self.cache_dir.join_iter([&repo.owner, &repo.name]);

		if template_dir.exists() {
			remove_dir_all(&template_dir)?;
		}

		dircpy::copy_dir(first_dir, &template_dir)?;

		fs::remove_file(zipball_path)?;
		remove_dir_all(tmp_dir)?;

		Ok(template_dir)
	}

	pub fn add(&mut self, args: AddArgs) -> Result<()> {
		debug!("args: {:#?}", args);

		let repo = Repository::parse(&args.repository)?;

		println!("Downloading `{}` ...", args.repository);

		let mut template_dir = self.cache_template(&repo, &args)?;

		debug!("template_dir: {:?}", template_dir);

		let mut template_name = args.name.unwrap_or(repo.name.clone());

		if let Some(subdir) = args.subdir {
			Path::new(&subdir)
				.components()
				.filter(|c| matches!(c, Component::Normal(_)))
				.for_each(|c| {
					template_dir.push(c);
				});

			debug!("template_dir: {:?}", template_dir);

			if let Some(name) = template_dir.file_name() {
				template_name = name.to_string_lossy().to_string();
			}
		}

		match args.depth {
			0 => {
				self.store.add(Template::new(
					template_name,
					repo.url(),
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
								repo.url(),
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
			let suggestion = self.store.similar_name_suggestion(&args.name);
			anyhow::bail!("{}", suggestion);
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

		globs.sort_unstable();
		globs.dedup();

		if let Ok(gw) = globwalk::GlobWalkerBuilder::from_patterns(from, &globs)
			.case_insensitive(true)
			.build()
		{
			glob_walk_and_copy(gw, dest);
		}
	}

	pub fn rename(&mut self, args: RenameArgs) -> Result<()> {
		debug!("args: {:#?}", args);

		let renamed = self.store.rename(&args.name, &args.new_name);

		if renamed {
			self.store.save()?;
		}

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
	use std::fs;

	use mockito::{Mock, ServerGuard};
	use tempfile::{tempdir, TempDir};

	use super::Scafalra;
	use crate::store::{test_utils::StoreJsonMock, Store};

	pub struct ScafalraMock {
		pub scafalra: Scafalra,
		pub tmp_dir: TempDir,
		endpoint_cache: Option<String>,
	}

	impl ScafalraMock {
		pub fn new() -> Self {
			let tmp_dir = tempdir().unwrap();
			let scafalra = Scafalra::new(
				tmp_dir.path().join("scafalra"),
				None,
				Some("token"),
			)
			.unwrap();

			Self {
				scafalra,
				tmp_dir,
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

	pub struct ServerMock {
		pub server: ServerGuard,
		pub download_mock: Mock,
	}

	impl ServerMock {
		pub fn new() -> Self {
			let mut server = mockito::Server::new();

			let download_mock = server
				.mock("GET", "/repos/foo/bar/zipball")
				.with_status(200)
				.with_header("content-type", "application/zip")
				.with_body_from_file("fixtures/scafalra-test.zip")
				.create();

			Self {
				server,
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

	use super::test_utils::{ScafalraMock, ServerMock};
	use crate::{
		cli::{test_utils::AddArgsMock, CreateArgs},
		path_ext::*,
		store::test_utils::StoreJsonMock,
	};

	#[test]
	fn test_scafalra_new() {
		let ScafalraMock {
			tmp_dir: _tmp_dir,
			scafalra,
			..
		} = ScafalraMock::new();

		assert!(scafalra.cache_dir.exists());
		assert!(scafalra.store.path.exists());
		assert!(scafalra.config.path.exists());
	}

	#[test]
	fn test_scafalra_add() -> Result<()> {
		let ServerMock {
			server,
			download_mock,
		} = ServerMock::new();

		let ScafalraMock {
			tmp_dir: _tmp_dir,
			mut scafalra,
			..
		} = ScafalraMock::new().endpoint(&server.url());

		scafalra.add(AddArgsMock::new().build())?;

		download_mock.assert();

		let bar_dir = scafalra.cache_dir.join_slash("foo/bar");
		let actual = fs::read_to_string(&scafalra.store.path)?;
		let expect = StoreJsonMock::new().push("bar", &bar_dir).build();

		assert!(bar_dir.exists());
		assert_eq!(actual, expect);

		Ok(())
	}

	#[test]
	fn test_scafalra_add_specified_name() -> Result<()> {
		let ServerMock {
			server,
			download_mock,
		} = ServerMock::new();

		let ScafalraMock {
			tmp_dir: _tmp_dir,
			mut scafalra,
			..
		} = ScafalraMock::new().endpoint(&server.url());

		scafalra.add(AddArgsMock::new().name("foo").build())?;

		download_mock.assert();

		let bar_dir = scafalra.cache_dir.join_slash("foo/bar");
		let actual = fs::read_to_string(&scafalra.store.path)?;
		let expect = StoreJsonMock::new().push("foo", &bar_dir).build();

		assert!(bar_dir.exists());
		assert_eq!(actual, expect);

		Ok(())
	}

	#[test]
	fn test_scafalra_add_depth_1() -> Result<()> {
		let ServerMock {
			server,
			download_mock,
		} = ServerMock::new();

		let ScafalraMock {
			tmp_dir: _tmp_dir,
			mut scafalra,
			..
		} = ScafalraMock::new().endpoint(&server.url());

		scafalra.add(AddArgsMock::new().depth(1).build())?;

		download_mock.assert();

		let bar_dir = scafalra.cache_dir.join_slash("foo/bar");
		let actual = fs::read_to_string(&scafalra.store.path)?;
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
		let ServerMock {
			server,
			download_mock,
		} = ServerMock::new();

		let ScafalraMock {
			tmp_dir: _tmp_dir,
			mut scafalra,
			..
		} = ScafalraMock::new().endpoint(&server.url());

		scafalra.add(AddArgsMock::new().subdir("/a/a1").build())?;

		download_mock.assert();

		let a1_dir = scafalra.cache_dir.join_slash("foo/bar/a/a1");
		let actual = fs::read_to_string(&scafalra.store.path)?;
		let expect = StoreJsonMock::new().push("a1", &a1_dir).build();

		assert!(a1_dir.exists());
		assert_eq!(actual, expect);

		Ok(())
	}

	#[test]
	fn test_scafalra_add_subdir_and_depth_1() -> Result<()> {
		let ServerMock {
			server,
			download_mock,
		} = ServerMock::new();

		let ScafalraMock {
			tmp_dir: _tmp_dir,
			mut scafalra,
			..
		} = ScafalraMock::new().endpoint(&server.url());

		scafalra.add(AddArgsMock::new().subdir("/a").depth(1).build())?;

		download_mock.assert();

		let a_dir = scafalra.cache_dir.join_slash("foo/bar/a");
		let a1_dir = a_dir.join("a1");
		let a2_dir = a_dir.join("a2");
		let a3_dir = a_dir.join("a3");
		let actual = fs::read_to_string(&scafalra.store.path)?;
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
		let ScafalraMock {
			tmp_dir,
			scafalra,
			..
		} = ScafalraMock::new().with_content();

		let tmp_dir_path = tmp_dir.path();

		scafalra.create(CreateArgs {
			name: "bar".to_string(),
			// Due to chroot restrictions, a directory is specified here to
			// simulate the current working directory
			directory: Some(tmp_dir_path.join("bar")),
			with: None,
		})?;

		assert!(tmp_dir_path.join_slash("bar/baz.txt").exists());

		Ok(())
	}

	#[test]
	fn test_scafalra_create_not_found() -> Result<()> {
		let ScafalraMock {
			tmp_dir: _tmp_dir,
			scafalra,
			..
		} = ScafalraMock::new();

		let ret = scafalra.create(CreateArgs {
			name: "bar".to_string(),
			directory: None,
			with: None,
		});

		assert!(ret.is_err());

		Ok(())
	}

	fn is_all_exists(paths: &[PathBuf]) -> bool {
		paths.iter().any(|p| !p.exists())
	}

	#[test]
	fn test_scafalra_copy_on_add() -> Result<()> {
		let ServerMock {
			server, ..
		} = ServerMock::new();

		let ScafalraMock {
			tmp_dir: _tmp_dir,
			mut scafalra,
			..
		} = ScafalraMock::new().endpoint(&server.url());

		scafalra.add(AddArgsMock::new().depth(1).build())?;

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
		let ServerMock {
			server, ..
		} = ServerMock::new();

		let ScafalraMock {
			tmp_dir: _tmp_dir,
			mut scafalra,
			..
		} = ScafalraMock::new().endpoint(&server.url());

		scafalra.add(AddArgsMock::new().depth(1).build())?;

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
