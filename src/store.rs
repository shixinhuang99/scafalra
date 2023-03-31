#![allow(dead_code)]

use std::collections::HashMap;
use std::fs;
use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};

use anyhow::Result;
use remove_dir_all::remove_dir_all;
use serde::{Deserialize, Serialize};

mod log_symbols {
    pub const ADD: &str = "+";
    pub const REMOVE: &str = "-";
    pub const ERROR: &str = "x";
}

#[derive(Deserialize, Serialize, Default)]
struct StoreContent {
    #[serde(rename = "scaffold")]
    scaffolds: Vec<Scaffold>,
}

impl StoreContent {
    pub fn new(file_path: &Path) -> Result<Self> {
        let content: Self = if file_path.exists() {
            toml::from_str(&fs::read_to_string(&file_path)?)?
        } else {
            fs::File::create(file_path)?;
            StoreContent::default()
        };

        Ok(content)
    }

    pub fn save(&self, file_path: &Path) -> Result<()> {
        let content = toml::to_string_pretty(self)?;
        fs::write(file_path, &content)?;

        Ok(())
    }
}

#[derive(Deserialize, Serialize, Clone)]
pub struct Scaffold {
    name: String,
    input: String,
    url: String,
    commit: String,
    local: String,
}

#[derive(Clone)]
struct ScaffoldMap(HashMap<String, Scaffold>);

impl From<StoreContent> for ScaffoldMap {
    fn from(value: StoreContent) -> Self {
        Self(HashMap::from_iter(
            value
                .scaffolds
                .into_iter()
                .map(|ele| (ele.name.clone(), ele)),
        ))
    }
}

impl Into<StoreContent> for ScaffoldMap {
    fn into(self) -> StoreContent {
        StoreContent {
            scaffolds: self.0.into_values().collect(),
        }
    }
}

impl Deref for ScaffoldMap {
    type Target = HashMap<String, Scaffold>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for ScaffoldMap {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

pub struct Store {
    path: PathBuf,
    scaffolds: ScaffoldMap,
    changes: Vec<String>,
}

impl Store {
    pub fn new(scafalra_dir: &Path) -> Result<Self> {
        let path = scafalra_dir.join("store.toml");

        let scaffolds = ScaffoldMap::from(StoreContent::new(&path)?);

        Ok(Self {
            path,
            scaffolds,
            changes: Vec::new(),
        })
    }

    pub fn save(&self) -> Result<()> {
        let st: StoreContent = self.scaffolds.clone().into();
        st.save(&self.path)?;

        self.changes.iter().for_each(|v| {
            println!("{}", v);
        });

        Ok(())
    }

    pub fn add(&mut self, name: String, scaffold: Scaffold) {
        if self.scaffolds.contains_key(&name) {
            self.changes
                .push(format!("{} {}", log_symbols::REMOVE, &name));
        }

        self.changes.push(format!("{} {}", log_symbols::ADD, &name));

        self.scaffolds.insert(name, scaffold);
    }

    pub fn remove(&mut self, name: String) -> Result<()> {
        match self.scaffolds.get(&name) {
            Some(sc) => {
                remove_dir_all(&sc.local)?;

                self.changes
                    .push(format!("{} {}", log_symbols::REMOVE, &name));

                self.scaffolds.remove(&name);
            }
            None => {
                self.changes.push(format!(
                    "{} {} {}",
                    log_symbols::ERROR,
                    &name,
                    "not found"
                ));
            }
        }

        Ok(())
    }

    pub fn rename(&mut self, name: String, new_name: String) {
        if self.scaffolds.get(&new_name).is_some() {
            println!(r#""{}" already exists"#, &new_name);
            return;
        }

        match self.scaffolds.remove(&name) {
            Some(sc) => {
                self.scaffolds.insert(new_name, sc);
            }
            None => {
                println!(r#""{}" not found"#, &name);
            }
        };
    }

    pub fn print_list() {
        todo!();
    }

    pub fn print_list_with_table() {
        todo!();
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::io::Write;
    use std::path::PathBuf;

    use anyhow::Result;
    use pretty_assertions::assert_eq;
    use tempfile::{tempdir, TempDir};

    use super::{Scaffold, Store, StoreContent};

    fn create_temp_file(content: &str) -> Result<(PathBuf, TempDir)> {
        let dir = tempdir()?;
        let file_path = dir.path().join("store.toml");
        let mut file = fs::File::create(&file_path)?;
        file.write_all(content.as_bytes())?;

        Ok((file_path, dir))
    }

    #[test]
    fn store_content_new_when_file_exists() -> Result<()> {
        let file_content = r#"
            [[scaffold]]
            name = "test scaffold"
            input = "test input"
            url = "test url"
            commit = "test commit"
            local = "test local"
        "#;
        let (file_path, _dir) = create_temp_file(file_content)?;

        let st = StoreContent::new(&file_path)?;

        assert_eq!(st.scaffolds.len(), 1);
        let sc = &st.scaffolds[0];
        assert_eq!(sc.name, "test scaffold");
        assert_eq!(sc.input, "test input");
        assert_eq!(sc.url, "test url");
        assert_eq!(sc.commit, "test commit");
        assert_eq!(sc.local, "test local");

        Ok(())
    }

    #[test]
    fn store_content_new_when_file_does_not_exist() -> Result<()> {
        let dir = tempdir()?;
        let file_path = dir.path().join("test.toml");

        let st = StoreContent::new(&file_path)?;

        assert_eq!(st.scaffolds.len(), 0);

        Ok(())
    }

    #[test]
    fn store_content_save() -> Result<()> {
        let file_content = r#"
            [[scaffold]]
            name = "test scaffold"
            input = "test input"
            url = "test url"
            commit = "test commit"
            local = "test local"
        "#;
        let (file_path, _dir) = create_temp_file(file_content)?;

        let mut st = StoreContent::new(&file_path)?;
        st.scaffolds.push(Scaffold {
            name: String::from("new scaffold"),
            input: String::from("new input"),
            url: String::from("new url"),
            commit: String::from("new commit"),
            local: String::from("new local"),
        });
        st.save(&file_path)?;

        let content = fs::read_to_string(&file_path)?;
        let expected_content = r#"[[scaffold]]
name = "test scaffold"
input = "test input"
url = "test url"
commit = "test commit"
local = "test local"

[[scaffold]]
name = "new scaffold"
input = "new input"
url = "new url"
commit = "new commit"
local = "new local"
"#;
        assert_eq!(content, expected_content);

        Ok(())
    }

    #[ignore]
    #[test]
    fn scaffold_map_from_store_content() {
        todo!();
    }

    #[ignore]
    #[test]
    fn scaffold_map_into_store_content() {
        todo!();
    }

    #[test]
    fn store_new() -> Result<()> {
        let file_content = r#"
            [[scaffold]]
            name = "test scaffold"
            input = "test input"
            url = "test url"
            commit = "test commit"
            local = "test local"
        "#;
        let (file_path, dir) = create_temp_file(file_content)?;

        let store = Store::new(dir.path())?;
        assert_eq!(store.path, file_path);
        assert!(store.scaffolds.contains_key("test scaffold"));
        assert_eq!(store.changes.len(), 0);

        Ok(())
    }

    #[test]
    fn store_add() -> Result<()> {
        let dir = tempdir()?;

        let mut store = Store::new(dir.path())?;

        let name = "test".to_string();
        let default_sc = Scaffold {
            name: "test".to_string(),
            input: "input".to_string(),
            url: "url".to_string(),
            commit: "commit".to_string(),
            local: "local".to_string(),
        };
        store.add(name.clone(), default_sc.clone());

        assert_eq!(store.scaffolds.len(), 1);
        assert!(store.scaffolds.contains_key(&name));
        assert_eq!(store.changes, vec![format!("+ {}", &name)]);

        Ok(())
    }

    #[test]
    fn store_add_same() -> Result<()> {
        let file_content = r#"
            [[scaffold]]
            name = "test"
            input = "test"
            url = "test"
            commit = "test"
            local = "test"
        "#;
        let (_, dir) = create_temp_file(file_content)?;

        let mut store = Store::new(dir.path())?;

        let name = "test".to_string();
        let default_sc = Scaffold {
            name: "test".to_string(),
            input: "input".to_string(),
            url: "url".to_string(),
            commit: "commit".to_string(),
            local: "local".to_string(),
        };
        store.add(name.clone(), default_sc.clone());

        assert_eq!(store.scaffolds.len(), 1);
        assert!(store.scaffolds.contains_key(&name));
        assert_eq!(
            store.changes,
            vec![format!("- {}", &name), format!("+ {}", &name)]
        );

        Ok(())
    }
}
