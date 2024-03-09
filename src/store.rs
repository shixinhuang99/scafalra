use std::{
	collections::BTreeMap,
	ops::{Deref, DerefMut},
	path::{Path, PathBuf},
};

use anyhow::Result;
use remove_dir_all::remove_dir_all;
use serde::{Deserialize, Serialize};
use tabled::{
	settings::{format::Format, object::Segment, Alignment, Modify, Style},
	Table,
};
use term_grid::{Cell, Direction, Filling, Grid, GridOptions};

use crate::{json::JsonContent, template::Template};

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

	fn print_all(&self) {
		for ele in &self.inner {
			println!("{}", ele);
		}
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
		self.changes.print_all();

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

	pub fn rename(&mut self, name: &str, new_name: &str) -> bool {
		if self.templates.contains_key(new_name) {
			println!("`{}` already exists", new_name);
			return false;
		}

		let ret = match self.templates.remove(name) {
			Some(template) => {
				self.templates.insert(new_name.to_string(), template);
				self.changes.push_remove(name).push_add(new_name);

				true
			}
			None => {
				let suggestion = self.similar_name_suggestion(name);
				println!("{}", suggestion);

				false
			}
		};

		ret
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

	pub fn similar_name_suggestion<'a: 'b, 'b>(
		&'a self,
		target: &'a str,
	) -> Suggestion<'b> {
		use strsim::normalized_levenshtein;

		let similar = self
			.templates
			.keys()
			.filter_map(|name| {
				let score = normalized_levenshtein(target, name).abs();
				if score > 0.5 {
					return Some((name, score));
				}
				None
			})
			.max_by(|x, y| x.1.total_cmp(&y.1))
			.map(|v| v.0.as_str());

		Suggestion {
			target,
			similar,
		}
	}

	pub fn all_templates_name(&self) -> Vec<&String> {
		self.templates.values().map(|v| &v.name).collect()
	}
}

pub struct Suggestion<'a> {
	pub target: &'a str,
	pub similar: Option<&'a str>,
}

impl std::fmt::Display for Suggestion<'_> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let mut msg = format!("No such template `{}`", self.target);

		if let Some(similar) = self.similar {
			msg.push_str(&format!("\nA similar template is `{}`", similar));
		}

		write!(f, "{}", msg)
	}
}

#[cfg(test)]
pub mod test_utils {
	use std::{collections::BTreeMap, fs, path::Path};

	use tempfile::{tempdir, TempDir};

	use super::{Store, Template, TemplateMap};
	use crate::sub_template::test_utils::sub_tempaltes_dir_setup;

	pub struct StoreJsonMock {
		data: Vec<Template>,
	}

	impl StoreJsonMock {
		pub fn new() -> Self {
			Self {
				data: Vec::new(),
			}
		}

		pub fn push(&mut self, name: &str, path: &Path) -> &mut Self {
			self.data.push(Template::new(name, "url", path));

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
		pub tmp_dir: TempDir,
	}

	impl StoreMock {
		pub fn with_no_content() -> Self {
			let tmp_dir = tempdir().unwrap();
			let store = Store::new(tmp_dir.path()).unwrap();

			Self {
				tmp_dir,
				store,
			}
		}

		pub fn with_default_content() -> Self {
			let tmp_dir = tempdir().unwrap();
			let tmp_dir_path = tmp_dir.path();
			let foo_path = tmp_dir_path.join("foo");

			fs::create_dir(&foo_path).unwrap();
			fs::write(
				tmp_dir_path.join(Store::FILE_NAME),
				StoreJsonMock::new().push("foo", &foo_path).build(),
			)
			.unwrap();

			let store = Store::new(tmp_dir_path).unwrap();

			Self {
				tmp_dir,
				store,
			}
		}

		pub fn from_range(range: std::ops::Range<i32>) -> Self {
			let tmp_dir = tempdir().unwrap();
			let tmp_dir_path = tmp_dir.path();

			let mut store_json_mock = StoreJsonMock::new();
			for n in range {
				let name = format!("foo-{}", n);
				let name_path = tmp_dir_path.join(&name);
				fs::create_dir(&name_path).unwrap();

				match n {
					0 => {
						sub_tempaltes_dir_setup(
							&name_path,
							&["dir-1", "dir-2"],
						);
					}
					1 => {
						sub_tempaltes_dir_setup(&name_path, &["dir-3"]);
					}
					_ => (),
				}

				store_json_mock.push(&name, &name_path);
			}

			fs::write(
				tmp_dir_path.join(Store::FILE_NAME),
				store_json_mock.build(),
			)
			.unwrap();

			let store = Store::new(tmp_dir_path).unwrap();

			Self {
				tmp_dir,
				store,
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use std::fs;

	use anyhow::Result;
	use similar_asserts::assert_eq;
	use test_case::test_case;

	use super::test_utils::{StoreMock, TemplateMock};

	#[test]
	fn test_store_new_file_not_exists() {
		let StoreMock {
			tmp_dir: _tmp_dir,
			store,
		} = StoreMock::with_no_content();

		assert_eq!(store.templates.len(), 0);
		assert_eq!(store.changes.inner.len(), 0);
	}

	#[test]
	fn test_store_new() {
		let StoreMock {
			tmp_dir: _tmp_dir,
			store,
		} = StoreMock::with_default_content();

		assert_eq!(store.templates.len(), 1);
		assert!(store.templates.contains_key("foo"));
	}

	#[test]
	fn test_store_save() -> Result<()> {
		let StoreMock {
			tmp_dir: _tmp_dir,
			store,
		} = StoreMock::with_default_content();

		fs::write(&store.path, "")?;

		store.save()?;

		let store_file_content = fs::read_to_string(store.path)?;

		assert!(!store_file_content.is_empty());

		Ok(())
	}

	#[test]
	fn test_store_add() {
		let StoreMock {
			tmp_dir: _tmp_dir,
			mut store,
		} = StoreMock::with_no_content();

		store.add(TemplateMock::build("foo"));

		assert_eq!(store.templates.len(), 1);
		assert!(store.templates.contains_key("foo"));
		assert_eq!(store.changes.inner, vec!["+ foo"]);
	}

	#[test]
	fn test_store_add_same() {
		let StoreMock {
			tmp_dir: _tmp_dir,
			mut store,
		} = StoreMock::with_default_content();

		store.add(TemplateMock::build("foo"));

		assert_eq!(store.templates.len(), 1);
		assert!(store.templates.contains_key("foo"));
		assert_eq!(store.changes.inner, vec!["- foo", "+ foo"]);
	}

	#[test]
	fn test_store_remove() -> Result<()> {
		let StoreMock {
			tmp_dir,
			mut store,
		} = StoreMock::with_default_content();

		store.remove("foo")?;

		assert!(!tmp_dir.path().join("foo").exists());
		assert_eq!(store.templates.len(), 0);
		assert_eq!(store.changes.inner, vec!["- foo"]);

		Ok(())
	}

	#[test]
	fn test_store_remove_not_found() -> Result<()> {
		let StoreMock {
			tmp_dir: _tmp_dir,
			mut store,
		} = StoreMock::with_default_content();

		store.remove("bar")?;

		assert_eq!(store.changes.inner.len(), 0);

		Ok(())
	}

	#[test]
	fn test_store_rename() {
		let StoreMock {
			tmp_dir: _tmp_dir,
			mut store,
		} = StoreMock::with_default_content();

		store.rename("foo", "bar");

		assert_eq!(store.templates.len(), 1);
		assert!(!store.templates.contains_key("foo"));
		assert!(store.templates.contains_key("bar"));
		assert_eq!(store.changes.inner, vec!["- foo", "+ bar"]);
	}

	#[test_case("foo"; "exists")]
	#[test_case("bar"; "not found")]
	fn test_store_bad_rename(name: &str) {
		let StoreMock {
			tmp_dir: _tmp_dir,
			mut store,
		} = StoreMock::with_default_content();

		store.rename(name, "foo");

		assert_eq!(store.templates.len(), 1);
		assert!(store.templates.contains_key("foo"));
	}

	#[test]
	fn test_store_print_empty() {
		let StoreMock {
			tmp_dir: _tmp_dir,
			store,
			..
		} = StoreMock::with_no_content();

		assert_eq!(store.print_grid(), None);
		assert_eq!(store.print_table(), None);
	}

	#[test]
	fn test_store_print_grid() {
		let StoreMock {
			tmp_dir: _tmp_dir,
			store,
		} = StoreMock::from_range(0..7);

		assert_eq!(
			store.print_grid().unwrap(),
			concat!(
				"foo-0    foo-1    foo-2    foo-3    foo-4    foo-5\n",
				"foo-6"
			)
		);
	}

	#[test]
	fn test_store_print_table() -> Result<()> {
		let StoreMock {
			tmp_dir: _tmp_dir,
			store,
		} = StoreMock::from_range(0..2);

		assert_eq!(
			store.print_table().unwrap(),
			concat!(
				" name  | url | sub templates | created at          \n",
				"-------+-----+---------------+---------------------\n",
				" foo-0 | url | dir-1,dir-2   | 2023-05-19 00:00:00 \n",
				" foo-1 | url | dir-3         | 2023-05-19 00:00:00 ",
			)
		);

		Ok(())
	}

	#[test]
	fn test_store_similar_name() {
		let StoreMock {
			tmp_dir: _tmp_dir,
			store,
		} = StoreMock::with_default_content();

		assert_eq!(store.similar_name_suggestion("fop").similar, Some("foo"));
	}
}
