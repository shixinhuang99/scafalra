use anyhow::Result;
use inquire::{MultiSelect, Select, Text};

pub fn template_select(options: Vec<&String>) -> Result<Option<&String>> {
	if options.is_empty() {
		anyhow::bail!("There are no templates");
	}

	Ok(Select::new("Select a template:", options).prompt_skippable()?)
}

pub fn multi_select<'a>(
	options: Vec<&'a String>,
	prompt: &str,
	msg_when_empty: &str,
) -> Result<Option<Vec<&'a String>>> {
	if options.is_empty() {
		anyhow::bail!("{}", msg_when_empty);
	}

	Ok(MultiSelect::new(prompt, options).prompt_skippable()?)
}

pub fn input(prompt: &str) -> Result<Option<String>> {
	Ok(Text::new(prompt).prompt_skippable()?)
}
