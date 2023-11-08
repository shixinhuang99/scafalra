mod gql_query_response;
pub mod release;
pub mod repo;

use anyhow::Result;
use gql_query_response::{GraphQLQuery, GraphQLResponse};
use serde::de::DeserializeOwned;
use ureq::Agent;

use crate::{
	debug,
	error::ScafalraError,
	utils::{build_proxy_agent, get_self_version},
};

pub struct GitHubApi {
	token: Option<String>,
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
			token: None,
			endpoint,
			agent,
		}
	}

	pub fn set_token(&mut self, token: &str) {
		self.token = Some(token.to_string());
	}

	pub fn request<T>(&self, query: GraphQLQuery) -> Result<T>
	where
		T: DeserializeOwned + std::fmt::Debug,
	{
		let Some(ref token) = self.token else {
			anyhow::bail!(ScafalraError::NoToken);
		};

		let response: GraphQLResponse<T> = self
			.agent
			.post(&self.endpoint)
			.set("authorization", &format!("bearer {}", token))
			.set("content-type", "application/json")
			.set("user-agent", &format!("scafalra/{}", get_self_version()))
			.send_json(query)?
			.into_json()?;

		debug!("response: {:#?}", response);

		let GraphQLResponse { data, errors } = response;

		if let Some(errors) = errors {
			anyhow::bail!(ScafalraError::GitHubApiError(
				errors[0].message.clone()
			));
		}

		data.ok_or(anyhow::anyhow!("No response data"))
	}
}

#[cfg(test)]
mod tests {
	use anyhow::Result;
	use pretty_assertions::assert_eq;

	use super::{
		release::{build_release_query, Release, ReleaseResponseData},
		repo::{build_repo_query, RepoInfo, RepoResponseData},
		GitHubApi, GraphQLQuery,
	};
	use crate::repository::Repository;

	fn mock_repo_query() -> GraphQLQuery {
		let repo = Repository {
			owner: "shixinhuang99".to_string(),
			name: "scafalra".to_string(),
			subdir: None,
			query: None,
		};
		build_repo_query(&repo)
	}

	#[test]
	fn test_repo_query() -> Result<()> {
		let mut server = mockito::Server::new();

		let data = r#"{
            "data": {
                "repository": {
                    "url": "url",
                    "defaultBranchRef": {
                        "target": {
                            "tarballUrl": "tarballUrl"
                        }
                    }
                }
            }
        }"#;

		let mock = server
			.mock("POST", "/")
			.with_status(200)
			.with_header("content-type", "application/json")
			.with_body(data)
			.create();

		let mut github_api = GitHubApi::new(Some(&server.url()));

		github_api.set_token("token");

		let repo_info: RepoInfo = github_api
			.request::<RepoResponseData>(mock_repo_query())?
			.into();

		mock.assert();
		assert_eq!(repo_info.url, "url");
		assert_eq!(repo_info.tarball_url, "tarballUrl");

		Ok(())
	}

	#[test]
	fn test_github_api_request_no_token() {
		let github_api = GitHubApi::new(None);
		let api_result =
			github_api.request::<RepoResponseData>(mock_repo_query());

		assert!(api_result.is_err());
	}

	#[test]
	fn test_github_api_request_error() -> Result<()> {
		let mut server = mockito::Server::new();

		let data = r#"{
            "data": {
                "repository": null,
            },
			"errors": [
				{
					"message": "Could not resolve to a Repository with the name 'foo/bar'."
				}
			]
        }"#;

		let mock = server
			.mock("POST", "/")
			.with_status(200)
			.with_header("content-type", "application/json")
			.with_body(data)
			.create();

		let mut github_api = GitHubApi::new(Some(&server.url()));

		github_api.set_token("token");

		let api_result = github_api.request::<RepoResponseData>(
			build_repo_query(&Repository {
				owner: "foo".to_string(),
				name: "bar".to_string(),
				subdir: None,
				query: None,
			}),
		);

		mock.assert();
		assert!(api_result.is_err());

		Ok(())
	}

	#[test]
	fn test_release_query() -> Result<()> {
		let mut server = mockito::Server::new();

		let data = r#"{
			"data": {
				"repository": {
					"latestRelease": {
						"releaseAssets": {
							"nodes": [
								{
									"downloadUrl": "https://github.com/shixinhuang99/scafalra/releases/download/0.6.0/scafalra-0.6.0-x86_64-unknown-linux-gnu.tar.gz"
								},
								{
									"downloadUrl": "https://github.com/shixinhuang99/scafalra/releases/download/0.6.0/scafalra-0.6.0-x86_64-pc-windows-msvc.zip"
								},
								{
									"downloadUrl": "https://github.com/shixinhuang99/scafalra/releases/download/0.6.0/scafalra-0.6.0-aarch64-apple-darwin.tar.gz"
								},
								{
									"downloadUrl": "https://github.com/shixinhuang99/scafalra/releases/download/0.6.0/scafalra-0.6.0-x86_64-apple-darwin.tar.gz"
								},
								{
									"downloadUrl": "https://github.com/shixinhuang99/scafalra/releases/download/0.6.0/scafalra-0.6.0-aarch64-unknown-linux-gnu.tar.gz"
								},
								{
									"downloadUrl": "https://github.com/shixinhuang99/scafalra/releases/download/0.6.0/scafalra-0.6.0-aarch64-pc-windows-msvc.zip"
								},
							]
						}
					}
				}
			}
		}"#;

		let mock = server
			.mock("POST", "/")
			.with_status(200)
			.with_header("content-type", "application/json")
			.with_body(data)
			.create();

		let mut github_api = GitHubApi::new(Some(&server.url()));

		github_api.set_token("token");

		let release: Release = github_api
			.request::<ReleaseResponseData>(build_release_query())?
			.into();

		mock.assert();
		assert_eq!(release.version.to_string(), "0.6.0");

		Ok(())
	}
}
