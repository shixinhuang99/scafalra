use thiserror::Error;

#[derive(Error, Debug)]
pub enum ScafalraError {
	#[error("Error accessing specified path: `{}`", .0)]
	IO(String),
	#[error("No GitHub personal access token configured")]
	NoToken,
	#[error("Call to GitHub api error")]
	GitHubApi,
	#[error("Could not parse the input: `{}`", .0)]
	RepositoryParse(String),
}
