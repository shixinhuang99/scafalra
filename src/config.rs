use anyhow::Result;
use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};

use crate::json_content::JsonContent;

#[derive(Deserialize, Serialize, Default)]
struct ConfigContent {
	token: Option<String>,
}

impl JsonContent for ConfigContent {}

pub struct Config {
	pub path: Utf8PathBuf,
	content: ConfigContent,
}

impl Config {
	pub const FILE_NAME: &'static str = "config.json";

	pub fn new(scafalra_dir: &Utf8Path) -> Result<Self> {
		let path = scafalra_dir.join(Self::FILE_NAME);
		let content = ConfigContent::load(&path)?;

		Ok(Self { path, content })
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
mod tests {
	use std::{fs, io::Write};

	use anyhow::Result;
	use pretty_assertions::assert_eq;
	use tempfile::{tempdir, TempDir};

	use super::Config;
	use crate::utf8_path::Utf8PathBufExt;

	fn mock_config(create_file: bool) -> Result<(Config, TempDir)> {
		let temp_dir = tempdir()?;
		let temp_dir_path = temp_dir.path().into_utf8_path_buf()?;

		if create_file {
			let file_path = temp_dir_path.join(Config::FILE_NAME);
			let mut file = fs::File::create(file_path)?;
			file.write_all(b"{\n\"token\": \"token\"\n}\n")?;
		}

		let config = Config::new(&temp_dir_path)?;

		Ok((config, temp_dir))
	}

	#[test]
	fn test_config_new_not_exists() -> Result<()> {
		let (config, _dir) = mock_config(false)?;

		assert_eq!(config.token(), None);

		Ok(())
	}

	#[test]
	fn test_config_new_exists() -> Result<()> {
		let (config, _dir) = mock_config(true)?;

		assert_eq!(config.token(), Some("token"));

		Ok(())
	}

	#[test]
	fn test_config_save() -> Result<()> {
		let (mut config, _dir) = mock_config(false)?;

		config.set_token("token2");
		config.save()?;

		let content = fs::read_to_string(&config.path)?;
		assert_eq!(content, "{\n\"token\": \"token2\"\n}\n");

		Ok(())
	}
}
