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

use crate::json_content::JsonContent;

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
struct StoreContent {
	scaffolds: Vec<Scaffold>,
}

impl JsonContent for StoreContent {}

#[cfg(test)]
pub fn mock_store_json(data: Vec<(&str, impl AsRef<Path>)>) -> String {
	let content = StoreContent {
		scaffolds: data
			.into_iter()
			.map(|v| Scaffold::new(v.0, "url", v.1))
			.collect(),
	};
	serde_json::to_string_pretty(&content).unwrap()
}

#[derive(Clone)]
struct ScaffoldMap(BTreeMap<String, Scaffold>);

impl From<StoreContent> for ScaffoldMap {
	fn from(value: StoreContent) -> Self {
		Self(
			value
				.scaffolds
				.into_iter()
				.map(|ele| (ele.name.clone(), ele))
				.collect(),
		)
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

struct Changes {
	inner: Vec<String>,
}

impl Changes {
	fn new() -> Self {
		Self { inner: Vec::new() }
	}

	fn push_add(&mut self, name: &str) {
		use crate::colorize::Colorize;

		self.inner.push(format!("{} {}", "+".success(), name));
	}

	fn push_remove(&mut self, name: &str) {
		use crate::colorize::Colorize;

		self.inner.push(format!("{} {}", "-".error(), name));
	}

	fn iter(&self) -> Iter<'_, String> {
		self.inner.iter()
	}
}

pub struct Store {
	pub path: Utf8PathBuf,
	scaffolds: ScaffoldMap,
	changes: Changes,
}

impl Store {
	pub const FILE_NAME: &'static str = "store.json";

	pub fn new(scafalra_dir: &Utf8Path) -> Result<Self> {
		let path = scafalra_dir.join(Self::FILE_NAME);
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
			remove_dir_all(&scaffold.path)?;

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
	use std::fs;

	use anyhow::Result;
	use camino::Utf8PathBuf;
	use pretty_assertions::assert_eq;
	use tempfile::{tempdir, TempDir};

	use super::{mock_store_json, Scaffold, ScaffoldMap, Store, StoreContent};
	use crate::utf8_path::Utf8PathBufExt;

	fn mock_scaffold(name: &str) -> Scaffold {
		Scaffold::new(name, "url", "path")
	}

	fn mock_store(init_content: bool) -> Result<(Store, TempDir, Utf8PathBuf)> {
		let temp_dir = tempdir()?;
		let temp_dir_path = temp_dir.path().into_utf8_path_buf()?;
		let foo_path = temp_dir_path.join("foo");

		if init_content {
			let store_file = temp_dir_path.join(Store::FILE_NAME);
			fs::create_dir(&foo_path)?;
			let content = mock_store_json(vec![("foo", &foo_path)]);
			fs::write(store_file, content)?;
		}

		let store = Store::new(&temp_dir_path)?;

		Ok((store, temp_dir, foo_path))
	}

	#[test]
	fn test_scaffold_map_store_content_transform() {
		let store_content = StoreContent {
			scaffolds: (0..2)
				.map(|v| mock_scaffold(&format!("foo-{}", v)))
				.collect(),
		};

		let scaffold_map = ScaffoldMap::from(store_content);

		assert_eq!(scaffold_map.len(), 2);
		scaffold_map.iter().enumerate().for_each(|(idx, kv)| {
			assert_eq!(kv.0, &format!("foo-{}", idx));
		});

		let store_content = StoreContent::from(scaffold_map);

		assert_eq!(store_content.scaffolds.len(), 2);
		assert_eq!(store_content.scaffolds[0].name, "foo-0");
		assert_eq!(store_content.scaffolds[1].name, "foo-1");
	}

	#[test]
	fn test_store_new_file_not_exists() -> Result<()> {
		let (store, _dir, _) = mock_store(false)?;

		assert_eq!(store.scaffolds.len(), 0);
		assert_eq!(store.changes.inner.len(), 0);

		Ok(())
	}

	#[test]
	fn test_store_new_file_exists() -> Result<()> {
		let (store, _dir, _) = mock_store(true)?;

		assert_eq!(store.scaffolds.len(), 1);
		assert!(store.scaffolds.contains_key("foo"));

		Ok(())
	}

	#[test]
	fn test_store_save() -> Result<()> {
		let (store, _dir, foo_path) = mock_store(true)?;

		store.save()?;

		let content = fs::read_to_string(store.path)?;
		let expected_content = mock_store_json(vec![("foo", &foo_path)]);

		assert_eq!(content, expected_content);

		Ok(())
	}

	#[test]
	fn test_store_add() -> Result<()> {
		let (mut store, _dir, _) = mock_store(false)?;

		store.add(mock_scaffold("foo"));

		assert_eq!(store.scaffolds.len(), 1);
		assert!(store.scaffolds.contains_key("foo"));
		assert_eq!(store.changes.inner, vec!["+ foo"]);

		Ok(())
	}

	#[test]
	fn test_store_add_same() -> Result<()> {
		let (mut store, _dir, _) = mock_store(true)?;

		store.add(mock_scaffold("foo"));

		assert_eq!(store.scaffolds.len(), 1);
		assert!(store.scaffolds.contains_key("foo"));
		assert_eq!(store.changes.inner, vec!["- foo", "+ foo"]);

		Ok(())
	}

	#[test]
	fn test_store_remove() -> Result<()> {
		let (mut store, dir, foo_path) = mock_store(true)?;

		store.remove("foo")?;

		assert!(!foo_path.exists());
		assert_eq!(store.scaffolds.len(), 0);
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

		assert_eq!(store.scaffolds.len(), 1);
		assert!(!store.scaffolds.contains_key("foo"));
		assert!(store.scaffolds.contains_key("bar"));
		assert_eq!(store.changes.inner, vec!["- foo", "+ bar"]);

		Ok(())
	}

	#[test]
	fn store_rename_exists_or_not_found() -> Result<()> {
		let (mut store, _dir, _) = mock_store(true)?;

		store.rename("foo", "foo");

		assert_eq!(store.scaffolds.len(), 1);
		assert!(store.scaffolds.contains_key("foo"));

		store.rename("bar", "foo");

		assert_eq!(store.scaffolds.len(), 1);
		assert!(store.scaffolds.contains_key("foo"));

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

		#[rustfmt::skip]
		let expected = " name  | url | created at          \n\
						-------+-----+---------------------\n \
						 foo-0 | url | 2023-05-19 00:00:00 \n \
						 foo-1 | url | 2023-05-19 00:00:00 ";

		assert_eq!(store.print_table().unwrap(), expected);

		Ok(())
	}
}
