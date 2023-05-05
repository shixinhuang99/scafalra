use std::{env, fs, path::Path};

use anyhow::Result;
use owo_colors::{colors::xterm, OwoColorize, Stream, SupportsColorsDisplay};
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

fn to_custom_color<'a, InVal, Out, ApplyFn>(
    val: &'a InVal,
    apply: ApplyFn,
) -> SupportsColorsDisplay<'a, InVal, Out, ApplyFn>
where
    InVal: Sized + std::fmt::Display,
    ApplyFn: Fn(&'a InVal) -> Out,
{
    #[cfg(test)]
    owo_colors::set_override(false);

    val.if_supports_color(Stream::Stdout, apply)
}

pub trait Colorize: Sized + std::fmt::Display {
    fn primary(&self) -> String {
        to_custom_color(self, |s| s.fg::<xterm::Cyan>()).to_string()
    }

    fn error(&self) -> String {
        to_custom_color(self, |s| s.fg::<xterm::UserRed>()).to_string()
    }

    fn success(&self) -> String {
        to_custom_color(self, |s| s.fg::<xterm::UserGreen>()).to_string()
    }
}

impl Colorize for &str {}

impl Colorize for String {}

pub trait TomlContent: DeserializeOwned + Serialize + Default {
    fn load(file_path: &Path) -> Result<Self> {
        let content: Self = if file_path.exists() {
            toml::from_str(&fs::read_to_string(file_path)?)?
        } else {
            fs::File::create(file_path)?;
            Self::default()
        };

        Ok(content)
    }

    fn save(&self, file_path: &Path) -> Result<()> {
        let content = toml::to_string_pretty(self)?;
        fs::write(file_path, content)?;

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
"#,
        name, quote, local, quote,
    )
}

#[cfg(test)]
mod tests {
    use super::Colorize;

    #[test]
    fn no_color_in_test() {
        assert_eq!("foo".primary(), "foo");
        assert_eq!("foo".to_string().primary(), "foo");
    }
}
