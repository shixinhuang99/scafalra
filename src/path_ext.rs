use std::path::{Path, PathBuf};

use path_slash::PathBufExt as SlashPathBufExt;

trait PathBufExt {
	fn join_slash<T>(&self, s: T) -> PathBuf
	where
		T: AsRef<str>;
}

impl PathBufExt for Path {
	fn join_slash<T>(&self, s: T) -> PathBuf
	where
		T: AsRef<str>,
	{
		self.join(PathBuf::from_slash(s))
	}
}
