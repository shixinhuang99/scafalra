use std::path::PathBuf;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ScafalraError {
	#[error("No GitHub personal access token configured")]
	NoToken,
	#[error("Call to GitHub api error: {}", .0)]
	GitHubApiError(String),
	#[error("Could not parse the input: `{}`", .0)]
	RepositoryParseError(String),
	#[error("`{}` it is not valid UTF-8 path", .0.display())]
	NonUtf8Path(PathBuf),
}
