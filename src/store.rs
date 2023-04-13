#![allow(dead_code)]

use std::collections::HashMap;
use std::fs;
use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};

use anyhow::Result;
use remove_dir_all::remove_dir_all;
use serde::{Deserialize, Serialize};

use crate::utils::Colorize;

mod log_symbols {
    pub const ADD: &str = "+";
    pub const REMOVE: &str = "-";
    pub const ERROR: &str = "x";
}

#[derive(Deserialize, Serialize)]
struct StoreContent {
    #[serde(rename = "scaffold", default)]
    scaffolds: Vec<Scaffold>,
}

impl StoreContent {
    pub fn new(file_path: &Path) -> Result<Self> {
        let content: Self = if file_path.exists() {
            toml::from_str(&fs::read_to_string(&file_path)?)?
        } else {
            fs::File::create(file_path)?;
            StoreContent {
                scaffolds: Vec::new(),
            }
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
            self.changes.push(format!(
                "{} {}",
                log_symbols::REMOVE.success(),
                &name
            ));
        }

        self.changes
            .push(format!("{} {}", log_symbols::ADD.success(), &name));

        self.scaffolds.insert(name, scaffold);
    }

    pub fn remove(&mut self, name: String) -> Result<()> {
        match self.scaffolds.get(&name) {
            Some(sc) => {
                remove_dir_all(Path::new(&sc.local))?;

                self.changes.push(format!(
                    "{} {}",
                    log_symbols::REMOVE.success(),
                    &name
                ));

                self.scaffolds.remove(&name);
            }
            None => {
                self.changes.push(format!(
                    "{} {} {}",
                    log_symbols::ERROR.error(),
                    &name,
                    "not found".error()
                ));
            }
        }

        Ok(())
    }

    pub fn rename(&mut self, name: &str, new_name: &str) {
        if self.scaffolds.contains_key(new_name) {
            println!(r#""{}" already exists"#, new_name);
            return;
        }

        match self.scaffolds.remove(name) {
            Some(sc) => {
                self.scaffolds.insert(new_name.to_string(), sc);
                self.changes.push(format!(
                    "{} {}",
                    log_symbols::REMOVE.success(),
                    name
                ));
                self.changes.push(format!(
                    "{} {}",
                    log_symbols::ADD.success(),
                    new_name
                ));
            }
            None => {
                println!(r#""{}" not found"#, name);
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
    use std::collections::HashMap;
    use std::fs;
    use std::io::Write;
    use std::path::PathBuf;

    use anyhow::Result;
    use pretty_assertions::assert_eq;
    use tempfile::{tempdir, TempDir};

    use super::{Scaffold, ScaffoldMap, Store, StoreContent};

    fn create_temp_file(
        with_content: bool,
    ) -> Result<(TempDir, PathBuf, PathBuf)> {
        let dir = tempdir()?;
        let store_file_path = dir.path().join("store.toml");
        let mut file = fs::File::create(&store_file_path)?;
        let local = dir.path().join("scaffold");
        fs::create_dir(&local)?;

        if with_content {
            let content = format!(
                r#"[[scaffold]]
name = "scaffold"
input = "input"
url = "url"
commit = "commit"
local = "{}"
"#,
                local.display()
            );
            file.write_all(content.as_bytes())?;
        }

        Ok((dir, store_file_path, local))
    }

    fn build_scaffold() -> Scaffold {
        Scaffold {
            name: "scaffold".to_string(),
            input: "input".to_string(),
            url: "url".to_string(),
            commit: "commit".to_string(),
            local: "local".to_string(),
        }
    }

    fn build_store_content(
        with_content: bool,
    ) -> Result<(StoreContent, TempDir, PathBuf, PathBuf)> {
        let (dir, file_path, local) = create_temp_file(with_content)?;
        let stc = StoreContent::new(&file_path)?;

        Ok((stc, dir, file_path, local))
    }

    fn build_store(
        with_content: bool,
    ) -> Result<(Store, TempDir, PathBuf, PathBuf)> {
        let (dir, file_path, local) = create_temp_file(with_content)?;
        let store = Store::new(dir.path())?;

        Ok((store, dir, file_path, local))
    }

    #[test]
    fn store_content_new_when_file_exists() -> Result<()> {
        let (stc, _dir, _, local) = build_store_content(true)?;

        assert_eq!(stc.scaffolds.len(), 1);
        let sc = &stc.scaffolds[0];
        assert_eq!(sc.name, "scaffold");
        assert_eq!(sc.input, "input");
        assert_eq!(sc.url, "url");
        assert_eq!(sc.commit, "commit");
        assert_eq!(sc.local, local.display().to_string());

        Ok(())
    }

    #[test]
    fn store_content_new_when_file_does_not_exist() -> Result<()> {
        let (stc, _dir, _, _) = build_store_content(false)?;

        assert_eq!(stc.scaffolds.len(), 0);

        Ok(())
    }

    #[test]
    fn store_content_save() -> Result<()> {
        let (mut stc, _dir, file_path, local) = build_store_content(true)?;
        stc.scaffolds.push(Scaffold {
            name: String::from("new scaffold"),
            input: String::from("new input"),
            url: String::from("new url"),
            commit: String::from("new commit"),
            local: String::from("new local"),
        });
        stc.save(&file_path)?;

        let content = fs::read_to_string(&file_path)?;
        let expected_content = format!(
            r#"[[scaffold]]
name = "scaffold"
input = "input"
url = "url"
commit = "commit"
local = "{}"

[[scaffold]]
name = "new scaffold"
input = "new input"
url = "new url"
commit = "new commit"
local = "new local"
"#,
            local.display().to_string()
        );
        assert_eq!(content, expected_content);

        Ok(())
    }

    #[test]
    fn scaffold_map_from_store_content() -> Result<()> {
        let (stc, _dir, _, _) = build_store_content(true)?;
        let scm = ScaffoldMap::from(stc);

        assert_eq!(scm.len(), 1);
        assert!(scm.contains_key("scaffold"));

        Ok(())
    }

    #[test]
    fn scaffold_map_into_store_content() {
        let mut scm = ScaffoldMap(HashMap::new());
        let sc = build_scaffold();
        scm.insert(sc.name.clone(), sc);
        let st: StoreContent = scm.into();

        assert_eq!(st.scaffolds.len(), 1);
        assert_eq!(st.scaffolds[0].name, "scaffold");
    }

    #[test]
    fn store_new() -> Result<()> {
        let (store, _dir, file_path, _) = build_store(true)?;

        assert_eq!(store.path, file_path);
        assert!(store.scaffolds.contains_key("scaffold"));
        assert_eq!(store.changes.len(), 0);

        Ok(())
    }

    #[test]
    fn store_save() -> Result<()> {
        let (mut store, _dir, file_path, _) = build_store(false)?;
        let sc = build_scaffold();

        store.add(sc.name.clone(), sc);
        store.save()?;

        let content = fs::read_to_string(&file_path)?;
        let expected_content = r#"[[scaffold]]
name = "scaffold"
input = "input"
url = "url"
commit = "commit"
local = "local"
"#;
        assert_eq!(content, expected_content);
        assert_eq!(store.changes.len(), 1);
        assert_eq!(store.changes[0], format!("+ scaffold"));

        Ok(())
    }

    #[test]
    fn store_add() -> Result<()> {
        let (mut store, _dir, _, _) = build_store(false)?;
        let sc = build_scaffold();
        store.add(sc.name.clone(), sc);

        assert_eq!(store.scaffolds.len(), 1);
        assert!(store.scaffolds.contains_key("scaffold"));
        assert_eq!(store.changes, vec!["+ scaffold"]);

        Ok(())
    }

    #[test]
    fn store_add_same() -> Result<()> {
        let (mut store, _dir, _, _) = build_store(true)?;
        let sc = build_scaffold();
        store.add(sc.name.clone(), sc);

        assert_eq!(store.scaffolds.len(), 1);
        assert!(store.scaffolds.contains_key("scaffold"));
        assert_eq!(store.changes, vec!["- scaffold", "+ scaffold"]);

        Ok(())
    }

    #[test]
    fn store_remove_ok() -> Result<()> {
        let (mut store, _dir, _, local) = build_store(true)?;

        assert!(local.exists());
        store.remove("scaffold".to_string())?;

        assert!(!local.exists());
        assert_eq!(store.scaffolds.len(), 0);
        assert_eq!(store.changes, vec!["- scaffold"]);

        Ok(())
    }

    #[test]
    fn store_remove_not_found() -> Result<()> {
        let (mut store, _dir, _, _) = build_store(true)?;
        store.remove("foo".to_string())?;

        assert_eq!(store.changes, vec!["x foo not found"]);

        Ok(())
    }

    #[test]
    fn store_rename_ok() -> Result<()> {
        let (mut store, _dir, _, _) = build_store(true)?;
        store.rename("scaffold", "foo");

        assert_eq!(store.scaffolds.len(), 1);
        assert!(!store.scaffolds.contains_key("scaffold"));
        assert!(store.scaffolds.contains_key("foo"));
        assert_eq!(store.changes, vec!["- scaffold", "+ foo"]);

        Ok(())
    }

    #[test]
    fn store_rename_exists_or_not_found() -> Result<()> {
        let (mut store, _dir, _, _) = build_store(true)?;

        store.rename("scaffold", "scaffold");

        assert_eq!(store.scaffolds.len(), 1);
        assert!(store.scaffolds.contains_key("scaffold"));

        store.rename("foo", "bar");

        assert_eq!(store.scaffolds.len(), 1);
        assert!(store.scaffolds.contains_key("scaffold"));

        Ok(())
    }
}
