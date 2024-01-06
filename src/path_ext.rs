use std::path::{Path, PathBuf};

use path_slash::PathBufExt;

pub trait JoinSlash {
	fn join_slash<T>(&self, s: T) -> PathBuf
	where
		T: AsRef<str>;
}

pub trait JoinIter<I> {
	fn join_iter<T>(&self, iter: T) -> PathBuf
	where
		T: IntoIterator<Item = I>;
}

impl JoinSlash for Path {
	fn join_slash<T>(&self, s: T) -> PathBuf
	where
		T: AsRef<str>,
	{
		self.join(PathBuf::from_slash(s))
	}
}

impl<I: AsRef<Path>> JoinIter<I> for Path {
	fn join_iter<T>(&self, iter: T) -> PathBuf
	where
		T: IntoIterator<Item = I>,
	{
		self.join(PathBuf::from_iter(iter))
	}
}