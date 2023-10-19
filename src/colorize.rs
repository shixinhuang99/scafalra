use std::fmt::Display;

#[cfg(not(test))]
use owo_colors::OwoColorize;
use owo_colors::{colors::xterm, Color};

pub trait Colorize: Sized + Display {
	#[cfg(not(test))]
	fn with_color<T>(&self) -> String
	where
		T: Color,
	{
		self.fg::<T>().to_string()
	}

	#[cfg(test)]
	fn with_color<T>(&self) -> String
	where
		T: Color,
	{
		self.to_string()
	}

	fn primary(&self) -> String {
		self.with_color::<xterm::Cyan>()
	}

	fn error(&self) -> String {
		self.with_color::<xterm::UserRed>()
	}

	fn success(&self) -> String {
		self.with_color::<xterm::UserGreen>()
	}
}

impl Colorize for &str {}

impl Colorize for String {}

#[cfg(test)]
mod tests {
	use super::Colorize;

	#[test]
	fn test_no_color() {
		assert_eq!("foo".primary(), "foo");
		assert_eq!("foo".to_string().primary(), "foo");
	}
}
