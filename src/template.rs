use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tabled::Tabled;

use crate::sub_template::{read_sub_templates, SubTemplate};

#[derive(Deserialize, Serialize, Clone, Tabled)]
pub struct Template {
	#[tabled(order = 0)]
	pub name: String,
	#[tabled(order = 1)]
	pub url: String,
	#[tabled(skip)]
	pub path: PathBuf,
	#[tabled(rename = "created at", order = 3)]
	pub created_at: String,
	#[tabled(
		rename = "sub templates",
		order = 2,
		display_with = "display_sub_templates"
	)]
	pub sub_templates: Vec<SubTemplate>,
}

impl Template {
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

		let path = path.as_ref().to_path_buf();

		let sub_templates = read_sub_templates(&path);

		Self {
			name: String::from(name.as_ref()),
			url: String::from(url.as_ref()),
			path,
			created_at,
			sub_templates,
		}
	}
}

fn display_sub_templates(sub_templates: &[SubTemplate]) -> String {
	sub_templates
		.iter()
		.map(|sub_tpl| sub_tpl.name.as_str())
		.collect::<Vec<&str>>()
		.join(",")
}
