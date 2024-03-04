use std::path::{Path, PathBuf};

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::json::JsonContent;

#[derive(Deserialize, Serialize, Default)]
struct ConfigContent {
	token: Option<String>,
}

impl JsonContent for ConfigContent {}

pub struct Config {
	pub path: PathBuf,
	content: ConfigContent,
}

impl Config {
	pub const FILE_NAME: &'static str = "config.json";

	pub fn new(scafalra_dir: &Path) -> Result<Self> {
		let path = scafalra_dir.join(Self::FILE_NAME);
		let content = ConfigContent::load(&path)?;

		Ok(Self {
			path,
			content,
		})
	}

	pub fn save(&self) -> Result<()> {
		self.content.save(&self.path)
	}

	pub fn set_token(&mut self, token: &str) {
		self.content.token = Some(token.to_string());
	}

	pub fn token(&self) -> Option<&str> {
		self.content.token.as_deref()
	}
}

#[cfg(test)]
mod test_utils {
	use std::fs;

	use tempfile::{tempdir, TempDir};

	use super::Config;

	pub struct ConfigMock {
		pub config: Config,
		pub tmp_dir: TempDir,
	}

	impl ConfigMock {
		pub fn new() -> Self {
			let tmp_dir = tempdir().unwrap();
			let config = Config::new(tmp_dir.path()).unwrap();

			Self {
				tmp_dir,
				config,
			}
		}

		pub fn with_content(self) -> Self {
			let tmp_dir_path = self.tmp_dir.path();

			fs::write(
				tmp_dir_path.join(Config::FILE_NAME),
				"{\n  \"token\": \"token\"\n}",
			)
			.unwrap();

			let config = Config::new(tmp_dir_path).unwrap();

			Self {
				config,
				..self
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use std::fs;

	use anyhow::Result;

	use super::test_utils::ConfigMock;

	#[test]
	fn test_config_new_not_exists() {
		let ConfigMock {
			config, ..
		} = ConfigMock::new();

		assert_eq!(config.token(), None);
	}

	#[test]
	fn test_config_new_exists() {
		let ConfigMock {
			config, ..
		} = ConfigMock::new().with_content();

		assert_eq!(config.token(), Some("token"));
	}

	#[test]
	fn test_config_save() -> Result<()> {
		let ConfigMock {
			mut config, ..
		} = ConfigMock::new();

		config.set_token("token2");
		config.save()?;

		let actual = fs::read_to_string(&config.path)?;
		assert_eq!(actual, "{\n  \"token\": \"token2\"\n}");

		Ok(())
	}
}
