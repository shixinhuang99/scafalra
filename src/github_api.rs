#![allow(dead_code)]

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

use crate::{
    repotitory::{Query, Repository},
    utils::build_proxy_agent,
};

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
                tarballUrl
            }
        }
        defaultBranchRef {
            target {
                ... on Commit {
                    oid
                    tarballUrl
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
    endpoint: String,
}

#[derive(Debug)]
pub struct GitHubApiResult {
    pub oid: String,
    pub tarball_url: String,
    pub url: String,
}

impl GitHubApi {
    pub fn new(token: &str, endpoint: Option<&str>) -> Self {
        let endpoint = endpoint
            .unwrap_or("https://api.github.com/graphql")
            .to_string();

        Self {
            token: token.to_string(),
            endpoint,
        }
    }

    pub fn request(&self, repo: &Repository) -> Result<GitHubApiResult> {
        let query = GraphQLQuery::new(repo);

        let agent = build_proxy_agent();

        let response: GitHubApiResponse = agent
            .post(&self.endpoint)
            .set("authorization", format!("bearer {}", self.token).as_str())
            .set("content-type", "application/json")
            .set("user-agent", "scafalra")
            .send_json(query)?
            .into_json()?;

        let Some(data) = response.data else {
            bail!("GitHub GraphQL response errors");
        };

        let RepositoryData {
            default_branch_ref,
            object,
            url,
        } = data.repository;

        let (oid, tarball_url) = match object {
            Some(val) => (val.oid, val.tarball_url),
            None => (
                default_branch_ref.target.oid,
                default_branch_ref.target.tarball_url,
            ),
        };

        Ok(GitHubApiResult {
            oid,
            tarball_url,
            url,
        })
    }
}

#[derive(Deserialize)]
struct GitHubApiResponse<'a> {
    data: Option<GitHubApiData>,

    #[allow(dead_code)]
    #[serde(skip_deserializing)]
    errors: Option<&'a str>,
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

    #[serde(rename = "tarballUrl")]
    tarball_url: String,
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use pretty_assertions::assert_eq;

    use super::{GitHubApi, Query, Repository, Variable};

    fn build_repository() -> Repository {
        Repository {
            owner: "shixinhuang99".to_string(),
            name: "scafalra".to_string(),
            subdir: None,
            query: None,
        }
    }

    #[test]
    fn variable_basic() {
        let v = Variable::new(&build_repository());

        assert_eq!("scafalra", &v.name);
        assert_eq!("shixinhuang99", &v.owner);
        assert_eq!(None, v.oid);
        assert_eq!(None, v.expression);
        assert_eq!(false, v.not_default_branch);
    }

    #[test]
    fn variable_query_branch() {
        let v = Variable::new(&Repository {
            query: Some(Query::BRANCH("foo".to_string())),
            ..build_repository()
        });

        assert_eq!("scafalra", &v.name);
        assert_eq!("shixinhuang99", &v.owner);
        assert_eq!(None, v.oid);
        assert_eq!(Some("refs/heads/foo".to_string()), v.expression);
        assert_eq!(true, v.not_default_branch);
    }

    #[test]
    fn variable_query_tag() {
        let v = Variable::new(&Repository {
            query: Some(Query::TAG("foo".to_string())),
            ..build_repository()
        });

        assert_eq!("scafalra", &v.name);
        assert_eq!("shixinhuang99", &v.owner);
        assert_eq!(None, v.oid);
        assert_eq!(Some("refs/tags/foo".to_string()), v.expression);
        assert_eq!(true, v.not_default_branch);
    }

    #[test]
    fn variable_query_commit() {
        let v = Variable::new(&Repository {
            query: Some(Query::COMMIT("foo".to_string())),
            ..build_repository()
        });

        assert_eq!("scafalra", &v.name);
        assert_eq!("shixinhuang99", &v.owner);
        assert_eq!(Some("foo".to_string()), v.oid);
        assert_eq!(None, v.expression);
        assert_eq!(true, v.not_default_branch);
    }

    #[test]
    fn github_api_request_ok() -> Result<()> {
        let mut server = mockito::Server::new();

        let data = r#"{
            "data": {
              "repository": {
                "url": "https://github.com/shixinhuang99/scafalra",
                "defaultBranchRef": {
                  "target": {
                    "oid": "ea7c165bac336140bcf08f84758ab752769799be",
                    "tarballUrl": "https://codeload.github.com/shixinhuang99/scafalra/legacy.tar.gz/ea7c165bac336140bcf08f84758ab752769799be"
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

        let api_result = GitHubApi::new("token", Some(&server.url()))
            .request(&build_repository())?;

        mock.assert();
        assert_eq!(api_result.oid, "ea7c165bac336140bcf08f84758ab752769799be");
        assert_eq!(api_result.url, "https://github.com/shixinhuang99/scafalra");
        assert_eq!(
            api_result.tarball_url,
            "https://codeload.github.com/shixinhuang99/scafalra/legacy.tar.gz/ea7c165bac336140bcf08f84758ab752769799be"
        );

        Ok(())
    }
}
