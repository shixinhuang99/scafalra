use std::{env, fs, path::Path};

use anyhow::{Context, Result};
use serde::{de::DeserializeOwned, Serialize};
use ureq::{Agent, AgentBuilder, Proxy};

pub fn build_proxy_agent() -> Agent {
	let env_proxy = env::var("https_proxy").or_else(|_| env::var("http_proxy"));
	let agent = AgentBuilder::new();

	if let Ok(env_proxy) = env_proxy {
		let proxy = Proxy::new(env_proxy);
		if let Ok(proxy) = proxy {
			return agent.proxy(proxy).build();
		}
	}

	agent.build()
}

pub trait TomlContent: DeserializeOwned + Serialize + Default {
	fn load(file_path: &Path) -> Result<Self> {
		let content: Self = if file_path.exists() {
			toml::from_str(&fs::read_to_string(file_path).with_context(
				|| format!("failed to read the file `{}`", file_path.display()),
			)?)
			.with_context(|| {
				format!("failed to parse the file `{}`", file_path.display())
			})?
		} else {
			fs::File::create(file_path).with_context(|| {
				format!("failed to create the file `{}`", file_path.display())
			})?;
			Self::default()
		};

		Ok(content)
	}

	fn save(&self, file_path: &Path) -> Result<()> {
		let content = toml::to_string_pretty(self).with_context(|| {
			format!(
				"failed to serialize data to the file `{}`",
				file_path.display()
			)
		})?;
		fs::write(file_path, content).with_context(|| {
			format!(
				"failed to write date to the file `{}`",
				file_path.display()
			)
		})?;

		Ok(())
	}
}

#[cfg(test)]
pub fn scaffold_toml<P>(name: &str, local: P) -> String
where
	P: AsRef<Path>,
{
	use std::path::PathBuf;

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
