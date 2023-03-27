#![allow(dead_code)]

use serde::{Deserialize, Serialize};

use crate::repotitory::{Query, Repository};

#[derive(Deserialize, Serialize)]
struct GraphQLQuery {
    query: &'static str,
    variables: String,
}

impl GraphQLQuery {
    fn new(repo: &Repository) -> Self {
        Self {
            query: QUERY_TEMPLATE,
            variables: Variable::new(repo).to_string(),
        }
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

#[derive(Serialize, Debug, PartialEq)]
struct Variable {
    name: String,
    owner: String,
    expression: Option<String>,
    oid: Option<String>,

    #[serde(rename = "notDefaultBranch")]
    not_default_branch: bool,
}

impl Variable {
    fn new(repo: &Repository) -> Self {
        let (expression, oid) = match &repo.query {
            Some(Query::BRANCH(branch_val)) => {
                (Some(format!("refs/heads/{}", branch_val)), None)
            }
            Some(Query::TAG(tag_val)) => {
                (Some(format!("refs/tags/{}", tag_val)), None)
            }
            Some(Query::COMMIT(oid_val)) => (None, Some(oid_val.clone())),
            _ => (None, None),
        };

        let not_default_branch = match (&expression, &oid) {
            (None, None) => false,
            _ => true,
        };

        Variable {
            name: repo.name.clone(),
            owner: repo.owner.clone(),
            expression,
            oid,
            not_default_branch,
        }
    }

    fn to_string(&self) -> String {
        ureq::serde_json::to_string(self).unwrap()
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

    pub fn request(&self, repo: &Repository) -> GitHubApiResult {
        let query = GraphQLQuery::new(repo);

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

#[derive(Deserialize)]
struct GitHubApiResponse<'a> {
    data: Option<GitHubApiData>,

    #[allow(dead_code)]
    #[serde(skip_deserializing, default)]
    errors: &'a str,
}

#[derive(Deserialize)]
struct GitHubApiData {
    repository: RepositoryData,
}

#[derive(Deserialize)]
struct RepositoryData {
    url: String,

    #[serde(rename = "defaultBranchRef")]
    default_branch_ref: DefaultBranchRef,

    object: Option<Target>,
}

#[derive(Deserialize)]
struct DefaultBranchRef {
    target: Target,
}

#[derive(Deserialize)]
struct Target {
    oid: String,

    #[serde(rename = "zipballUrl")]
    zipball_url: String,
}

#[cfg(test)]
mod tests {
    use super::{Query, Repository, Variable};

    impl Default for Repository {
        fn default() -> Self {
            Self {
                owner: "scafalra".to_string(),
                name: "shixinhuang99".to_string(),
                subdir: None,
                query: None,
            }
        }
    }

    impl Default for Variable {
        fn default() -> Self {
            Self {
                owner: "scafalra".to_string(),
                name: "shixinhuang99".to_string(),
                expression: None,
                oid: None,
                not_default_branch: false,
            }
        }
    }

    #[test]
    fn variable_default() {
        assert_eq!(Variable::default(), Variable::new(&Repository::default()))
    }

    #[test]
    fn variable_query_branch() {
        assert_eq!(
            Variable {
                expression: Some("refs/heads/foo".to_string()),
                not_default_branch: true,
                ..Default::default()
            },
            Variable::new(&Repository {
                query: Some(Query::BRANCH("foo".to_string())),
                ..Default::default()
            })
        )
    }

    #[test]
    fn variable_query_tag() {
        assert_eq!(
            Variable {
                expression: Some("refs/tags/foo".to_string()),
                not_default_branch: true,
                ..Default::default()
            },
            Variable::new(&Repository {
                query: Some(Query::TAG("foo".to_string())),
                ..Default::default()
            })
        )
    }

    #[test]
    fn variable_query_commit() {
        assert_eq!(
            Variable {
                oid: Some("foo".to_string()),
                not_default_branch: true,
                ..Default::default()
            },
            Variable::new(&Repository {
                query: Some(Query::COMMIT("foo".to_string())),
                ..Default::default()
            })
        )
    }
}
