use std::path::Path;

use anyhow::Result;
use fs_err as fs;
use serde::{de::DeserializeOwned, Serialize};

pub trait JsonContent
where
	Self: DeserializeOwned + Serialize + Default,
{
	fn load(file_path: &Path) -> Result<Self> {
		if file_path.exists() {
			let content = fs::read_to_string(file_path)?;
			if !content.is_empty() {
				let value: Self = serde_json::from_str(&content)?;
				return Ok(value);
			}
		}

		let default = Self::default();
		fs::write(file_path, serde_json::to_string_pretty(&default)?)?;

		Ok(default)
	}

	fn save(&self, file_path: &Path) -> Result<()> {
		let content = serde_json::to_string_pretty(&self)?;
		fs::write(file_path, content)?;

		Ok(())
	}
}

#[cfg(test)]
mod test_utils {
	use std::{fs, path::PathBuf};

	use serde::{Deserialize, Serialize};
	use tempfile::{tempdir, TempDir};

	use super::JsonContent;

	#[derive(Deserialize, Serialize, Default)]
	pub struct Foo {
		pub bar: String,
	}

	impl JsonContent for Foo {}

	pub struct JsonContentMock {
		pub foo: Foo,
		pub tmp_dir: TempDir,
		pub path: PathBuf,
	}

	impl JsonContentMock {
		pub fn new() -> Self {
			let tmp_dir = tempdir().unwrap();
			let path = tmp_dir.path().join("foo.json");
			let foo = Foo::load(&path).unwrap();

			Self {
				foo,
				tmp_dir,
				path,
			}
		}

		pub fn no_content(self) -> Self {
			fs::write(&self.path, "").unwrap();

			let foo = Foo::load(&self.path).unwrap();

			Self {
				foo,
				..self
			}
		}

		pub fn with_content(self) -> Self {
			fs::write(&self.path, "{\n  \"bar\": \"bar\"\n}").unwrap();

			let foo = Foo::load(&self.path).unwrap();

			Self {
				foo,
				..self
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use std::fs;

	use anyhow::Result;

	use super::{test_utils::JsonContentMock, JsonContent};

	#[test]
	fn test_json_load_file() {
		let JsonContentMock {
			tmp_dir: _tmp_dir,
			foo,
			..
		} = JsonContentMock::new().with_content();

		assert_eq!(foo.bar, "bar");
	}

	#[test]
	fn test_json_load_no_content() {
		let JsonContentMock {
			tmp_dir: _tmp_dir,
			foo,
			..
		} = JsonContentMock::new().no_content();

		assert_eq!(foo.bar, "");
	}

	#[test]
	fn test_json_load_file_not_exists() {
		let JsonContentMock {
			tmp_dir: _tmp_dir,
			foo,
			path,
		} = JsonContentMock::new();

		assert_eq!(foo.bar, "");
		assert!(path.exists());
	}

	#[test]
	fn test_json_save() -> Result<()> {
		let JsonContentMock {
			tmp_dir: _tmp_dir,
			mut foo,
			path,
		} = JsonContentMock::new().with_content();

		foo.bar = "bar2".to_string();
		foo.save(&path)?;

		let actual = fs::read_to_string(&path)?;
		assert_eq!(actual, "{\n  \"bar\": \"bar2\"\n}");

		Ok(())
	}
}
