mod gql;
mod release;
mod repo;

use std::cell::RefCell;

use anyhow::Result;
use gql::{GraphQLQuery, GraphQLResponse};
#[cfg(test)]
pub use release::mock_release_response_json;
use release::{build_release_query, Release, ReleaseResponseData};
use repo::{build_repo_query, RepoInfo, RepoResponseData};
use serde::de::DeserializeOwned;
use ureq::Agent;

use crate::{
	debug,
	repository::Repository,
	utils::{build_proxy_agent, get_self_version},
};

pub struct GitHubApi {
	token: RefCell<Option<String>>,
	endpoint: String,
	agent: Agent,
}

impl GitHubApi {
	pub fn new(endpoint: Option<&str>) -> Self {
		let endpoint = endpoint
			.unwrap_or("https://api.github.com/graphql")
			.to_string();

		let agent = build_proxy_agent();

		Self {
			token: RefCell::new(None),
			endpoint,
			agent,
		}
	}

	pub fn set_token(&self, token: &str) {
		self.token.replace(Some(token.to_string()));
	}

	fn request<T>(&self, query: GraphQLQuery) -> Result<T>
	where
		T: DeserializeOwned + std::fmt::Debug,
	{
		let Some(ref token) = *self.token.borrow() else {
			anyhow::bail!("No GitHub personal access token configured");
		};

		let response: GraphQLResponse<T> = serde_json::from_reader(
			self.agent
				.post(&self.endpoint)
				.set("authorization", &format!("bearer {}", token))
				.set("content-type", "application/json")
				.set("user-agent", &format!("scafalra/{}", get_self_version()))
				.send_bytes(&serde_json::to_vec(&query)?)?
				.into_reader(),
		)?;

		debug!("response: {:#?}", response);

		let GraphQLResponse { data, errors } = response;

		if let Some(errors) = errors {
			if errors.is_empty() {
				anyhow::bail!("Call to GitHub api error");
			} else {
				anyhow::bail!(
					"Call to GitHub api error: {}",
					errors[0].message
				);
			}
		}

		data.ok_or(anyhow::anyhow!("No response data"))
	}

	pub fn query_repository(&self, repo: &Repository) -> Result<RepoInfo> {
		let repo_info: RepoInfo = self
			.request::<RepoResponseData>(build_repo_query(repo))?
			.into();

		Ok(repo_info)
	}

	pub fn query_release(&self) -> Result<Release> {
		let release: Release = self
			.request::<ReleaseResponseData>(build_release_query())?
			.into();

		Ok(release)
	}
}

#[cfg(test)]
mod tests {
	use anyhow::Result;
	use pretty_assertions::assert_eq;

	use super::GitHubApi;
	use crate::repository::Repository;

	fn mock_repo() -> Repository {
		Repository {
			owner: "shixinhuang99".to_string(),
			name: "scafalra".to_string(),
			subdir: None,
			query: None,
		}
	}

	#[test]
	fn test_repo_query() -> Result<()> {
		let mut server = mockito::Server::new();

		let data = include_str!("../../assets/repo-query-response.json");

		let mock = server
			.mock("POST", "/")
			.with_status(200)
			.with_header("content-type", "application/json")
			.with_body(data)
			.create();

		let github_api = GitHubApi::new(Some(&server.url()));

		github_api.set_token("token");

		let repo_info = github_api.query_repository(&mock_repo())?;

		mock.assert();
		assert_eq!(repo_info.url, "url");
		assert_eq!(repo_info.tarball_url, "tarballUrl");

		Ok(())
	}

	#[test]
	fn test_github_api_request_no_token() {
		let github_api = GitHubApi::new(None);
		let api_result = github_api.query_repository(&mock_repo());

		assert!(api_result.is_err());
	}

	#[test]
	fn test_github_api_request_error() -> Result<()> {
		let mut server = mockito::Server::new();

		let data = include_str!("../../assets/repo-query-error.json");

		let mock = server
			.mock("POST", "/")
			.with_status(200)
			.with_header("content-type", "application/json")
			.with_body(data)
			.create();

		let github_api = GitHubApi::new(Some(&server.url()));

		github_api.set_token("token");

		let api_result = github_api.query_repository(&Repository {
			owner: "foo".to_string(),
			name: "bar".to_string(),
			subdir: None,
			query: None,
		});

		mock.assert();
		assert!(api_result.is_err());

		Ok(())
	}

	#[test]
	fn test_release_query() -> Result<()> {
		let mut server = mockito::Server::new();

		let data = include_str!("../../assets/release-query-response.json");

		let mock = server
			.mock("POST", "/")
			.with_status(200)
			.with_header("content-type", "application/json")
			.with_body(data)
			.create();

		let github_api = GitHubApi::new(Some(&server.url()));

		github_api.set_token("token");

		let release = github_api.query_release()?;

		mock.assert();
		assert_eq!(release.version.to_string(), "0.6.0");

		Ok(())
	}
}
