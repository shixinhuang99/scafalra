use std::path::{Path, PathBuf};

use anyhow::Result;
use camino::Utf8PathBuf;

pub trait Utf8PathBufExt
where
	Self: Into<PathBuf>,
{
	fn into_utf8_path_buf(self) -> Result<Utf8PathBuf> {
		Utf8PathBuf::from_path_buf(self.into()).map_err(|err_path| {
			anyhow::anyhow!(
				"`{}` it is not valid UTF-8 path",
				err_path.display()
			)
		})
	}
}

impl Utf8PathBufExt for &Path {}
impl Utf8PathBufExt for PathBuf {}
