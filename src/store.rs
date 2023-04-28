use std::{
    collections::BTreeMap,
    ops::{Deref, DerefMut},
    path::{Path, PathBuf},
};

use anyhow::Result;
use remove_dir_all::remove_dir_all;
use serde::{Deserialize, Serialize};
use tabled::{
    settings::{format::Format, object::Columns, Alignment, Modify, Style},
    Table, Tabled,
};
use term_grid::{Cell, Direction, Filling, Grid, GridOptions};

use crate::utils::{Colorize, TomlContent};

mod log_symbols {
    pub const ADD: &str = "+";
    pub const REMOVE: &str = "-";
    pub const ERROR: &str = "x";
}

#[derive(Deserialize, Serialize, Default)]
struct StoreContent {
    #[serde(rename = "scaffold", default)]
    scaffolds: Vec<Scaffold>,
}

impl TomlContent for StoreContent {}

#[derive(Deserialize, Serialize, Clone, Tabled)]
pub struct Scaffold {
    pub name: String,
    pub input: String,
    pub url: String,
    pub commit: String,

    #[tabled(display_with = "display_local")]
    pub local: PathBuf,
}

fn display_local(local: &Path) -> String {
    local.display().to_string()
}

impl Scaffold {
    pub fn new(
        name: &str,
        input: &str,
        url: &str,
        commit: &str,
        local: PathBuf,
    ) -> Self {
        Self {
            name: name.to_string(),
            input: input.to_string(),
            url: url.to_string(),
            commit: commit.to_string(),
            local,
        }
    }
}

#[derive(Clone)]
struct ScaffoldMap(BTreeMap<String, Scaffold>);

impl From<StoreContent> for ScaffoldMap {
    fn from(value: StoreContent) -> Self {
        Self(BTreeMap::from_iter(
            value
                .scaffolds
                .into_iter()
                .map(|ele| (ele.name.clone(), ele)),
        ))
    }
}

impl From<ScaffoldMap> for StoreContent {
    fn from(value: ScaffoldMap) -> Self {
        StoreContent {
            scaffolds: value.0.into_values().collect(),
        }
    }
}

impl Deref for ScaffoldMap {
    type Target = BTreeMap<String, Scaffold>;

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

        let scaffolds = ScaffoldMap::from(StoreContent::load(&path)?);

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

    pub fn print_grid(&self) -> String {
        let mut grid = Grid::new(GridOptions {
            filling: Filling::Spaces(4),
            direction: Direction::LeftToRight,
        });

        self.scaffolds.keys().for_each(|key| {
            grid.add(Cell::from(key.primary()));
        });

        grid.fit_into_columns(6).to_string()
    }

    pub fn print_table(&self) -> String {
        let data = Vec::from_iter(self.scaffolds.values().cloned());
        let mut table = Table::new(data);

        let modify = Modify::new(Columns::first())
            .with(Format::content(|s| s.primary()));

        table
            .with(Style::psql())
            .with(Alignment::left())
            .with(modify);

        table.to_string()
    }

    pub fn get(&self, name: &str) -> Option<Scaffold> {
        self.scaffolds.get(name).cloned()
    }

    pub fn scaffolds_len(&self) -> usize {
        self.scaffolds.len()
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::BTreeMap, fs, io::Write, path::PathBuf};

    use anyhow::Result;
    use pretty_assertions::assert_eq;
    use tempfile::{tempdir, TempDir};

    use super::{Scaffold, ScaffoldMap, Store, StoreContent, TomlContent};
    use crate::utils::scaffold_toml;

    fn create_temp_file(with_content: bool) -> Result<(TempDir, PathBuf)> {
        let temp_dir = tempdir()?;
        let store_file_path = temp_dir.path().join("store.toml");
        let mut file = fs::File::create(&store_file_path)?;

        if with_content {
            let content = scaffold_toml("scaffold", "input", "local");
            file.write_all(content.as_bytes())?;
        }

        Ok((temp_dir, store_file_path))
    }

    fn build_scaffold() -> Scaffold {
        Scaffold::new(
            "scaffold",
            "input",
            "url",
            "aaaaaaa",
            PathBuf::from("local"),
        )
    }

    fn build_store_content(
        with_content: bool,
    ) -> Result<(StoreContent, TempDir, PathBuf)> {
        let (dir, file_path) = create_temp_file(with_content)?;
        let stc = StoreContent::load(&file_path)?;

        Ok((stc, dir, file_path))
    }

    fn build_store(with_content: bool) -> Result<(Store, TempDir, PathBuf)> {
        let (dir, file_path) = create_temp_file(with_content)?;
        let store = Store::new(dir.path())?;

        Ok((store, dir, file_path))
    }

    #[test]
    fn store_content_new_file_exists_with_content() -> Result<()> {
        let (stc, _dir, _) = build_store_content(true)?;

        assert_eq!(stc.scaffolds.len(), 1);
        let sc = &stc.scaffolds[0];
        assert_eq!(sc.name, "scaffold");
        assert_eq!(sc.input, "input");
        assert_eq!(sc.url, "url");
        assert_eq!(sc.commit, "aaaaaaa");
        assert_eq!(sc.local, PathBuf::from("local"));

        Ok(())
    }

    #[test]
    fn store_content_new_file_exists_no_content() -> Result<()> {
        let (stc, _dir, _) = build_store_content(false)?;

        assert_eq!(stc.scaffolds.len(), 0);

        Ok(())
    }

    #[test]
    fn store_content_new_file_not_exist() -> Result<()> {
        let dir = tempdir()?;
        let store_file_path = dir.path().join("store.toml");

        let stc = StoreContent::load(&store_file_path)?;

        assert_eq!(stc.scaffolds.len(), 0);

        Ok(())
    }

    #[test]
    fn store_content_save() -> Result<()> {
        let (mut stc, _dir, file_path) = build_store_content(true)?;
        stc.scaffolds.push(Scaffold::new(
            "new scaffold",
            "new input",
            "url",
            "aaaaaaa",
            PathBuf::from("new-local"),
        ));
        stc.save(&file_path)?;

        let content = fs::read_to_string(&file_path)?;
        let expected_content = format!(
            "{}\n{}",
            scaffold_toml("scaffold", "input", "local"),
            scaffold_toml("new scaffold", "new input", "new-local")
        );

        assert_eq!(content, expected_content);

        Ok(())
    }

    #[test]
    fn scaffold_map_from_store_content() -> Result<()> {
        let (stc, _dir, _) = build_store_content(true)?;
        let scm = ScaffoldMap::from(stc);

        assert_eq!(scm.len(), 1);
        assert!(scm.contains_key("scaffold"));

        Ok(())
    }

    #[test]
    fn scaffold_map_into_store_content() {
        let mut scm = ScaffoldMap(BTreeMap::new());
        let sc = build_scaffold();
        scm.insert(sc.name.clone(), sc);
        let st: StoreContent = scm.into();

        assert_eq!(st.scaffolds.len(), 1);
        assert_eq!(st.scaffolds[0].name, "scaffold");
    }

    #[test]
    fn store_new() -> Result<()> {
        let (store, _dir, file_path) = build_store(true)?;

        assert_eq!(store.path, file_path);
        assert!(store.scaffolds.contains_key("scaffold"));
        assert_eq!(store.changes.len(), 0);

        Ok(())
    }

    #[test]
    fn store_save() -> Result<()> {
        let (mut store, _dir, file_path) = build_store(false)?;
        let sc = build_scaffold();

        store.add(sc.name.clone(), sc);
        store.save()?;

        let content = fs::read_to_string(file_path)?;
        let expected_content = scaffold_toml("scaffold", "input", "local");

        assert_eq!(content, expected_content);
        assert_eq!(store.changes.len(), 1);
        assert_eq!(store.changes[0], format!("+ scaffold"));

        Ok(())
    }

    #[test]
    fn store_add() -> Result<()> {
        let (mut store, _dir, _) = build_store(false)?;
        let sc = build_scaffold();
        store.add(sc.name.clone(), sc);

        assert_eq!(store.scaffolds.len(), 1);
        assert!(store.scaffolds.contains_key("scaffold"));
        assert_eq!(store.changes, vec!["+ scaffold"]);

        Ok(())
    }

    #[test]
    fn store_add_same() -> Result<()> {
        let (mut store, _dir, _) = build_store(true)?;
        let sc = build_scaffold();
        store.add(sc.name.clone(), sc);

        assert_eq!(store.scaffolds.len(), 1);
        assert!(store.scaffolds.contains_key("scaffold"));
        assert_eq!(store.changes, vec!["- scaffold", "+ scaffold"]);

        Ok(())
    }

    #[test]
    fn store_remove_ok() -> Result<()> {
        let (_, dir, store_file_path) = build_store(true)?;

        let local = dir.path().join("foo");
        fs::create_dir(&local)?;
        let content = scaffold_toml("scaffold", "input", &local);
        fs::write(store_file_path, content)?;
        let mut store = Store::new(dir.path())?;

        assert!(local.exists());
        store.remove("scaffold".to_string())?;

        assert!(!local.exists());
        assert_eq!(store.scaffolds.len(), 0);
        assert_eq!(store.changes, vec!["- scaffold"]);

        Ok(())
    }

    #[test]
    fn store_remove_not_found() -> Result<()> {
        let (mut store, _dir, _) = build_store(true)?;
        store.remove("foo".to_string())?;

        assert_eq!(store.changes, vec!["x foo not found"]);

        Ok(())
    }

    #[test]
    fn store_rename_ok() -> Result<()> {
        let (mut store, _dir, _) = build_store(true)?;
        store.rename("scaffold", "foo");

        assert_eq!(store.scaffolds.len(), 1);
        assert!(!store.scaffolds.contains_key("scaffold"));
        assert!(store.scaffolds.contains_key("foo"));
        assert_eq!(store.changes, vec!["- scaffold", "+ foo"]);

        Ok(())
    }

    #[test]
    fn store_rename_exists_or_not_found() -> Result<()> {
        let (mut store, _dir, _) = build_store(true)?;

        store.rename("scaffold", "scaffold");

        assert_eq!(store.scaffolds.len(), 1);
        assert!(store.scaffolds.contains_key("scaffold"));

        store.rename("foo", "bar");

        assert_eq!(store.scaffolds.len(), 1);
        assert!(store.scaffolds.contains_key("scaffold"));

        Ok(())
    }

    #[test]
    fn print_grid_less_than_six_scaffolds() -> Result<()> {
        let (mut store, _dir, _) = build_store(false)?;

        for i in 0..5 {
            let mut sc = build_scaffold();
            sc.name.push_str(&format!("-{}", i));
            store.add(sc.name.clone(), sc);
        }

        assert_eq!(
            store.print_grid(),
            "scaffold-0    scaffold-1    scaffold-2    scaffold-3    \
             scaffold-4    \n"
        );

        Ok(())
    }

    #[test]
    fn print_grid_equal_six_scaffolds() -> Result<()> {
        let (mut store, _dir, _) = build_store(false)?;

        for i in 0..6 {
            let mut sc = build_scaffold();
            sc.name.push_str(&format!("-{}", i));
            store.add(sc.name.clone(), sc);
        }

        assert_eq!(
            store.print_grid(),
            "scaffold-0    scaffold-1    scaffold-2    scaffold-3    \
             scaffold-4    scaffold-5\n"
        );

        Ok(())
    }

    #[test]
    fn print_grid_more_than_six_scaffolds() -> Result<()> {
        let (mut store, _dir, _) = build_store(false)?;

        for i in 0..7 {
            let mut sc = build_scaffold();
            sc.name.push_str(&format!("-{}", i));
            store.add(sc.name.clone(), sc);
        }

        assert_eq!(
            store.print_grid(),
            "scaffold-0    scaffold-1    scaffold-2    scaffold-3    \
             scaffold-4    scaffold-5\nscaffold-6    \n"
        );

        Ok(())
    }

    #[test]
    fn print_table() -> Result<()> {
        let (mut store, _dir, _) = build_store(false)?;

        for i in 0..2 {
            let mut sc = build_scaffold();
            sc.name.push_str(&format!("-{}", i));
            store.add(sc.name.clone(), sc);
        }

        #[rustfmt::skip]
        let expected = " name       | input | url | commit  | local \n\
                        ------------+-------+-----+---------+-------\n \
                         scaffold-0 | input | url | aaaaaaa | local \n \
                         scaffold-1 | input | url | aaaaaaa | local ";

        assert_eq!(store.print_table(), expected);

        Ok(())
    }
}
