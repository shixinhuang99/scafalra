use std::{collections::HashMap, path::Path};

use anyhow::Result;
use fs_err as fs;
use serde::Deserialize;

#[derive(Deserialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct RepositoryConfig {
	pub copy_on_add: HashMap<String, Vec<String>>,
}

impl RepositoryConfig {
	pub const DIR_NAME: &'static str = ".scafalra";
	pub const FILE_NAME: &'static str = "scafalra.json";

	fn try_load(template_dir: &Path) -> Result<Self> {
		use crate::path_ext::*;

		let file = template_dir.join_iter([Self::DIR_NAME, Self::FILE_NAME]);
		let content = fs::read_to_string(file)?;
		let value: Self = serde_json::from_str(&content)?;

		Ok(value)
	}

	pub fn load(template_dir: &Path) -> Self {
		Self::try_load(template_dir).unwrap_or_default()
	}
}

#[cfg(test)]
mod test_utils {
	use std::path::PathBuf;

	use tempfile::{tempdir, TempDir};

	use super::RepositoryConfig;

	pub struct RepositoryConfigMock {
		pub repo_cfg: RepositoryConfig,
		pub tmpdir: TempDir,
		pub path: PathBuf,
	}

	impl RepositoryConfigMock {
		pub fn new() -> Self {
			let tmpdir = tempdir().unwrap();
			let tmpdir_path = tmpdir.path().to_path_buf();
			let repo_cfg = RepositoryConfig::load(&tmpdir_path);

			Self {
				repo_cfg,
				tmpdir,
				path: tmpdir_path,
			}
		}

		pub fn with_fixture(self) -> Self {
			let fixture_path = PathBuf::from("fixtures");
			let repo_cfg = RepositoryConfig::load(&fixture_path);

			Self {
				repo_cfg,
				path: fixture_path,
				..self
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use std::collections::HashMap;

	use anyhow::Result;

	use super::test_utils::RepositoryConfigMock;

	#[test]
	fn test_config_file_load() {
		let repo_cfg_mock = RepositoryConfigMock::new().with_fixture();

		assert_eq!(
			repo_cfg_mock.repo_cfg.copy_on_add,
			HashMap::from_iter([("foo".to_string(), vec!["baz".to_string()])])
		);
	}

	#[test]
	fn test_config_load_file_not_exists() -> Result<()> {
		let repo_cfg_mock = RepositoryConfigMock::new();

		assert!(repo_cfg_mock.repo_cfg.copy_on_add.is_empty());

		Ok(())
	}
}
