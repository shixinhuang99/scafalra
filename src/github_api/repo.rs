use serde::{Deserialize, Serialize};

use super::gql::GraphQLQuery;
use crate::{cli::AddArgs, json::ToJson, repository::Repository};

static REPO_GQL: &str = include_str!("repo.gql");

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RepoVariables {
	name: String,
	owner: String,
	expression: Option<String>,
	oid: Option<String>,
	is_default_branch: bool,
}

impl ToJson for RepoVariables {}

#[derive(Debug)]
pub struct RemoteRepo {
	pub tarball_url: String,
	pub url: String,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct RepoResponseData {
	repository: RepositoryData,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
struct RepositoryData {
	url: String,
	default_branch_ref: DefaultBranchRef,
	object: Option<Target>,
}

#[derive(Deserialize, Serialize, Debug)]
struct DefaultBranchRef {
	target: Target,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Target {
	tarball_url: String,
}

impl From<RepoResponseData> for RemoteRepo {
	fn from(value: RepoResponseData) -> Self {
		let RepositoryData {
			default_branch_ref,
			object,
			url,
		} = value.repository;

		let tarball_url = match object {
			Some(val) => val.tarball_url,
			None => default_branch_ref.target.tarball_url,
		};

		Self {
			tarball_url,
			url,
		}
	}
}

pub struct RepoQuery {
	query: &'static str,
	variables: RepoVariables,
}

impl RepoQuery {
	pub fn new(repo: &Repository, args: &AddArgs) -> Self {
		let expression = args
			.branch
			.clone()
			.map(|branch| format!("refs/heads/{}", branch))
			.or_else(|| {
				args.tag.clone().map(|tag| format!("refs/tags/{}", tag))
			});
		let oid = args.commit.clone();
		let is_default_branch = expression.is_none() && oid.is_none();

		Self {
			query: REPO_GQL,
			variables: RepoVariables {
				name: repo.name.clone(),
				owner: repo.owner.clone(),
				expression,
				oid,
				is_default_branch,
			},
		}
	}

	pub fn build(self) -> GraphQLQuery {
		GraphQLQuery {
			query: self.query,
			variables: self.variables.to_json(),
		}
	}
}

#[cfg(test)]
pub fn mock_repo_response_json(url: &str) -> String {
	use crate::github_api::gql::GraphQLResponse;

	let data: RepoResponseData = RepoResponseData {
		repository: RepositoryData {
			url: "url".to_string(),
			default_branch_ref: DefaultBranchRef {
				target: Target {
					tarball_url: format!("{}/tarball", url),
				},
			},
			object: None,
		},
	};

	let response: GraphQLResponse<RepoResponseData> = GraphQLResponse {
		data: Some(data),
		errors: None,
	};

	serde_json::to_string(&response).unwrap()
}

#[cfg(test)]
pub mod test_utils {
	use super::RepoQuery;
	use crate::{
		cli::test_utils::AddArgsMock, repository::test_utils::RepositoryMock,
	};

	pub struct RepoQueryMock {
		repo_query: RepoQuery,
	}

	impl RepoQueryMock {
		pub fn new() -> Self {
			Self {
				repo_query: RepoQuery::new(
					&RepositoryMock::new().build(),
					&AddArgsMock::new().build(),
				),
			}
		}

		pub fn branch(self, branch: &str) -> Self {
			Self {
				repo_query: RepoQuery::new(
					&RepositoryMock::new().build(),
					&AddArgsMock::new().branch(branch).build(),
				),
			}
		}

		pub fn tag(self, tag: &str) -> Self {
			Self {
				repo_query: RepoQuery::new(
					&RepositoryMock::new().build(),
					&AddArgsMock::new().tag(tag).build(),
				),
			}
		}

		pub fn commit(self, commit: &str) -> Self {
			Self {
				repo_query: RepoQuery::new(
					&RepositoryMock::new().build(),
					&AddArgsMock::new().commit(commit).build(),
				),
			}
		}

		pub fn build(self) -> RepoQuery {
			self.repo_query
		}
	}
}

#[cfg(test)]
mod tests {
	use super::test_utils::RepoQueryMock;

	#[test]
	fn test_repo_query() {
		let repo_query = RepoQueryMock::new().build();

		assert_eq!(repo_query.variables.name, "scafalra");
		assert_eq!(repo_query.variables.owner, "shixinhuang99");
		assert_eq!(repo_query.variables.oid, None);
		assert_eq!(repo_query.variables.expression, None);
		assert!(repo_query.variables.is_default_branch);
	}

	#[test]
	fn test_repo_query_branch() {
		let repo_query = RepoQueryMock::new().branch("foo").build();

		assert_eq!(repo_query.variables.oid, None);
		assert_eq!(
			repo_query.variables.expression,
			Some("refs/heads/foo".to_string())
		);
		assert!(!repo_query.variables.is_default_branch);
	}

	#[test]
	fn test_repo_query_tag() {
		let repo_query = RepoQueryMock::new().tag("foo").build();

		assert_eq!(repo_query.variables.oid, None);
		assert_eq!(
			repo_query.variables.expression,
			Some("refs/tags/foo".to_string())
		);
		assert!(!repo_query.variables.is_default_branch);
	}

	#[test]
	fn test_repo_query_commit() {
		let repo_query = RepoQueryMock::new().commit("foo").build();

		assert_eq!(repo_query.variables.oid, Some("foo".to_string()));
		assert_eq!(repo_query.variables.expression, None,);
		assert!(!repo_query.variables.is_default_branch);
	}
}
