use std::path::PathBuf;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ScafalraError {
	#[error("Error accessing specified path: `{}`", .0.display())]
	IOError(PathBuf),
	#[error("No GitHub personal access token configured")]
	NoToken,
	#[error("Call to GitHub api error")]
	GitHubApiError,
	#[error("Could not parse the input: `{}`", .0)]
	RepositoryParseError(String),
	#[error("Serialization or deserialization errors")]
	SerdeError,
}
