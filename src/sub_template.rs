use std::{
	fs,
	path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

#[derive(Clone, Deserialize, Serialize)]
pub struct SubTemplate {
	pub name: String,
	pub path: PathBuf,
}

impl SubTemplate {
	pub fn new(path: &Path) -> Option<Self> {
		if path.is_dir() {
			if let Some(name) = path.file_name() {
				if let Some(name) = name.to_str() {
					return Some(Self {
						name: name.to_string(),
						path: path.to_path_buf(),
					});
				}
			}
		}

		None
	}
}

pub const SUB_TEMPLATE_DIR: &str = ".scafalra";

pub fn read_sub_templates(template_path: &Path) -> Vec<SubTemplate> {
	match fs::read_dir(template_path.join(SUB_TEMPLATE_DIR)) {
		Ok(entries) => {
			let mut vs = Vec::new();
			for entry in entries.filter_map(|e| e.ok()) {
				if let Some(sub_tpl) = SubTemplate::new(&entry.path()) {
					vs.push(sub_tpl);
				}
			}
			vs
		}
		_ => Vec::with_capacity(0),
	}
}

#[cfg(test)]
pub mod test_utils {
	use std::{fs, path::Path};

	use super::SUB_TEMPLATE_DIR;

	pub fn sub_tempaltes_dir_setup(template_path: &Path, dirs: &[&str]) {
		let sub_template_path = template_path.join(SUB_TEMPLATE_DIR);
		fs::create_dir(&sub_template_path).unwrap();

		for dir in dirs {
			fs::create_dir(sub_template_path.join(dir)).unwrap();
		}
	}
}
