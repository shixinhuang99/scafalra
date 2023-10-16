use std::{fs, path::Path};

use anyhow::{Context, Result};
use serde::{de::DeserializeOwned, Serialize};

use crate::error::ScafalraError;

fn load_or_default<T>(file: &Path, defalut: T) -> Result<T>
where
	T: TomlContent,
{
	let content: T = {
		if file.exists() {
			toml::from_str(&fs::read_to_string(file)?)?
		} else {
			fs::File::create(file)?;
			defalut
		}
	};

	Ok(content)
}

fn save_content<T>(file: &Path, that: &T) -> Result<()>
where
	T: TomlContent,
{
	let content = toml::to_string_pretty(that)?;
	fs::write(file, content)?;

	Ok(())
}

pub trait TomlContent: DeserializeOwned + Serialize + Default {
	fn load(file: &Path) -> Result<Self> {
		load_or_default(file, Self::default())
			.context(ScafalraError::IOError(file.display().to_string()))
	}

	fn save(&self, file: &Path) -> Result<()> {
		save_content(file, self)
			.context(ScafalraError::IOError(file.display().to_string()))
	}
}

#[cfg(test)]
mod tests {
	use std::{fs, io::Write, path::PathBuf};

	use anyhow::Result;
	use serde::{Deserialize, Serialize};
	use tempfile::{tempdir, TempDir};

	use super::TomlContent;

	#[derive(Deserialize, Serialize, Default)]
	struct Foo {
		bar: String,
	}

	impl TomlContent for Foo {}

	impl Foo {
		fn build(create_file: bool) -> Result<(Self, TempDir, PathBuf)> {
			let temp_dir = tempdir()?;
			let file_path = temp_dir.path().join("foo.toml");

			if create_file {
				let mut file = fs::File::create(&file_path)?;
				file.write_all(b"bar = \"bar\"\n")?;
			}

			let foo = Foo::load(&file_path)?;

			Ok((foo, temp_dir, file_path))
		}
	}

	#[test]
	fn test_load_file_exists() {
		let (foo, _dir, _file_path) = Foo::build(true).unwrap();

		assert_eq!(foo.bar, "bar");
	}

	#[test]
	fn test_load_file_not_exists() {
		let (foo, _dir, file_path) = Foo::build(false).unwrap();

		assert_eq!(foo.bar, "");
		assert!(file_path.exists());
	}

	#[test]
	fn test_save() {
		let (mut foo, _dir, file_path) = Foo::build(true).unwrap();

		foo.bar = "bar2".to_string();
		foo.save(&file_path).unwrap();

		let content = fs::read_to_string(&file_path).unwrap();
		assert_eq!(content, "bar = \"bar2\"\n")
	}
}
