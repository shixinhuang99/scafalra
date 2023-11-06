use serde::{Deserialize, Serialize};

use super::gql_query_response::{GraphQLQuery, ToJson};
use crate::repository::{Query, Repository};

const REPO_QUERY: &str = r"
query ($name: String!, $owner: String!, $oid: GitObjectID, $expression: String, $isDefaultBranch: Boolean!) {
    repository(name: $name, owner: $owner) {
        url
        object(oid: $oid, expression: $expression) @skip(if: $isDefaultBranch) {
            ... on Commit {
                tarballUrl
            }
        }
        defaultBranchRef {
            target {
                ... on Commit {
                    tarballUrl
                }
            }
        }
    }
}";

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
pub struct RepoQueryResult {
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

impl From<RepoResponseData> for RepoQueryResult {
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
	GraphQLQuery::new(REPO_QUERY, RepoVariables::new(repo).to_json())
}
