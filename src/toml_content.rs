use anyhow::Result;
use camino::Utf8Path;
use fs_err as fs;
use serde::{de::DeserializeOwned, Serialize};

pub trait TomlContent: DeserializeOwned + Serialize + Default {
	fn load(file: &Utf8Path) -> Result<Self> {
		let content: Self = {
			if file.exists() {
				toml::from_str(&fs::read_to_string(file)?)?
			} else {
				fs::File::create(file)?;
				Self::default()
			}
		};

		Ok(content)
	}

	fn save(&self, file: &Utf8Path) -> Result<()> {
		let content = toml::to_string_pretty(self)?;
		fs::write(file, content)?;

		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use std::{fs, io::Write};

	use anyhow::Result;
	use camino::{Utf8Path, Utf8PathBuf};
	use serde::{Deserialize, Serialize};
	use tempfile::{tempdir, TempDir};

	use super::TomlContent;

	#[derive(Deserialize, Serialize, Default)]
	struct Foo {
		bar: String,
	}

	impl TomlContent for Foo {}

	impl Foo {
		fn build(create_file: bool) -> Result<(Self, TempDir, Utf8PathBuf)> {
			let temp_dir = tempdir()?;
			let file_path = Utf8Path::from_path(temp_dir.path())
				.unwrap()
				.join("foo.toml");

			if create_file {
				let mut file = fs::File::create(&file_path)?;
				file.write_all(b"bar = \"bar\"\n")?;
			}

			let foo = Foo::load(&file_path)?;

			Ok((foo, temp_dir, file_path))
		}
	}

	#[test]
	fn test_load_file_exists() -> Result<()> {
		let (foo, _dir, _file_path) = Foo::build(true)?;

		assert_eq!(foo.bar, "bar");

		Ok(())
	}

	#[test]
	fn test_load_file_not_exists() -> Result<()> {
		let (foo, _dir, file_path) = Foo::build(false)?;

		assert_eq!(foo.bar, "");
		assert!(file_path.exists());

		Ok(())
	}

	#[test]
	fn test_save() -> Result<()> {
		let (mut foo, _dir, file_path) = Foo::build(true)?;

		foo.bar = "bar2".to_string();
		foo.save(&file_path)?;

		let content = fs::read_to_string(&file_path)?;
		assert_eq!(content, "bar = \"bar2\"\n");

		Ok(())
	}
}
