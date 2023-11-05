mod gql_query_response;
pub mod release;
pub mod repo;

use anyhow::Result;
use gql_query_response::{GraphQLQuery, GraphQLResponse};
use serde::de::DeserializeOwned;
use ureq::Agent;

use crate::{debug, error::ScafalraError, utils::build_proxy_agent};

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

	pub fn request<T, P>(&self, query: GraphQLQuery) -> Result<P>
	where
		T: DeserializeOwned + std::fmt::Debug,
		P: std::convert::From<T>,
	{
		let Some(ref token) = self.token else {
			anyhow::bail!(ScafalraError::NoToken);
		};

		let response: GraphQLResponse<T> = self
			.agent
			.post(&self.endpoint)
			.set("authorization", &format!("bearer {}", token))
			.set("content-type", "application/json")
			.set(
				"user-agent",
				&format!("scafalra/{}", env!("CARGO_PKG_VERSION")),
			)
			.send_json(query)?
			.into_json()?;

		debug!("response: {:#?}", response);

		let GraphQLResponse { data, errors } = response;

		if let Some(errors) = errors {
			anyhow::bail!(ScafalraError::GitHubApiError(
				errors[0].message.clone()
			));
		}

		let data = data.unwrap();

		Ok(data.into())
	}
}

// #[cfg(test)]
// mod tests {
// 	use anyhow::Result;
// 	use pretty_assertions::assert_eq;

// 	use super::{GitHubApi, Query, Repository, Variable};

// 	fn build_repository() -> Repository {
// 		Repository {
// 			owner: "shixinhuang99".to_string(),
// 			name: "scafalra".to_string(),
// 			subdir: None,
// 			query: None,
// 		}
// 	}

// 	#[test]
// 	fn test_variable_new() {
// 		let v = Variable::new(&build_repository());

// 		assert_eq!(&v.name, "scafalra");
// 		assert_eq!(&v.owner, "shixinhuang99");
// 		assert_eq!(v.oid, None);
// 		assert_eq!(v.expression, None);
// 		assert_eq!(v.is_default_branch, true);
// 	}

// 	#[test]
// 	fn test_variable_query_branch() {
// 		let v = Variable::new(&Repository {
// 			query: Some(Query::Branch("foo".to_string())),
// 			..build_repository()
// 		});

// 		assert_eq!(&v.name, "scafalra");
// 		assert_eq!(&v.owner, "shixinhuang99");
// 		assert_eq!(v.oid, None);
// 		assert_eq!(v.expression, Some("refs/heads/foo".to_string()));
// 		assert_eq!(v.is_default_branch, false);
// 	}

// 	#[test]
// 	fn test_variable_query_tag() {
// 		let v = Variable::new(&Repository {
// 			query: Some(Query::Tag("foo".to_string())),
// 			..build_repository()
// 		});

// 		assert_eq!(&v.name, "scafalra");
// 		assert_eq!(&v.owner, "shixinhuang99");
// 		assert_eq!(v.oid, None);
// 		assert_eq!(v.expression, Some("refs/tags/foo".to_string()));
// 		assert_eq!(v.is_default_branch, false);
// 	}

// 	#[test]
// 	fn test_variable_query_commit() {
// 		let v = Variable::new(&Repository {
// 			query: Some(Query::Commit("foo".to_string())),
// 			..build_repository()
// 		});

// 		assert_eq!(&v.name, "scafalra");
// 		assert_eq!(&v.owner, "shixinhuang99");
// 		assert_eq!(v.oid, Some("foo".to_string()));
// 		assert_eq!(v.expression, None);
// 		assert_eq!(v.is_default_branch, false);
// 	}

// 	#[test]
// 	fn test_github_api_request() -> Result<()> {
// 		let mut server = mockito::Server::new();

// 		let data = r#"{
//             "data": {
//                 "repository": {
//                     "url": "url",
//                     "defaultBranchRef": {
//                         "target": {
//                             "tarballUrl": "tarballUrl"
//                         }
//                     }
//                 }
//             }
//         }"#;

// 		let mock = server
// 			.mock("POST", "/")
// 			.with_status(200)
// 			.with_header("content-type", "application/json")
// 			.with_body(data)
// 			.create();

// 		let mut github_api = GitHubApi::new(Some(&server.url()));

// 		github_api.set_token("token");

// 		let api_result = github_api.request(&build_repository())?;

// 		mock.assert();
// 		assert_eq!(api_result.url, "url");
// 		assert_eq!(api_result.tarball_url, "tarballUrl");

// 		Ok(())
// 	}

// 	#[test]
// 	fn test_github_api_request_no_token() {
// 		let github_api = GitHubApi::new(None);
// 		let api_result = github_api.request(&build_repository());

// 		assert!(api_result.is_err());
// 	}

// 	#[test]
// 	fn test_github_api_request_error() -> Result<()> {
// 		let mut server = mockito::Server::new();

// 		let data = r#"{
//             "data": {
//                 "repository": null,
//             },
// 			"errors": [
// 				{
// 					"message": "Could not resolve to a Repository with the name 'foo/bar'."
// 				}
// 			]
//         }"#;

// 		let mock = server
// 			.mock("POST", "/")
// 			.with_status(200)
// 			.with_header("content-type", "application/json")
// 			.with_body(data)
// 			.create();

// 		let mut github_api = GitHubApi::new(Some(&server.url()));

// 		github_api.set_token("token");

// 		let api_result = github_api.request(&Repository {
// 			owner: "foo".to_string(),
// 			name: "bar".to_string(),
// 			..build_repository()
// 		});

// 		mock.assert();
// 		assert!(api_result.is_err());

// 		Ok(())
// 	}
// }
