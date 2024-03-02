macro_rules! trait_colorize {
	($($method:ident),+) => {
		pub trait Colorize {
			$(
				fn $method(&self) -> String;
			)*
		}
	};
}

macro_rules! impl_colorize_for_str {
	($(($method:ident, $color:ident)),+) => {
		impl Colorize for str {
			$(
				#[cfg(not(test))]
				fn $method(&self) -> String {
					use owo_colors::{colors::xterm, OwoColorize};

					self.fg::<xterm::$color>().to_string()
				}

				#[cfg(test)]
				fn $method(&self) -> String {
					self.to_string()
				}
			)*
		}
	};
}

trait_colorize!(blue, red, green);

impl_colorize_for_str!((blue, UserBlue), (red, UserRed), (green, UserGreen));

#[cfg(test)]
mod tests {
	use super::Colorize;

	#[test]
	fn test_no_color() {
		assert_eq!("foo".blue(), "foo");
		assert_eq!("foo".to_string().blue(), "foo");
	}
}
