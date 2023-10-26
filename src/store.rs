use std::{
	collections::BTreeMap,
	ops::{Deref, DerefMut},
	path::{Path, PathBuf},
	slice::Iter,
};

use anyhow::Result;
use camino::{Utf8Path, Utf8PathBuf};
use remove_dir_all::remove_dir_all;
use serde::{Deserialize, Serialize};
use tabled::{
	settings::{format::Format, object::Segment, Alignment, Modify, Style},
	Table, Tabled,
};
use term_grid::{Cell, Direction, Filling, Grid, GridOptions};

use crate::toml_content::TomlContent;

struct Changes {
	inner: Vec<String>,
}

impl Changes {
	fn new() -> Self {
		Self { inner: vec![] }
	}

	fn push_add(&mut self, name: &str) {
		use crate::colorize::Colorize;

		self.inner.push(format!("{} {}", "+".success(), name))
	}

	fn push_remove(&mut self, name: &str) {
		use crate::colorize::Colorize;

		self.inner.push(format!("{} {}", "-".error(), name))
	}

	fn iter(&self) -> Iter<'_, String> {
		self.inner.iter()
	}
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
	pub url: String,
	#[tabled(skip)]
	pub local: PathBuf,
	#[tabled(rename = "created at")]
	pub created_at: String,
}

#[cfg(test)]
const TEST_CREATED_AT: &str = "2023-05-19 00:00:00";

impl Scaffold {
	pub fn new<N, U, L>(name: N, url: U, local: L) -> Self
	where
		N: AsRef<str>,
		U: AsRef<str>,
		L: AsRef<Path>,
	{
		#[cfg(not(test))]
		let created_at = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

		#[cfg(test)]
		let created_at = TEST_CREATED_AT.to_string();

		Self {
			name: String::from(name.as_ref()),
			url: String::from(url.as_ref()),
			local: PathBuf::from(local.as_ref()),
			created_at,
		}
	}

	#[cfg(test)]
	pub fn build_toml_str<T>(name: &str, local: T) -> String
	where
		T: AsRef<Path>,
	{
		let local = PathBuf::from(local.as_ref()).display().to_string();

		let quote = if local.contains('\\') { '\'' } else { '"' };

		format!(
			r#"[[scaffold]]
name = "{}"
url = "url"
local = {}{}{}
created_at = "{}"
"#,
			name, quote, local, quote, TEST_CREATED_AT
		)
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
	path: Utf8PathBuf,
	scaffolds: ScaffoldMap,
	changes: Changes,
}

impl Store {
	pub fn new(scafalra_dir: &Utf8Path) -> Result<Self> {
		let path = scafalra_dir.join("store.toml");
		let scaffolds = ScaffoldMap::from(StoreContent::load(&path)?);
		let changes = Changes::new();

		Ok(Self {
			path,
			scaffolds,
			changes,
		})
	}

	pub fn save(&self) -> Result<()> {
		let store_content = StoreContent::from(self.scaffolds.clone());
		store_content.save(&self.path)?;

		self.changes.iter().for_each(|v| {
			println!("{}", v);
		});

		Ok(())
	}

	pub fn add(&mut self, scaffold: Scaffold) {
		let name: &str = scaffold.name.as_ref();

		if self.scaffolds.contains_key(name) {
			self.changes.push_remove(name);
		}

		self.changes.push_add(name);

		self.scaffolds.insert(scaffold.name.clone(), scaffold);
	}

	pub fn remove(&mut self, name: &str) -> Result<()> {
		if let Some(scaffold) = self.scaffolds.get(name) {
			remove_dir_all(&scaffold.local)?;

			self.changes.push_remove(name);

			self.scaffolds.remove(name);
		}

		Ok(())
	}

	pub fn rename(&mut self, name: &str, new_name: &str) {
		if self.scaffolds.contains_key(new_name) {
			println!("`{}` already exists", new_name);
			return;
		}

		match self.scaffolds.remove(name) {
			Some(scaffold) => {
				self.scaffolds.insert(new_name.to_string(), scaffold);
				self.changes.push_remove(name);
				self.changes.push_add(new_name);
			}
			None => {
				println!("No such scaffold `{}`", name);
			}
		};
	}

	pub fn print_grid(&self) -> Option<String> {
		use crate::colorize::Colorize;

		if self.scaffolds.is_empty() {
			return None;
		}

		let mut grid = Grid::new(GridOptions {
			filling: Filling::Spaces(4),
			direction: Direction::LeftToRight,
		});

		self.scaffolds.keys().for_each(|key| {
			grid.add(Cell::from(key.primary()));
		});

		Some(grid.fit_into_columns(6).to_string().trim_end().to_string())
	}

	pub fn print_table(&self) -> Option<String> {
		use crate::colorize::Colorize;

		if self.scaffolds.is_empty() {
			return None;
		}

		let data = Vec::from_iter(self.scaffolds.values().cloned());
		let mut table = Table::new(data);

		let modify = Modify::new(Segment::new(1.., ..1))
			.with(Format::content(|s| s.primary()));

		table
			.with(Style::psql())
			.with(Alignment::left())
			.with(modify);

		Some(table.to_string())
	}

	pub fn get(&self, name: &str) -> Option<&Scaffold> {
		self.scaffolds.get(name)
	}
}

#[cfg(test)]
mod tests {
	use std::{fs, io::Write};

	use anyhow::Result;
	use camino::Utf8Path;
	use pretty_assertions::assert_eq;
	use tempfile::{tempdir, TempDir};

	use super::{Scaffold, ScaffoldMap, Store, StoreContent};

	fn build_scaffold(name: Option<&str>) -> Scaffold {
		Scaffold::new(name.unwrap_or("scaffold"), "url", "local")
	}

	fn build_store(create_file: bool) -> Result<(Store, TempDir)> {
		let temp_dir = tempdir()?;
		let temp_dir_path = Utf8Path::from_path(temp_dir.path()).unwrap();

		if create_file {
			let file_path = temp_dir_path.join("store.toml");
			let mut file = fs::File::create(file_path)?;
			let content = Scaffold::build_toml_str("scaffold", "local");
			file.write_all(content.as_bytes())?;
		}

		let store = Store::new(temp_dir_path)?;

		Ok((store, temp_dir))
	}

	#[test]
	fn test_scaffolds_store_content_transform() {
		let store_content = StoreContent {
			scaffolds: (0..2)
				.map(|v| {
					Scaffold::new(format!("scaffold-{}", v), "url", "local")
				})
				.collect(),
		};

		let scaffold_map = ScaffoldMap::from(store_content);

		assert_eq!(scaffold_map.len(), 2);
		assert!(scaffold_map.contains_key("scaffold-0"));
		assert!(scaffold_map.contains_key("scaffold-1"));

		let store_content = StoreContent::from(scaffold_map);

		assert_eq!(store_content.scaffolds.len(), 2);
		assert_eq!(store_content.scaffolds[0].name, "scaffold-0");
		assert_eq!(store_content.scaffolds[1].name, "scaffold-1");
	}

	#[test]
	fn test_store_new_file_not_exists() -> Result<()> {
		let (store, dir) = build_store(false)?;

		assert_eq!(store.path, dir.path().join("store.toml"));
		assert_eq!(store.scaffolds.len(), 0);
		assert_eq!(store.changes.inner.len(), 0);

		Ok(())
	}

	#[test]
	fn test_store_new_file_exists() -> Result<()> {
		let (store, _dir) = build_store(true)?;

		assert_eq!(store.scaffolds.len(), 1);
		assert!(store.scaffolds.contains_key("scaffold"));

		Ok(())
	}

	#[test]
	fn test_store_save() -> Result<()> {
		let (mut store, dir) = build_store(false)?;
		let scaffold = build_scaffold(None);

		store.add(scaffold);
		store.save()?;

		let content = fs::read_to_string(dir.path().join("store.toml"))?;
		let expected_content = Scaffold::build_toml_str("scaffold", "local");

		assert_eq!(content, expected_content);
		assert_eq!(store.changes.inner.len(), 1);
		assert_eq!(store.changes.inner[0], "+ scaffold");

		Ok(())
	}

	#[test]
	fn test_store_add() -> Result<()> {
		let (mut store, _dir) = build_store(false)?;
		let scaffold = build_scaffold(None);
		store.add(scaffold);

		assert_eq!(store.scaffolds.len(), 1);
		assert!(store.scaffolds.contains_key("scaffold"));
		assert_eq!(store.changes.inner, vec!["+ scaffold"]);

		Ok(())
	}

	#[test]
	fn test_store_add_same() -> Result<()> {
		let (mut store, _dir) = build_store(true)?;
		let scaffold = build_scaffold(None);
		store.add(scaffold);

		assert_eq!(store.scaffolds.len(), 1);
		assert!(store.scaffolds.contains_key("scaffold"));
		assert_eq!(store.changes.inner, vec!["- scaffold", "+ scaffold"]);

		Ok(())
	}

	#[test]
	fn test_store_remove() -> Result<()> {
		let (_, dir) = build_store(true)?;
		let dir_path = Utf8Path::from_path(dir.path()).unwrap();

		let local = dir_path.join("foo");
		fs::create_dir(&local)?;
		let content = Scaffold::build_toml_str("scaffold", &local);
		fs::write(dir_path.join("store.toml"), content)?;
		let mut store = Store::new(dir_path)?;

		assert!(local.exists());
		store.remove("scaffold")?;

		assert!(!local.exists());
		assert_eq!(store.scaffolds.len(), 0);
		assert_eq!(store.changes.inner, vec!["- scaffold"]);

		Ok(())
	}

	#[test]
	fn test_store_remove_not_found() -> Result<()> {
		let (mut store, _dir) = build_store(true)?;
		store.remove("foo")?;

		assert_eq!(store.changes.inner, Vec::<String>::new());

		Ok(())
	}

	#[test]
	fn test_store_rename() -> Result<()> {
		let (mut store, _dir) = build_store(true)?;
		store.rename("scaffold", "foo");

		assert_eq!(store.scaffolds.len(), 1);
		assert!(!store.scaffolds.contains_key("scaffold"));
		assert!(store.scaffolds.contains_key("foo"));
		assert_eq!(store.changes.inner, vec!["- scaffold", "+ foo"]);

		Ok(())
	}

	#[test]
	fn store_rename_exists_or_not_found() -> Result<()> {
		let (mut store, _dir) = build_store(true)?;

		store.rename("scaffold", "scaffold");

		assert_eq!(store.scaffolds.len(), 1);
		assert!(store.scaffolds.contains_key("scaffold"));

		store.rename("foo", "bar");

		assert_eq!(store.scaffolds.len(), 1);
		assert!(store.scaffolds.contains_key("scaffold"));

		Ok(())
	}

	#[test]
	fn test_print_grid() -> Result<()> {
		let (mut store, _dir) = build_store(false)?;

		assert_eq!(store.print_grid(), None);

		for i in 0..7 {
			store.add(build_scaffold(Some(&format!("scaffold-{}", i))));
		}

		assert_eq!(
			store.print_grid().unwrap(),
			"scaffold-0    scaffold-1    scaffold-2    scaffold-3    \
			 scaffold-4    scaffold-5\nscaffold-6"
		);

		Ok(())
	}

	#[test]
	fn test_print_table() -> Result<()> {
		let (mut store, _dir) = build_store(false)?;

		assert_eq!(store.print_table(), None);

		for i in 0..2 {
			store.add(build_scaffold(Some(&format!("scaffold-{}", i))));
		}

		#[rustfmt::skip]
		let expected = " name       | url | created at          \n\
                        ------------+-----+---------------------\n \
                         scaffold-0 | url | 2023-05-19 00:00:00 \n \
                         scaffold-1 | url | 2023-05-19 00:00:00 ";

		assert_eq!(store.print_table().unwrap(), expected);

		Ok(())
	}
}
