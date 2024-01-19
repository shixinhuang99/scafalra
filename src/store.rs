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
pub struct Template {
	pub name: String,
	pub url: String,
	#[tabled(skip)]
	pub path: PathBuf,
	#[tabled(rename = "created at")]
	pub created_at: String,
	#[tabled(skip)]
	pub is_sub_template: Option<bool>,
}

impl Template {
	pub fn new<N, U, P>(name: N, url: U, path: P) -> Self
	where
		N: AsRef<str>,
		U: AsRef<str>,
		P: AsRef<Path>,
	{
		TemplateBuilder::new(name, url, path).build()
	}
}

pub struct TemplateBuilder {
	pub name: String,
	pub url: String,
	pub path: PathBuf,
	pub created_at: String,
	pub is_sub_template: Option<bool>,
}

impl TemplateBuilder {
	pub fn new<N, U, P>(name: N, url: U, path: P) -> Self
	where
		N: AsRef<str>,
		U: AsRef<str>,
		P: AsRef<Path>,
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
			is_sub_template: None,
		}
	}

	pub fn build(self) -> Template {
		Template {
			name: self.name,
			url: self.url,
			path: self.path,
			created_at: self.created_at,
			is_sub_template: self.is_sub_template,
		}
	}

	pub fn sub_template(self, value: bool) -> Self {
		let is_sub_template = value.then_some(true);

		Self {
			is_sub_template,
			..self
		}
	}
}

#[derive(Deserialize, Serialize, Default)]
struct TemplateMap(BTreeMap<String, Template>);

impl JsonContent for TemplateMap {}

impl Deref for TemplateMap {
	type Target = BTreeMap<String, Template>;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl DerefMut for TemplateMap {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.0
	}
}

struct Changes {
	inner: Vec<String>,
}

impl Changes {
	fn new() -> Self {
		Self {
			inner: Vec::new(),
		}
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
	templates: TemplateMap,
	changes: Changes,
}

impl Store {
	pub const FILE_NAME: &'static str = "store.json";

	pub fn new(scafalra_dir: &Path) -> Result<Self> {
		let path = scafalra_dir.join(Self::FILE_NAME);
		let templates = TemplateMap::load(&path)?;
		let changes = Changes::new();

		Ok(Self {
			path,
			templates,
			changes,
		})
	}

	pub fn save(&self) -> Result<()> {
		self.templates.save(&self.path)?;

		self.changes.iter().for_each(|v| {
			println!("{}", v);
		});

		Ok(())
	}

	pub fn add(&mut self, template: Template) {
		let name = &template.name;

		if self.templates.contains_key(name) {
			self.changes.push_remove(name);
		}

		self.changes.push_add(name);
		self.templates.insert(name.to_string(), template);
	}

	pub fn remove(&mut self, name: &str) -> Result<()> {
		if let Some(template) = self.templates.get(name) {
			remove_dir_all(&template.path)?;

			self.changes.push_remove(name);
			self.templates.remove(name);
		}

		Ok(())
	}

	pub fn rename(&mut self, name: &str, new_name: &str) {
		if self.templates.contains_key(new_name) {
			println!("`{}` already exists", new_name);
			return;
		}

		match self.templates.remove(name) {
			Some(template) => {
				self.templates.insert(new_name.to_string(), template);
				self.changes.push_remove(name).push_add(new_name);
			}
			None => {
				println!("No such template `{}`", name);
			}
		};
	}

	pub fn print_grid(&self) -> Option<String> {
		use crate::colorize::Colorize;

		if self.templates.is_empty() {
			return None;
		}

		let mut grid = Grid::new(GridOptions {
			filling: Filling::Spaces(4),
			direction: Direction::LeftToRight,
		});

		self.templates.keys().for_each(|key| {
			grid.add(Cell::from(key.blue()));
		});

		Some(grid.fit_into_columns(6).to_string().trim_end().to_string())
	}

	pub fn print_table(&self) -> Option<String> {
		use crate::colorize::Colorize;

		if self.templates.is_empty() {
			return None;
		}

		let data = Vec::from_iter(self.templates.values().cloned());
		let mut table = Table::new(data);

		let modify = Modify::new(Segment::new(1.., ..1))
			.with(Format::content(|s| s.blue()));

		table
			.with(Style::psql())
			.with(Alignment::left())
			.with(modify);

		Some(table.to_string())
	}

	pub fn get(&self, name: &str) -> Option<&Template> {
		self.templates.get(name)
	}

	pub fn get_similar_name(&self, target: &str) -> Option<&str> {
		use strsim::normalized_levenshtein;

		self.templates
			.keys()
			.filter_map(|name| {
				let score = normalized_levenshtein(target, name).abs();
				if score > 0.5 {
					return Some((name, score));
				}
				None
			})
			.min_by(|x, y| x.1.total_cmp(&y.1))
			.map(|v| v.0.as_str())
	}
}

#[cfg(test)]
pub mod test_utils {
	use std::{
		collections::BTreeMap,
		fs,
		path::{Path, PathBuf},
	};

	use tempfile::{tempdir, TempDir};

	use super::{Store, Template, TemplateMap};

	pub struct StoreJsonMock {
		data: Vec<Template>,
	}

	impl StoreJsonMock {
		pub fn new() -> Self {
			Self {
				data: Vec::new(),
			}
		}

		pub fn push<T>(&mut self, name: &str, path: T) -> &mut Self
		where
			T: AsRef<Path>,
		{
			self.data.push(Template::new(name, "url", path));

			self
		}

		pub fn all_sub_template(&mut self) -> &mut Self {
			for ele in &mut self.data {
				ele.is_sub_template = Some(true);
			}

			self
		}

		pub fn build(&self) -> String {
			let tempalte_map = TemplateMap(BTreeMap::from_iter(
				self.data
					.clone()
					.into_iter()
					.map(|ele| (ele.name.clone(), ele)),
			));

			serde_json::to_string_pretty(&tempalte_map).unwrap()
		}
	}

	pub struct TemplateMock;

	impl TemplateMock {
		pub fn build(name: &str) -> Template {
			Template::new(name, "url", "path")
		}
	}

	pub struct StoreMock {
		pub store: Store,
		pub tmpdir: TempDir,
		pub path: PathBuf,
	}

	impl StoreMock {
		pub fn new() -> Self {
			let tmpdir = tempdir().unwrap();
			let tmp_dir_path = tmpdir.path();
			let foo_path = tmp_dir_path.join("foo");
			let store = Store::new(tmp_dir_path).unwrap();

			Self {
				store,
				tmpdir,
				path: foo_path,
			}
		}

		pub fn with_content(self) -> Self {
			fs::create_dir(&self.path).unwrap();
			let content = StoreJsonMock::new().push("foo", &self.path).build();
			let tmp_dir_path = self.tmpdir.path();
			let store_file = tmp_dir_path.join(Store::FILE_NAME);
			fs::write(store_file, content).unwrap();
			let store = Store::new(tmp_dir_path).unwrap();

			Self {
				store,
				..self
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use std::fs;

	use anyhow::Result;
	use similar_asserts::assert_eq;

	use super::test_utils::{StoreJsonMock, StoreMock, TemplateMock};

	#[test]
	fn test_store_new_file_not_exists() {
		let store_mock = StoreMock::new();

		assert_eq!(store_mock.store.templates.len(), 0);
		assert_eq!(store_mock.store.changes.inner.len(), 0);
	}

	#[test]
	fn test_store_new() {
		let store_mock = StoreMock::new().with_content();

		assert_eq!(store_mock.store.templates.len(), 1);
		assert!(store_mock.store.templates.contains_key("foo"));
	}

	#[test]
	fn test_store_save() -> Result<()> {
		let store_mock = StoreMock::new().with_content();

		store_mock.store.save()?;

		let actual = fs::read_to_string(store_mock.store.path)?;
		let expect = StoreJsonMock::new().push("foo", &store_mock.path).build();

		assert_eq!(actual, expect);

		Ok(())
	}

	#[test]
	fn test_store_add() {
		let mut store_mock = StoreMock::new();

		store_mock.store.add(TemplateMock::build("foo"));

		assert_eq!(store_mock.store.templates.len(), 1);
		assert!(store_mock.store.templates.contains_key("foo"));
		assert_eq!(store_mock.store.changes.inner, vec!["+ foo"]);
	}

	#[test]
	fn test_store_add_same() {
		let mut store_mock = StoreMock::new().with_content();

		store_mock.store.add(TemplateMock::build("foo"));

		assert_eq!(store_mock.store.templates.len(), 1);
		assert!(store_mock.store.templates.contains_key("foo"));
		assert_eq!(store_mock.store.changes.inner, vec!["- foo", "+ foo"]);
	}

	#[test]
	fn test_store_remove() -> Result<()> {
		let mut store_mock = StoreMock::new().with_content();

		store_mock.store.remove("foo")?;

		assert!(!store_mock.path.exists());
		assert_eq!(store_mock.store.templates.len(), 0);
		assert_eq!(store_mock.store.changes.inner, vec!["- foo"]);

		Ok(())
	}

	#[test]
	fn test_store_remove_not_found() -> Result<()> {
		let mut store_mock = StoreMock::new().with_content();

		store_mock.store.remove("bar")?;

		assert_eq!(store_mock.store.changes.inner.len(), 0);

		Ok(())
	}

	#[test]
	fn test_store_rename() {
		let mut store_mock = StoreMock::new().with_content();

		store_mock.store.rename("foo", "bar");

		assert_eq!(store_mock.store.templates.len(), 1);
		assert!(!store_mock.store.templates.contains_key("foo"));
		assert!(store_mock.store.templates.contains_key("bar"));
		assert_eq!(store_mock.store.changes.inner, vec!["- foo", "+ bar"]);
	}

	#[test]
	fn store_rename_exists_or_not_found() {
		let mut store_mock = StoreMock::new().with_content();

		store_mock.store.rename("foo", "foo");

		assert_eq!(store_mock.store.templates.len(), 1);
		assert!(store_mock.store.templates.contains_key("foo"));

		store_mock.store.rename("bar", "foo");

		assert_eq!(store_mock.store.templates.len(), 1);
		assert!(store_mock.store.templates.contains_key("foo"));
	}

	#[test]
	fn test_print_grid() {
		let mut store_mock = StoreMock::new();

		assert_eq!(store_mock.store.print_grid(), None);

		for i in 0..7 {
			store_mock
				.store
				.add(TemplateMock::build(&format!("foo-{}", i)));
		}

		assert_eq!(
			store_mock.store.print_grid().unwrap(),
			"foo-0    foo-1    foo-2    foo-3    foo-4    foo-5\nfoo-6"
		);
	}

	#[test]
	fn test_print_table() -> Result<()> {
		let mut store_mock = StoreMock::new();

		assert_eq!(store_mock.store.print_table(), None);

		for i in 0..2 {
			store_mock
				.store
				.add(TemplateMock::build(&format!("foo-{}", i)));
		}

		let expect = fs::read_to_string("fixtures/print-table.txt")?;

		assert_eq!(
			Vec::from_iter(store_mock.store.print_table().unwrap().lines()),
			Vec::from_iter(expect.lines())
		);

		Ok(())
	}

	#[test]
	fn test_similar_name() {
		let store_mock = StoreMock::new().with_content();

		assert_eq!(store_mock.store.get_similar_name("fop"), Some("foo"));
	}
}
