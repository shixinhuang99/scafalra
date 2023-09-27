use std::path::PathBuf;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ScafalraError {
	#[error("Failed to read or write file: `{}`", .0.display())]
	FileReadOrWrite(PathBuf),
	#[error("No GitHub personal access token configured")]
	NoToken,
	#[error("Call to GitHub api error")]
	GitHubApi,
	#[error("Could not parse the input: `{}`", .0)]
	RepositoryParse(String),
	#[error("Failed to remove the old tarball")]
	RemoveTarball,
	#[error("Failed to download the tarball")]
	DownloadTarball,
	#[error("Failed to unpack the tarball")]
	UnPackTarball,
}
