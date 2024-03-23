use anyhow::Result;
use inquire::{MultiSelect, Select, Text};

pub fn select<'a>(
	options: Vec<&'a String>,
	prompt: &str,
	msg_when_empty: &str,
) -> Result<Option<&'a String>> {
	if options.is_empty() {
		anyhow::bail!("{}", msg_when_empty);
	}

	Ok(Select::new(prompt, options).prompt_skippable()?)
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
