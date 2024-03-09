use std::path::{Component, Path, PathBuf};

pub trait JoinIter<I> {
	fn join_iter<T>(&self, iter: T) -> PathBuf
	where
		T: IntoIterator<Item = I>;
}

impl<I: AsRef<Path>> JoinIter<I> for Path {
	fn join_iter<T>(&self, iter: T) -> PathBuf
	where
		T: IntoIterator<Item = I>,
	{
		self.join(PathBuf::from_iter(iter))
	}
}

pub trait JoinCanonicalize {
	fn join_canonicalize(&mut self, path: &Path);
}

impl JoinCanonicalize for PathBuf {
	fn join_canonicalize(&mut self, path: &Path) {
		for comp in path
			.components()
			.filter(|comp| matches!(comp, Component::Normal(_)))
		{
			self.push(comp);
		}
	}
}

#[cfg(test)]
pub use join_slash_ext::JoinSlash;

#[cfg(test)]
mod join_slash_ext {
	use std::path::{Path, PathBuf};

	use path_slash::PathBufExt;

	pub trait JoinSlash {
		fn join_slash<T>(&self, s: T) -> PathBuf
		where
			T: AsRef<str>;
	}

	impl JoinSlash for Path {
		fn join_slash<T>(&self, s: T) -> PathBuf
		where
			T: AsRef<str>,
		{
			self.join(PathBuf::from_slash(s))
		}
	}
}
