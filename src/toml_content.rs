use std::{fs, path::Path};

use anyhow::{Context, Result};
use serde::{de::DeserializeOwned, Serialize};

use crate::error::ScafalraError;

fn load<T>(file: &Path, defalut: T) -> Result<T>
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

fn save<T>(file: &Path, that: &T) -> Result<()>
where
	T: TomlContent,
{
	let content = toml::to_string_pretty(that)?;
	fs::write(file, content)?;

	Ok(())
}

pub trait TomlContent: DeserializeOwned + Serialize + Default {
	fn load(file: &Path) -> Result<Self> {
		load(file, Self::default())
			.with_context(|| ScafalraError::FileReadOrWrite(file.to_path_buf()))
	}

	fn save(&self, file: &Path) -> Result<()> {
		save(file, self)
			.with_context(|| ScafalraError::FileReadOrWrite(file.to_path_buf()))
	}
}
