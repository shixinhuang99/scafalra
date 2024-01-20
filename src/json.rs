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

pub trait ToJson
where
	Self: Serialize,
{
	fn to_json(&self) -> String {
		serde_json::to_string(self).unwrap()
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
		pub tmpdir: TempDir,
		pub path: PathBuf,
	}

	impl JsonContentMock {
		pub fn new() -> Self {
			let tmpdir = tempdir().unwrap();
			let file_path = tmpdir.path().join("foo.json");
			let foo = Foo::load(&file_path).unwrap();

			Self {
				foo,
				tmpdir,
				path: file_path,
			}
		}

		pub fn with_empty_content(self) -> Self {
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
		let json_content_mock = JsonContentMock::new().with_content();

		assert_eq!(json_content_mock.foo.bar, "bar");
	}

	#[test]
	fn test_json_load_empty_content() {
		let json_content_mock = JsonContentMock::new().with_empty_content();

		assert_eq!(json_content_mock.foo.bar, "");
	}

	#[test]
	fn test_json_load_file_not_exists() {
		let json_content_mock = JsonContentMock::new();

		assert_eq!(json_content_mock.foo.bar, "");
		assert!(json_content_mock.path.exists());
	}

	#[test]
	fn test_json_save() -> Result<()> {
		let mut json_content_mock = JsonContentMock::new().with_content();

		json_content_mock.foo.bar = "bar2".to_string();
		json_content_mock.foo.save(&json_content_mock.path)?;

		let actual = fs::read_to_string(&json_content_mock.path)?;
		assert_eq!(actual, "{\n  \"bar\": \"bar2\"\n}");

		Ok(())
	}
}
