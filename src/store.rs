#![allow(dead_code)]

use std::collections::{HashMap, HashSet};
use std::fs;
use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};

use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Default)]
struct StoreContent {
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
    changes: HashSet<String>,
}

impl Store {
    pub fn new(scafalra_dir: &Path) -> Result<Self> {
        let path = scafalra_dir.join("store.toml");

        let scaffolds = ScaffoldMap::from(StoreContent::new(&path)?);

        Ok(Self {
            path,
            scaffolds,
            changes: HashSet::new(),
        })
    }

    pub fn save(&self) -> Result<()> {
        let st: StoreContent = self.scaffolds.clone().into();
        st.save(&self.path)?;

        Ok(())
    }

    pub fn add(&mut self, name: String, scaffold: Scaffold) {
        self.scaffolds.insert(name, scaffold);
    }
}
