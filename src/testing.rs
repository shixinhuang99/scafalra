use std::path::{Path, PathBuf};

pub fn scaffold_toml<P>(name: &str, local: P) -> String
where
	P: AsRef<Path>,
{
	let local = PathBuf::from(local.as_ref()).display().to_string();

	let quote = if local.contains('\\') { '\'' } else { '"' };

	format!(
		r#"[[scaffold]]
name = "{}"
url = "url"
local = {}{}{}
created_at = "2023-05-19 00:00:00"
"#,
		name, quote, local, quote,
	)
}
