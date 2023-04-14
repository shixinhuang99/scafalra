use std::{fs, path::Path};

use anyhow::Result;
use serde::{de::DeserializeOwned, Serialize};

pub trait TomlContent: DeserializeOwned + Serialize + Default {
    fn new(file_path: &Path) -> Result<Self> {
        let content: Self = if file_path.exists() {
            toml::from_str(&fs::read_to_string(&file_path)?)?
        } else {
            fs::File::create(file_path)?;
            Self::default()
        };

        Ok(content)
    }

    fn save(&self, file_path: &Path) -> Result<()> {
        let content = toml::to_string_pretty(self)?;
        fs::write(file_path, &content)?;

        Ok(())
    }
}
