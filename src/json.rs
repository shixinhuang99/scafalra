use anyhow::Result;
use camino::Utf8Path;
use fs_err as fs;
use serde::{de::DeserializeOwned, Serialize};

pub trait JsonContent
where
	Self: DeserializeOwned + Serialize + Default,
{
	fn load(file_path: &Utf8Path) -> Result<Self> {
		let write_and_return_default = || -> Result<Self> {
			let default = Self::default();
			fs::write(file_path, serde_json::to_string_pretty(&default)?)?;
			Ok(default)
		};
		if file_path.exists() {
			let content = fs::read_to_string(file_path)?;
			if content.is_empty() {
				return write_and_return_default();
			}
			let value: Self = serde_json::from_str(&content)?;
			return Ok(value);
		}
		write_and_return_default()
	}

	fn save(&self, file_path: &Utf8Path) -> Result<()> {
		let content = serde_json::to_string_pretty(&self)?;
		fs::write(file_path, content)?;

		Ok(())
	}
}

pub trait ToJson
where
	Self: Serialize,
{
	fn to_json(&self) -> String {
		serde_json::to_string(self).unwrap()
	}
}

#[cfg(test)]
mod tests {
	use std::fs;

	use anyhow::Result;
	use camino::Utf8PathBuf;
	use serde::{Deserialize, Serialize};
	use tempfile::{tempdir, TempDir};

	use super::JsonContent;
	use crate::utf8_path::Utf8PathBufExt;

	#[derive(Deserialize, Serialize, Default)]
	struct Foo {
		bar: String,
	}

	impl JsonContent for Foo {}

	fn mock_foo(
		create_file: bool,
		init_content: bool,
	) -> Result<(Foo, TempDir, Utf8PathBuf)> {
		let temp_dir = tempdir()?;
		let file_path =
			temp_dir.path().join("foo.json").into_utf8_path_buf()?;

		if create_file {
			if init_content {
				fs::write(&file_path, "{\n  \"bar\": \"bar\"\n}")?;
			} else {
				fs::write(&file_path, "")?;
			}
		}

		let foo = Foo::load(&file_path)?;

		Ok((foo, temp_dir, file_path))
	}

	#[test]
	fn test_load_file_exists() -> Result<()> {
		let (foo, _dir, _file_path) = mock_foo(true, true)?;

		assert_eq!(foo.bar, "bar");

		Ok(())
	}

	#[test]
	fn test_load_file_exists_empty_content() -> Result<()> {
		let (foo, _tmpdir, _file_path) = mock_foo(true, false)?;

		assert_eq!(foo.bar, "");

		Ok(())
	}

	#[test]
	fn test_load_file_not_exists() -> Result<()> {
		let (foo, _dir, file_path) = mock_foo(false, false)?;

		assert_eq!(foo.bar, "");
		assert!(file_path.exists());

		Ok(())
	}

	#[test]
	fn test_save() -> Result<()> {
		let (mut foo, _dir, file_path) = mock_foo(true, true)?;

		foo.bar = "bar2".to_string();
		foo.save(&file_path)?;

		let content = fs::read_to_string(&file_path)?;
		assert_eq!(content, "{\n  \"bar\": \"bar2\"\n}");

		Ok(())
	}
}
