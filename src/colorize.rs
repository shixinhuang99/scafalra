#[cfg(not(test))]
use owo_colors::{colors::xterm, OwoColorize};

pub trait Colorize {
	fn blue(&self) -> String;

	fn red(&self) -> String;

	fn green(&self) -> String;
}

#[cfg(not(test))]
impl Colorize for str {
	fn blue(&self) -> String {
		self.fg::<xterm::UserBlue>().to_string()
	}

	fn green(&self) -> String {
		self.fg::<xterm::UserGreen>().to_string()
	}

	fn red(&self) -> String {
		self.fg::<xterm::UserRed>().to_string()
	}
}

#[cfg(test)]
impl Colorize for str {
	fn blue(&self) -> String {
		self.to_string()
	}

	fn green(&self) -> String {
		self.to_string()
	}

	fn red(&self) -> String {
		self.to_string()
	}
}

#[cfg(test)]
mod tests {
	use super::Colorize;

	#[test]
	fn test_no_color() {
		assert_eq!("foo".blue(), "foo");
		assert_eq!("foo".to_string().blue(), "foo");
	}
}
