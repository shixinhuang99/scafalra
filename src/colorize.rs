macro_rules! impl_colorize_for_str {
	($(($method:ident, $color:ident)),+) => {
		pub trait Colorize {
			$(
				fn $method(&self) -> String;
			)*
		}

		impl Colorize for str {
			$(
				fn $method(&self) -> String {
					use owo_colors::{colors::xterm, OwoColorize, Stream::Stdout};

					self.if_supports_color(Stdout, |v| v.fg::<xterm::$color>()).to_string()
				}
			)*
		}
	};
}

impl_colorize_for_str!((blue, UserBlue), (red, UserRed), (green, UserGreen));
