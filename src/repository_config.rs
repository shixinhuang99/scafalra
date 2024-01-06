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
mod tests {
	use std::{collections::HashMap, path::PathBuf};

	use anyhow::Result;

	use super::RepositoryConfig;

	#[test]
	fn test_config_file_exists() -> Result<()> {
		let template_dir = PathBuf::from("fixtures");
		let repo_cfg = RepositoryConfig::load(&template_dir);

		assert_eq!(
			repo_cfg.copy_on_add,
			HashMap::from_iter([(
				"foo".to_string(),
				vec!["baz".to_string()]
			)])
		);

		Ok(())
	}

	#[test]
	fn test_config_file_not_exists() -> Result<()> {
		let template_dir = tempfile::tempdir()?;
		let repo_cfg = RepositoryConfig::load(template_dir.path());

		assert!(repo_cfg.copy_on_add.is_empty());

		Ok(())
	}
}
