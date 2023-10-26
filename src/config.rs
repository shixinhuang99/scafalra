use anyhow::Result;
use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};

use crate::toml_content::TomlContent;

#[derive(Deserialize, Serialize, Default)]
struct ConfigContent {
	token: Option<String>,
}

impl TomlContent for ConfigContent {}

pub struct Config {
	path: Utf8PathBuf,
	content: ConfigContent,
}

impl Config {
	pub fn new(scafalra_dir: &Utf8Path) -> Result<Self> {
		let path = scafalra_dir.join("config.toml");
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
	use camino::Utf8Path;
	use pretty_assertions::assert_eq;
	use tempfile::{tempdir, TempDir};

	use super::Config;

	fn build(create_file: bool) -> Result<(Config, TempDir)> {
		let temp_dir = tempdir()?;
		let temp_dir_path = Utf8Path::from_path(temp_dir.path()).unwrap();

		if create_file {
			let file_path = temp_dir_path.join("config.toml");
			let mut file = fs::File::create(file_path)?;
			file.write_all(b"token = \"token\"\n")?;
		}

		let config = Config::new(temp_dir_path)?;

		Ok((config, temp_dir))
	}

	#[test]
	fn test_config_new_not_exists() -> Result<()> {
		let (config, dir) = build(false)?;

		assert_eq!(config.path, dir.path().join("config.toml"));
		assert_eq!(config.token(), None);

		Ok(())
	}

	#[test]
	fn test_config_new_exists() -> Result<()> {
		let (config, _dir) = build(true)?;

		assert_eq!(config.token(), Some("token"));

		Ok(())
	}

	#[test]
	fn test_config_save() -> Result<()> {
		let (mut config, _dir) = build(false)?;

		config.set_token("token2");
		config.save()?;

		let content = fs::read_to_string(&config.path)?;

		assert_eq!(content, "token = \"token2\"\n");

		Ok(())
	}
}
