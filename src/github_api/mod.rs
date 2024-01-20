mod gql;
#[cfg(feature = "self_update")]
mod release;
mod repo;

use std::cell::RefCell;

use anyhow::Result;
use gql::{GraphQLQuery, GraphQLResponse};
#[cfg(test)]
#[cfg(feature = "self_update")]
pub use release::mock_release_response_json;
#[cfg(feature = "self_update")]
use release::{build_release_query, Release, ReleaseResponseData};
#[cfg(test)]
pub use repo::mock_repo_response_json;
use repo::{build_repo_query, RemoteRepo, RepoResponseData};
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
		let token = self.token.borrow().clone().ok_or(anyhow::anyhow!(
			"No GitHub personal access token configured"
		))?;

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

		let GraphQLResponse {
			data,
			errors,
		} = response;

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

	pub fn query_remote_repo(&self, repo: &Repository) -> Result<RemoteRepo> {
		let remote_repo: RemoteRepo = self
			.request::<RepoResponseData>(build_repo_query(repo))?
			.into();

		Ok(remote_repo)
	}

	#[cfg(feature = "self_update")]
	pub fn query_release(&self) -> Result<Release> {
		let release: Release = self
			.request::<ReleaseResponseData>(build_release_query())?
			.into();

		Ok(release)
	}
}

#[cfg(test)]
mod test_utils {
	use mockito::{Mock, ServerGuard};

	use super::GitHubApi;
	use crate::repository::Repository;

	pub struct RepositoryMock {
		owner: String,
		name: String,
	}

	impl RepositoryMock {
		pub fn new() -> Self {
			Self {
				owner: "shixinhuang99".to_string(),
				name: "scafalra".to_string(),
			}
		}

		pub fn build(self) -> Repository {
			Repository {
				owner: self.owner,
				name: self.name,
				..Repository::default()
			}
		}

		pub fn owner(self, owner: &str) -> Self {
			Self {
				owner: owner.to_string(),
				..self
			}
		}

		pub fn name(self, name: &str) -> Self {
			Self {
				name: name.to_string(),
				..self
			}
		}
	}

	pub struct GitHubApiMock {
		pub github_api: GitHubApi,
		pub server: ServerGuard,
		pub mock: Mock,
	}

	impl GitHubApiMock {
		pub fn new(fixture: &str) -> Self {
			let mut server = mockito::Server::new();

			let mock = server
				.mock("POST", "/")
				.with_status(200)
				.with_header("content-type", "application/json")
				.with_body_from_file(fixture)
				.create();

			let github_api = GitHubApi::new(Some(&server.url()));

			Self {
				github_api,
				server,
				mock,
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use anyhow::Result;

	use super::{
		test_utils::{GitHubApiMock, RepositoryMock},
		GitHubApi,
	};

	#[test]
	fn test_repo_query() -> Result<()> {
		let github_api_mock =
			GitHubApiMock::new("fixtures/repo-query-response.json");

		github_api_mock.github_api.set_token("token");

		let repo_urls = github_api_mock
			.github_api
			.query_remote_repo(&RepositoryMock::new().build())?;

		github_api_mock.mock.assert();
		assert_eq!(repo_urls.url, "url");
		assert_eq!(repo_urls.tarball_url, "tarballUrl");

		Ok(())
	}

	#[test]
	fn test_github_api_request_no_token() {
		let github_api = GitHubApi::new(None);
		let api_result =
			github_api.query_remote_repo(&RepositoryMock::new().build());

		assert!(api_result.is_err());
	}

	#[test]
	fn test_github_api_request_error() -> Result<()> {
		let github_api_mock =
			GitHubApiMock::new("fixtures/repo-query-error.json");

		github_api_mock.github_api.set_token("token");

		let api_result = github_api_mock.github_api.query_remote_repo(
			&RepositoryMock::new().owner("foo").name("bar").build(),
		);

		github_api_mock.mock.assert();
		assert!(api_result.is_err());

		Ok(())
	}

	#[test]
	#[cfg(feature = "self_update")]
	fn test_release_query() -> Result<()> {
		let github_api_mock =
			GitHubApiMock::new("fixtures/release-query-response.json");

		github_api_mock.github_api.set_token("token");

		let release = github_api_mock.github_api.query_release()?;

		github_api_mock.mock.assert();
		assert_eq!(release.version.to_string(), "0.6.0");

		Ok(())
	}
}
