use std::{
	collections::BTreeMap,
	ops::{Deref, DerefMut},
	path::{Path, PathBuf},
	slice::Iter,
};

use anyhow::Result;
use remove_dir_all::remove_dir_all;
use serde::{Deserialize, Serialize};
use tabled::{
	settings::{format::Format, object::Segment, Alignment, Modify, Style},
	Table, Tabled,
};
use term_grid::{Cell, Direction, Filling, Grid, GridOptions};

use crate::json::JsonContent;

#[derive(Deserialize, Serialize, Clone, Tabled)]
pub struct Scaffold {
	pub name: String,
	pub url: String,
	#[tabled(skip)]
	pub path: PathBuf,
	#[tabled(rename = "created at")]
	pub created_at: String,
}

impl Scaffold {
	pub fn new<N, U, L>(name: N, url: U, path: L) -> Self
	where
		N: AsRef<str>,
		U: AsRef<str>,
		L: AsRef<Path>,
	{
		let created_at = if cfg!(test) {
			"2023-05-19 00:00:00".to_string()
		} else {
			chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
		};

		Self {
			name: String::from(name.as_ref()),
			url: String::from(url.as_ref()),
			path: PathBuf::from(path.as_ref()),
			created_at,
		}
	}
}

#[derive(Deserialize, Serialize, Default)]
struct ScaffoldMap(BTreeMap<String, Scaffold>);

impl JsonContent for ScaffoldMap {}

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

struct Changes {
	inner: Vec<String>,
}

impl Changes {
	fn new() -> Self {
		Self { inner: Vec::new() }
	}

	fn push_add(&mut self, name: &str) -> &mut Self {
		use crate::colorize::Colorize;

		self.inner.push(format!("{} {}", "+".green(), name));

		self
	}

	fn push_remove(&mut self, name: &str) -> &mut Self {
		use crate::colorize::Colorize;

		self.inner.push(format!("{} {}", "-".red(), name));

		self
	}

	fn iter(&self) -> Iter<'_, String> {
		self.inner.iter()
	}
}

pub struct Store {
	pub path: PathBuf,
	scaffold_map: ScaffoldMap,
	changes: Changes,
}

impl Store {
	pub const FILE_NAME: &'static str = "store.json";

	pub fn new(scafalra_dir: &Path) -> Result<Self> {
		let path = scafalra_dir.join(Self::FILE_NAME);
		let scaffold_map = ScaffoldMap::load(&path)?;
		let changes = Changes::new();

		Ok(Self {
			path,
			scaffold_map,
			changes,
		})
	}

	pub fn save(&self) -> Result<()> {
		self.scaffold_map.save(&self.path)?;

		self.changes.iter().for_each(|v| {
			println!("{}", v);
		});

		Ok(())
	}

	pub fn add(&mut self, scaffold: Scaffold) {
		let name = &scaffold.name;

		if self.scaffold_map.contains_key(name) {
			self.changes.push_remove(name);
		}

		self.changes.push_add(name);
		self.scaffold_map.insert(name.to_string(), scaffold);
	}

	pub fn remove(&mut self, name: &str) -> Result<()> {
		if let Some(scaffold) = self.scaffold_map.get(name) {
			remove_dir_all(&scaffold.path)?;

			self.changes.push_remove(name);
			self.scaffold_map.remove(name);
		}

		Ok(())
	}

	pub fn rename(&mut self, name: &str, new_name: &str) {
		if self.scaffold_map.contains_key(new_name) {
			println!("`{}` already exists", new_name);
			return;
		}

		match self.scaffold_map.remove(name) {
			Some(scaffold) => {
				self.scaffold_map.insert(new_name.to_string(), scaffold);
				self.changes.push_remove(name).push_add(new_name);
			}
			None => {
				println!("No such scaffold `{}`", name);
			}
		};
	}

	pub fn print_grid(&self) -> Option<String> {
		use crate::colorize::Colorize;

		if self.scaffold_map.is_empty() {
			return None;
		}

		let mut grid = Grid::new(GridOptions {
			filling: Filling::Spaces(4),
			direction: Direction::LeftToRight,
		});

		self.scaffold_map.keys().for_each(|key| {
			grid.add(Cell::from(key.blue()));
		});

		Some(grid.fit_into_columns(6).to_string().trim_end().to_string())
	}

	pub fn print_table(&self) -> Option<String> {
		use crate::colorize::Colorize;

		if self.scaffold_map.is_empty() {
			return None;
		}

		let data = Vec::from_iter(self.scaffold_map.values().cloned());
		let mut table = Table::new(data);

		let modify = Modify::new(Segment::new(1.., ..1))
			.with(Format::content(|s| s.blue()));

		table
			.with(Style::psql())
			.with(Alignment::left())
			.with(modify);

		Some(table.to_string())
	}

	pub fn get(&self, name: &str) -> Option<&Scaffold> {
		self.scaffold_map.get(name)
	}
}

#[cfg(test)]
pub fn mock_store_json<T, const N: usize>(data: [(&str, T); N]) -> String
where
	T: AsRef<Path>,
{
	let content =
		ScaffoldMap(BTreeMap::from_iter(data.into_iter().map(|ele| {
			(ele.0.to_string(), Scaffold::new(ele.0, "url", ele.1))
		})));

	serde_json::to_string_pretty(&content).unwrap()
}

#[cfg(test)]
mod tests {
	use std::{fs, path::PathBuf};

	use anyhow::Result;
	use tempfile::{tempdir, TempDir};

	use super::{mock_store_json, Scaffold, Store};

	fn mock_scaffold(name: &str) -> Scaffold {
		Scaffold::new(name, "url", "path")
	}

	fn mock_store(init_content: bool) -> Result<(Store, TempDir, PathBuf)> {
		let temp_dir = tempdir()?;
		let temp_dir_path = temp_dir.path();
		let foo_path = temp_dir_path.join("foo");

		if init_content {
			let store_file = temp_dir_path.join(Store::FILE_NAME);
			fs::create_dir(&foo_path)?;
			let content = mock_store_json([("foo", &foo_path)]);
			fs::write(store_file, content)?;
		}

		let store = Store::new(temp_dir_path)?;

		Ok((store, temp_dir, foo_path))
	}

	#[test]
	fn test_store_new_file_not_exists() -> Result<()> {
		let (store, _dir, _) = mock_store(false)?;

		assert_eq!(store.scaffold_map.len(), 0);
		assert_eq!(store.changes.inner.len(), 0);

		Ok(())
	}

	#[test]
	fn test_store_new_file_exists() -> Result<()> {
		let (store, _dir, _) = mock_store(true)?;

		assert_eq!(store.scaffold_map.len(), 1);
		assert!(store.scaffold_map.contains_key("foo"));

		Ok(())
	}

	#[test]
	fn test_store_save() -> Result<()> {
		let (store, _dir, foo_path) = mock_store(true)?;

		store.save()?;

		let content = fs::read_to_string(store.path)?;
		let expected_content = mock_store_json([("foo", &foo_path)]);

		assert_eq!(content, expected_content);

		Ok(())
	}

	#[test]
	fn test_store_add() -> Result<()> {
		let (mut store, _dir, _) = mock_store(false)?;

		store.add(mock_scaffold("foo"));

		assert_eq!(store.scaffold_map.len(), 1);
		assert!(store.scaffold_map.contains_key("foo"));
		assert_eq!(store.changes.inner, vec!["+ foo"]);

		Ok(())
	}

	#[test]
	fn test_store_add_same() -> Result<()> {
		let (mut store, _dir, _) = mock_store(true)?;

		store.add(mock_scaffold("foo"));

		assert_eq!(store.scaffold_map.len(), 1);
		assert!(store.scaffold_map.contains_key("foo"));
		assert_eq!(store.changes.inner, vec!["- foo", "+ foo"]);

		Ok(())
	}

	#[test]
	fn test_store_remove() -> Result<()> {
		let (mut store, _dir, foo_path) = mock_store(true)?;

		store.remove("foo")?;

		assert!(!foo_path.exists());
		assert_eq!(store.scaffold_map.len(), 0);
		assert_eq!(store.changes.inner, vec!["- foo"]);

		Ok(())
	}

	#[test]
	fn test_store_remove_not_found() -> Result<()> {
		let (mut store, _dir, _) = mock_store(true)?;

		store.remove("bar")?;

		assert_eq!(store.changes.inner.len(), 0);

		Ok(())
	}

	#[test]
	fn test_store_rename() -> Result<()> {
		let (mut store, _dir, _) = mock_store(true)?;
		store.rename("foo", "bar");

		assert_eq!(store.scaffold_map.len(), 1);
		assert!(!store.scaffold_map.contains_key("foo"));
		assert!(store.scaffold_map.contains_key("bar"));
		assert_eq!(store.changes.inner, vec!["- foo", "+ bar"]);

		Ok(())
	}

	#[test]
	fn store_rename_exists_or_not_found() -> Result<()> {
		let (mut store, _dir, _) = mock_store(true)?;

		store.rename("foo", "foo");

		assert_eq!(store.scaffold_map.len(), 1);
		assert!(store.scaffold_map.contains_key("foo"));

		store.rename("bar", "foo");

		assert_eq!(store.scaffold_map.len(), 1);
		assert!(store.scaffold_map.contains_key("foo"));

		Ok(())
	}

	#[test]
	fn test_print_grid() -> Result<()> {
		let (mut store, _dir, _) = mock_store(false)?;

		assert_eq!(store.print_grid(), None);

		for i in 0..7 {
			store.add(mock_scaffold(&format!("foo-{}", i)));
		}

		assert_eq!(
			store.print_grid().unwrap(),
			"foo-0    foo-1    foo-2    foo-3    foo-4    foo-5\nfoo-6"
		);

		Ok(())
	}

	#[test]
	fn test_print_table() -> Result<()> {
		let (mut store, _dir, _) = mock_store(false)?;

		assert_eq!(store.print_table(), None);

		for i in 0..2 {
			store.add(mock_scaffold(&format!("foo-{}", i)));
		}

		let expected = fs::read_to_string("fixtures/print-table.txt")?;

		assert_eq!(
			Vec::from_iter(store.print_table().unwrap().lines()),
			Vec::from_iter(expected.lines())
		);

		Ok(())
	}
}
