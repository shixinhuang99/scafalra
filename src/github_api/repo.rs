use serde::{Deserialize, Serialize};

use super::gql::{GraphQLQuery, ToJson};
use crate::repository::{Query, Repository};

const REPO_GQL: &str = include_str!("repo.gql");

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RepoVariables {
	name: String,
	owner: String,
	expression: Option<String>,
	oid: Option<String>,
	is_default_branch: bool,
}

impl RepoVariables {
	pub fn new(repo: &Repository) -> Self {
		let (expression, oid) = match repo.query {
			Some(Query::Branch(ref branch)) => {
				(Some(format!("refs/heads/{}", branch)), None)
			}
			Some(Query::Tag(ref tag)) => {
				(Some(format!("refs/tags/{}", tag)), None)
			}
			Some(Query::Commit(ref oid)) => (None, Some(oid.clone())),
			_ => (None, None),
		};

		let is_default_branch = expression.is_none() && oid.is_none();

		RepoVariables {
			name: repo.name.clone(),
			owner: repo.owner.clone(),
			expression,
			oid,
			is_default_branch,
		}
	}
}

impl ToJson for RepoVariables {}

#[derive(Debug)]
pub struct RepoInfo {
	pub tarball_url: String,
	pub url: String,
}

#[derive(Deserialize, Debug)]
pub struct RepoResponseData {
	repository: RepositoryData,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct RepositoryData {
	url: String,
	default_branch_ref: DefaultBranchRef,
	object: Option<Target>,
}

#[derive(Deserialize, Debug)]
struct DefaultBranchRef {
	target: Target,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Target {
	tarball_url: String,
}

impl From<RepoResponseData> for RepoInfo {
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

		Self { tarball_url, url }
	}
}

pub fn build_repo_query(repo: &Repository) -> GraphQLQuery {
	GraphQLQuery::new(REPO_GQL, RepoVariables::new(repo).to_json())
}

#[cfg(test)]
mod tests {
	use pretty_assertions::assert_eq;

	use super::{Query, RepoVariables, Repository};

	fn build_repository() -> Repository {
		Repository {
			owner: "shixinhuang99".to_string(),
			name: "scafalra".to_string(),
			subdir: None,
			query: None,
		}
	}

	#[test]
	fn test_variable_new() {
		let v = RepoVariables::new(&build_repository());

		assert_eq!(&v.name, "scafalra");
		assert_eq!(&v.owner, "shixinhuang99");
		assert_eq!(v.oid, None);
		assert_eq!(v.expression, None);
		assert_eq!(v.is_default_branch, true);
	}

	#[test]
	fn test_variable_query_branch() {
		let v = RepoVariables::new(&Repository {
			query: Some(Query::Branch("foo".to_string())),
			..build_repository()
		});

		assert_eq!(&v.name, "scafalra");
		assert_eq!(&v.owner, "shixinhuang99");
		assert_eq!(v.oid, None);
		assert_eq!(v.expression, Some("refs/heads/foo".to_string()));
		assert_eq!(v.is_default_branch, false);
	}

	#[test]
	fn test_variable_query_tag() {
		let v = RepoVariables::new(&Repository {
			query: Some(Query::Tag("foo".to_string())),
			..build_repository()
		});

		assert_eq!(&v.name, "scafalra");
		assert_eq!(&v.owner, "shixinhuang99");
		assert_eq!(v.oid, None);
		assert_eq!(v.expression, Some("refs/tags/foo".to_string()));
		assert_eq!(v.is_default_branch, false);
	}

	#[test]
	fn test_variable_query_commit() {
		let v = RepoVariables::new(&Repository {
			query: Some(Query::Commit("foo".to_string())),
			..build_repository()
		});

		assert_eq!(&v.name, "scafalra");
		assert_eq!(&v.owner, "shixinhuang99");
		assert_eq!(v.oid, Some("foo".to_string()));
		assert_eq!(v.expression, None);
		assert_eq!(v.is_default_branch, false);
	}
}
