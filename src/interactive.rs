use anyhow::Result;
use dialoguer::{theme::ColorfulTheme, FuzzySelect, MultiSelect};

pub fn fuzzy_select(itmes: Vec<&String>) -> Result<Option<&String>> {
	let idx = FuzzySelect::with_theme(&ColorfulTheme::default())
		.with_prompt(
			"Typing to search, use ↑↓ to pick, hit 'Enter' \
            to confirm, or hit 'Esc' to exit",
		)
		.items(&itmes)
		.highlight_matches(true)
		.interact_opt()?;

	Ok(idx.map(|i| itmes[i]))
}

pub fn multi_select(itmes: Vec<&String>) -> Result<Option<Vec<&String>>> {
	let vi = MultiSelect::with_theme(&ColorfulTheme::default())
		.with_prompt(
			"Use ↑↓ to pick, hit 'Enter' to confirm, \
            hit 'Space' to select, or hit 'Esc' to exit",
		)
		.items(&itmes)
		.interact_opt()?;

	Ok(vi.map(|vi| vi.into_iter().map(|i| itmes[i]).collect()))
}
