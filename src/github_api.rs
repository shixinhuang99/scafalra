#![allow(dead_code)]

use github_api_response::{GitHubApiResponse, RepositoryData};
use serde::{Deserialize, Serialize};

use crate::repotitory::{Query, Repository};

#[derive(Deserialize, Serialize)]
pub struct GraphQLQuery {
    query: &'static str,
    variables: String,
}

impl GraphQLQuery {
    pub fn new(repo: Repository) -> Self {
        Self {
            query: QUERY_TEMPLATE,
            variables: repo_to_variable_string(repo),
        }
    }
}

pub struct GitHubApi {
    token: String,
}

#[derive(Debug)]
pub struct GitHubApiResult {
    pub oid: String,
    pub zipball_url: String,
    pub url: String,
}

impl GitHubApi {
    pub fn new(token: String) -> Self {
        Self { token }
    }

    pub fn request(&self, query: GraphQLQuery) -> GitHubApiResult {
        let response: GitHubApiResponse =
            ureq::post("https://api.github.com/graphql")
                .set("Authorization", format!("bearer {}", self.token).as_str())
                .set("Content-Type", "application/json")
                .set("User-Agent", "scafalra")
                .send_json(query)
                .unwrap()
                .into_json()
                .unwrap();

        let Some(data) = response.data else {
            panic!("GitHub GraphQL response errors");
        };

        let RepositoryData {
            default_branch_ref,
            object,
            url,
        } = data.repository;

        let (oid, zipball_url) = match object {
            Some(val) => (val.oid, val.zipball_url),
            None => (
                default_branch_ref.target.oid,
                default_branch_ref.target.zipball_url,
            ),
        };

        GitHubApiResult {
            oid,
            zipball_url,
            url,
        }
    }
}

mod github_api_response {
    use serde::Deserialize;

    #[derive(Deserialize)]
    pub struct GitHubApiResponse<'a> {
        pub data: Option<GitHubApiData>,

        #[allow(dead_code)]
        #[serde(skip_deserializing, default)]
        errors: &'a str,
    }

    #[derive(Deserialize)]
    pub struct GitHubApiData {
        pub repository: RepositoryData,
    }

    #[derive(Deserialize)]
    pub struct RepositoryData {
        pub url: String,

        #[serde(rename = "defaultBranchRef")]
        pub default_branch_ref: DefaultBranchRef,

        pub object: Option<Target>,
    }

    #[derive(Deserialize)]
    pub struct DefaultBranchRef {
        pub target: Target,
    }

    #[derive(Deserialize)]
    pub struct Target {
        pub oid: String,

        #[serde(rename = "zipballUrl")]
        pub zipball_url: String,
    }
}

const QUERY_TEMPLATE: &str = r"
query ($name: String!, $owner: String!, $oid: GitObjectID, $expression: String, $notDefaultBranch: Boolean!) {
    repository(name: $name, owner: $owner) {
        url
        object(oid: $oid, expression: $expression) @include(if: $notDefaultBranch) {
            ... on Commit {
                oid
                zipballUrl
            }
        }
        defaultBranchRef {
            target {
                ... on Commit {
                oid
                zipballUrl
                }
            }
        }
    }
}";

#[derive(Serialize)]
struct Variable {
    name: String,
    owner: String,
    expression: Option<String>,
    oid: Option<String>,

    #[serde(rename = "notDefaultBranch")]
    not_default_branch: bool,
}

fn repo_to_variable_string(repo: Repository) -> String {
    let (expression, oid) = match repo.query {
        Some(Query::BRANCH(val)) => (Some(format!("refs/heads/{}", val)), None),
        Some(Query::TAG(val)) => (Some(format!("refs/tags/{}", val)), None),
        Some(Query::COMMIT(val)) => (None, Some(val)),
        _ => (None, None),
    };

    let not_default_branch = match (&expression, &oid) {
        (None, None) => false,
        _ => true,
    };

    let v = Variable {
        name: repo.name,
        owner: repo.owner,
        expression,
        oid,
        not_default_branch,
    };

    ureq::serde_json::to_string(&v).unwrap()
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::{GitHubApi, GitHubApiResult, GraphQLQuery, Repository};

    fn get_token() -> String {
        let content = fs::read_to_string("token.txt").unwrap();
        let tokens: Vec<&str> = content.lines().collect();

        tokens[0].to_string()
    }

    impl Default for Repository {
        fn default() -> Self {
            Self {
                owner: "shixinhuang99".to_string(),
                name: "scafalra".to_string(),
                subdir: None,
                query: None,
            }
        }
    }

    impl GitHubApiResult {
        fn assert(&self) -> bool {
            !self.oid.is_empty()
                && !self.url.is_empty()
                && !self.zipball_url.is_empty()
        }
    }

    #[test]
    fn github_api_basic() {
        let token = get_token();
        let repo = Repository::default();
        let api_result = GitHubApi::new(token).request(GraphQLQuery::new(repo));
        assert!(api_result.assert());
    }
}
