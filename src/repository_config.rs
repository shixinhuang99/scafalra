use std::{collections::HashMap, path::Path};

use fs_err as fs;
use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct RepositoryConfig {
	pub linking: HashMap<String, Vec<String>>,
}

impl RepositoryConfig {
	const FILE_PATH: &'static str = ".scafalra/scafalra.json";

	pub fn new() -> Self {
		Self {
			linking: HashMap::with_capacity(0),
		}
	}

	pub fn load(&mut self, template_dir: &Path) {
		use crate::path_ext::*;

		let file = template_dir.join_slash(Self::FILE_PATH);

		if let Ok(content) = fs::read_to_string(file) {
			if let Ok(value) = serde_json::from_str::<Self>(&content) {
				self.linking = value.linking;
			}
		}
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
		let mut repo_cfg = RepositoryConfig::new();

		repo_cfg.load(&template_dir);

		assert_eq!(
			repo_cfg.linking,
			HashMap::from_iter([("foo".to_string(), vec!["baz".to_string()])])
		);

		Ok(())
	}

	#[test]
	fn test_config_file_not_exists() -> Result<()> {
		let template_dir = tempfile::tempdir()?;
		let mut repo_cfg = RepositoryConfig::new();

		repo_cfg.load(template_dir.path());

		assert!(repo_cfg.linking.is_empty());

		Ok(())
	}
}
