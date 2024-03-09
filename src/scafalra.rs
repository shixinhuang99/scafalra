use std::{
	collections::HashMap,
	env, fs,
	path::{Path, PathBuf},
};

use anyhow::Result;
use remove_dir_all::remove_dir_all;

use crate::{
	api::GitHubApi,
	cli::{AddArgs, CreateArgs, ListArgs, RemoveArgs, RenameArgs, TokenArgs},
	config::Config,
	debug,
	interactive::{fuzzy_select, input, multi_select},
	path_ext::*,
	repository::Repository,
	store::Store,
	sub_template::SUB_TEMPLATE_DIR,
	template::Template,
};

pub struct Scafalra {
	pub path: PathBuf,
	cache_dir: PathBuf,
	config: Config,
	store: Store,
	github_api: GitHubApi,
	pub interactive_mode: bool,
}

impl Scafalra {
	const CACHE_DIR_NAME: &'static str = "cache";
	const TMP_DIR_NAME: &'static str = "t";

	pub fn new(
		path: PathBuf,
		endpoint: Option<&str>,
		token: Option<&str>,
	) -> Result<Self> {
		let cache_dir = path.join(Self::CACHE_DIR_NAME);

		if !cache_dir.exists() {
			fs::create_dir_all(&cache_dir)?;
		}

		let config = Config::new(&path)?;
		let store = Store::new(&path)?;
		let mut github_api = GitHubApi::new(endpoint);

		if let Some(token) = token.or_else(|| config.token()) {
			github_api.set_token(token);
		}

		Ok(Self {
			path,
			cache_dir,
			config,
			store,
			github_api,
			interactive_mode: false,
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
			template_dir.join_canonicalize(Path::new(&subdir));

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
				for entry_path in template_dir
					.read_dir()?
					.filter_map(|entry| entry.ok().map(|e| e.path()))
				{
					if entry_path.is_dir() {
						if let Some(Some(file_name)) =
							entry_path.file_name().map(|f| f.to_str())
						{
							if !file_name.starts_with('.') {
								self.store.add(Template::new(
									file_name,
									repo.url(),
									&entry_path,
								));
							}
						}
					}
				}
			}
			_ => anyhow::bail!("The argument `depth` allows only 0 or 1"),
		}

		self.store.save()?;

		Ok(())
	}

	pub fn create(&self, args: CreateArgs) -> Result<()> {
		debug!("args: {:#?}", args);

		let tpl_name = match (&args.name, self.interactive_mode) {
			(Some(arg_name), false) => Some(arg_name),
			(_, true) => fuzzy_select(self.store.all_templates_name())?,
			_ => {
				anyhow::bail!(
					"Provide a name or opt for interactive mode with the `-i` argument"
				)
			}
		};

		let Some(tpl_name) = tpl_name else {
			return Ok(());
		};

		let Some(template) = self.store.get(tpl_name) else {
			let suggestion = self.store.similar_name_suggestion(tpl_name);
			anyhow::bail!("{}", suggestion);
		};

		let cwd = env::current_dir()?;

		debug!("cwd: {:?}", cwd);

		let dest = if let Some(arg_dir) = args.directory {
			if arg_dir.is_absolute() {
				arg_dir
			} else {
				let mut ret = cwd.clone();
				ret.join_canonicalize(&arg_dir);
				ret
			}
		} else {
			cwd.join(tpl_name)
		};

		debug!("dest: {:?}", dest);

		let dest_display = dest.to_string_lossy();

		if dest.exists() {
			anyhow::bail!("`{}` is already exists", dest_display);
		}

		let sub_tpl_map: HashMap<&String, &PathBuf> = HashMap::from_iter(
			template
				.sub_templates
				.iter()
				.map(|sub_tpl| (&sub_tpl.name, &sub_tpl.path)),
		);

		let sub_tpl_names = match (&args.sub_templates, self.interactive_mode) {
			(Some(arg_sub_tpl_names), false) => {
				Some(arg_sub_tpl_names.iter().collect())
			}
			(_, true) => {
				multi_select(
					template
						.sub_templates
						.iter()
						.map(|sub_tpl| &sub_tpl.name)
						.collect(),
				)?
			}
			_ => None,
		};

		dircpy::copy_dir(&template.path, &dest)?;

		let sbu_tpl_dir = dest.join(SUB_TEMPLATE_DIR);
		if sbu_tpl_dir.exists() {
			let _ = fs::remove_dir_all(sbu_tpl_dir);
		}

		if let Some(sub_tpl_names) = sub_tpl_names {
			for name in sub_tpl_names {
				if let Some(sub_tpl) = sub_tpl_map.get(name) {
					let _ = dircpy::copy_dir(sub_tpl, dest.join(name));
				}
			}
		}

		println!("Created in `{}`", dest_display);

		Ok(())
	}

	pub fn rename(&mut self, args: RenameArgs) -> Result<()> {
		debug!("args: {:#?}", args);

		let (name, new_name) = match (
			args.name,
			args.new_name,
			self.interactive_mode,
		) {
			(Some(name), Some(new_name), false) => (name, new_name),
			(_, _, true) => {
				let name = fuzzy_select(self.store.all_templates_name())?;
				let Some(name) = name else {
					return Ok(());
				};
				let new_name = input("New name?")?;
				(name.clone(), new_name)
			}
			(Some(_), None, false) => {
				anyhow::bail!("Please provide a new name")
			}
			(_, _, _) => {
				anyhow::bail!(
					"Provide both the target and new names, or opt for interactive mode with the `-i` argument"
				)
			}
		};

		let renamed = self.store.rename(&name, &new_name);

		if renamed {
			self.store.save()?;
			println!("{} -> {}", name, new_name);
		}

		Ok(())
	}

	pub fn remove(&mut self, args: RemoveArgs) -> Result<()> {
		debug!("args: {:#?}", args);

		let names = match (args.names, self.interactive_mode) {
			(Some(names), false) => Some(names),
			(_, true) => {
				multi_select(self.store.all_templates_name())?
					.map(|vs| vs.into_iter().cloned().collect())
			}
			_ => {
				anyhow::bail!(
					"Provide names or opt for interactive mode with the `-i` argument"
				)
			}
		};

		let Some(names) = names else {
			return Ok(());
		};

		for name in names {
			self.store.remove(&name)?;
		}

		self.store.save()?;

		Ok(())
	}
}

#[cfg(test)]
mod test_utils {
	use std::fs;

	use mockito::{Mock, ServerGuard};
	use tempfile::{tempdir, TempDir};

	use super::Scafalra;
	use crate::{
		store::{test_utils::StoreJsonMock, Store},
		sub_template::test_utils::sub_tempaltes_dir_setup,
	};

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

		/// create a template that name is `bar`
		pub fn with_content(self) -> Self {
			use crate::path_ext::*;

			let store_file = self.scafalra.path.join(Store::FILE_NAME);
			let bar_dir = self.scafalra.path.join_iter([
				Scafalra::CACHE_DIR_NAME,
				"foo",
				"bar",
			]);
			fs::create_dir_all(&bar_dir).unwrap();
			sub_tempaltes_dir_setup(&bar_dir, &["dir-1", "dir-2"]);
			fs::write(bar_dir.join("baz.txt"), "").unwrap();
			fs::write(
				store_file,
				StoreJsonMock::new().push("bar", &bar_dir).build(),
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
	use std::fs;

	use anyhow::Result;
	use similar_asserts::assert_eq;

	use super::test_utils::{ScafalraMock, ServerMock};
	use crate::{
		cli::{test_utils::AddArgsMock, CreateArgs, RemoveArgs, RenameArgs},
		path_ext::*,
		store::test_utils::StoreJsonMock,
		sub_template::SUB_TEMPLATE_DIR,
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
	fn test_scafalra_add_basic() -> Result<()> {
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
			.push("a", &bar_dir.join("a"))
			.push("b", &bar_dir.join("b"))
			.push("c", &bar_dir.join("c"))
			.push("node_modules", &bar_dir.join("node_modules"))
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
			.build();

		assert!(a1_dir.exists());
		assert!(a2_dir.exists());
		assert!(a3_dir.exists());
		assert_eq!(actual, expect);

		Ok(())
	}

	#[test]
	fn test_scafalra_create_ok() -> Result<()> {
		let ScafalraMock {
			tmp_dir,
			scafalra,
			..
		} = ScafalraMock::new().with_content();

		let bar_dir = tmp_dir.path().join("bar");

		scafalra.create(CreateArgs {
			name: Some("bar".to_string()),
			// Due to chroot restrictions, a directory is specified here to
			// simulate the current working directory
			directory: Some(bar_dir.clone()),
			sub_templates: Some(vec!["dir-1".to_string()]),
		})?;

		assert!(bar_dir.join("baz.txt").exists());
		assert!(bar_dir.join("dir-1").exists());
		assert!(!bar_dir.join(SUB_TEMPLATE_DIR).exists());

		Ok(())
	}

	#[test]
	fn test_scafalra_create_bad_args() -> Result<()> {
		let ScafalraMock {
			tmp_dir: _tmp_dir,
			scafalra,
			..
		} = ScafalraMock::new();

		let ret = scafalra.create(CreateArgs {
			name: None,
			directory: None,
			sub_templates: None,
		});

		assert!(ret.is_err());

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
			name: Some("bar".to_string()),
			directory: None,
			sub_templates: None,
		});

		assert!(ret.is_err());

		Ok(())
	}

	#[test]
	fn test_scafalra_remove_bad_args() -> Result<()> {
		let ScafalraMock {
			tmp_dir: _tmp_dir,
			mut scafalra,
			..
		} = ScafalraMock::new();

		let ret = scafalra.remove(RemoveArgs {
			names: None,
		});

		assert!(ret.is_err());

		Ok(())
	}

	#[test]
	fn test_scafalra_rename_bad_args() -> Result<()> {
		let ScafalraMock {
			tmp_dir: _tmp_dir,
			mut scafalra,
			..
		} = ScafalraMock::new();

		let ret = scafalra.rename(RenameArgs {
			name: None,
			new_name: None,
		});

		assert!(ret.is_err());

		Ok(())
	}
}
