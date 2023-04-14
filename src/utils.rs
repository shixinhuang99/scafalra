use std::env;

use owo_colors::{colors::xterm, OwoColorize, Stream, SupportsColorsDisplay};
use ureq::{Agent, AgentBuilder, Proxy};

pub fn build_proxy_agent() -> Agent {
    let env_proxy = env::var("https_proxy").or(env::var("http_proxy"));
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
        to_custom_color(self, |s| s.fg::<xterm::UserBlue>()).to_string()
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

#[cfg(test)]
mod tests {

    #[test]
    fn no_color_in_test() {
        use super::Colorize;

        assert_eq!("foo".primary(), "foo");
        assert_eq!("foo".to_string().primary(), "foo");
    }
}
