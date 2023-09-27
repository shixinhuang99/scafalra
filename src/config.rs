use std::path::{Path, PathBuf};

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::toml_content::TomlContent;

#[derive(Deserialize, Serialize, Default)]
struct ConfigContent {
	token: Option<String>,
}

impl TomlContent for ConfigContent {}

pub struct Config {
	path: PathBuf,
	content: ConfigContent,
}

impl Config {
	pub fn new(scafalra_dir: &Path) -> Result<Self> {
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
	use std::{fs, io::Write, path::PathBuf};

	use anyhow::Result;
	use pretty_assertions::assert_eq;
	use tempfile::{tempdir, TempDir};

	use super::{Config, ConfigContent, TomlContent};

	fn create_temp_file(with_content: bool) -> Result<(TempDir, PathBuf)> {
		let temp_dir = tempdir()?;
		let config_file_path = temp_dir.path().join("config.toml");
		let mut file = fs::File::create(&config_file_path)?;

		if with_content {
			let content = "token = \"token\"\n";
			file.write_all(content.as_bytes())?;
		}

		Ok((temp_dir, config_file_path))
	}

	fn build_config_content(
		with_content: bool,
	) -> Result<(ConfigContent, TempDir, PathBuf)> {
		let (dir, file_path) = create_temp_file(with_content)?;
		let cc = ConfigContent::load(&file_path)?;

		Ok((cc, dir, file_path))
	}

	fn build_config(with_content: bool) -> Result<(Config, TempDir, PathBuf)> {
		let (dir, file_path) = create_temp_file(with_content)?;
		let config = Config::new(dir.path())?;

		Ok((config, dir, file_path))
	}

	#[test]
	fn config_content_new_file_exists_with_content() -> Result<()> {
		let (cc, _dir, _) = build_config_content(true)?;

		assert!(cc.token.is_some());
		assert_eq!(cc.token.unwrap(), "token");

		Ok(())
	}

	#[test]
	fn config_content_new_file_exists_no_content() -> Result<()> {
		let (cc, _dir, _) = build_config_content(false)?;

		assert!(cc.token.is_none());

		Ok(())
	}

	#[test]
	fn config_content_new_file_not_exist() -> Result<()> {
		let dir = tempdir()?;
		let config_file_path = dir.path().join("config.toml");

		let cc = ConfigContent::load(&config_file_path)?;

		assert!(cc.token.is_none());

		Ok(())
	}

	#[test]
	fn config_content_save() -> Result<()> {
		let (mut cc, _dir, file_path) = build_config_content(true)?;

		cc.token = Some("123".to_string());
		cc.save(&file_path)?;

		let content = fs::read_to_string(&file_path)?;
		let expected_content = "token = \"123\"\n";
		assert_eq!(content, expected_content);

		Ok(())
	}

	#[test]
	fn config_new_no_content() -> Result<()> {
		let (config, _dir, file_path) = build_config(false)?;

		assert_eq!(config.path, file_path);
		assert_eq!(config.content.token, None);

		Ok(())
	}

	#[test]
	fn config_new_with_content() -> Result<()> {
		let (config, _dir, file_path) = build_config(true)?;

		assert_eq!(config.path, file_path);
		assert_eq!(config.content.token, Some("token".to_string()));

		Ok(())
	}

	#[test]
	fn config_save_ok() -> Result<()> {
		let (mut config, _dir, _) = build_config(true)?;

		config.set_token("123");

		assert_eq!(config.content.token, Some("123".to_string()));

		Ok(())
	}
}
