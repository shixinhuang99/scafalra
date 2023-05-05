use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::{
    repository::{Query, Repository},
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

        let not_default_branch = !(expression.is_none() && oid.is_none());

        Variable {
            name: repo.name.clone(),
            owner: repo.owner.clone(),
            expression,
            oid,
            not_default_branch,
        }
    }
}

impl std::fmt::Display for Variable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", ureq::serde_json::to_string(self).unwrap())
    }
}

pub struct GitHubApi {
    token: Option<String>,
    endpoint: String,
}

#[derive(Debug)]
pub struct GitHubApiResult {
    pub oid: String,
    pub tarball_url: String,
    pub url: String,
}

impl GitHubApi {
    pub fn new(endpoint: Option<&str>) -> Self {
        let endpoint = endpoint
            .unwrap_or("https://api.github.com/graphql")
            .to_string();

        Self {
            token: None,
            endpoint,
        }
    }

    pub fn set_token(&mut self, token: &str) {
        self.token = Some(token.to_string());
    }

    pub fn request(&self, repo: &Repository) -> Result<GitHubApiResult> {
        let Some(ref token) = self.token else {
            anyhow::bail!("No GitHub personal access token configured");
        };

        let query = GraphQLQuery::new(repo);

        let agent = build_proxy_agent();

        let response: GitHubApiResponse = agent
            .post(&self.endpoint)
            .set("authorization", format!("bearer {}", token).as_str())
            .set("content-type", "application/json")
            .set("user-agent", "scafalra")
            .send_json(query)?
            .into_json()?;

        let Some(data) = response.data else {
            anyhow::bail!("GitHub GraphQL response errors");
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

        assert_eq!(&v.name, "scafalra");
        assert_eq!(&v.owner, "shixinhuang99");
        assert_eq!(v.oid, None);
        assert_eq!(v.expression, None);
        assert_eq!(v.not_default_branch, false);
    }

    #[test]
    fn variable_query_branch() {
        let v = Variable::new(&Repository {
            query: Some(Query::Branch("foo".to_string())),
            ..build_repository()
        });

        assert_eq!(&v.name, "scafalra");
        assert_eq!(&v.owner, "shixinhuang99");
        assert_eq!(v.oid, None);
        assert_eq!(v.expression, Some("refs/heads/foo".to_string()));
        assert_eq!(v.not_default_branch, true);
    }

    #[test]
    fn variable_query_tag() {
        let v = Variable::new(&Repository {
            query: Some(Query::Tag("foo".to_string())),
            ..build_repository()
        });

        assert_eq!(&v.name, "scafalra");
        assert_eq!(&v.owner, "shixinhuang99");
        assert_eq!(v.oid, None);
        assert_eq!(v.expression, Some("refs/tags/foo".to_string()));
        assert_eq!(v.not_default_branch, true);
    }

    #[test]
    fn variable_query_commit() {
        let v = Variable::new(&Repository {
            query: Some(Query::Commit("foo".to_string())),
            ..build_repository()
        });

        assert_eq!(&v.name, "scafalra");
        assert_eq!(&v.owner, "shixinhuang99");
        assert_eq!(v.oid, Some("foo".to_string()));
        assert_eq!(v.expression, None);
        assert_eq!(v.not_default_branch, true);
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

        let mut github_api = GitHubApi::new(Some(&server.url()));

        github_api.set_token("token");

        let api_result = github_api.request(&build_repository())?;

        mock.assert();
        assert_eq!(api_result.oid, "ea7c165bac336140bcf08f84758ab752769799be");
        assert_eq!(api_result.url, "https://github.com/shixinhuang99/scafalra");
        assert_eq!(
            api_result.tarball_url,
            "https://codeload.github.com/shixinhuang99/scafalra/legacy.tar.gz/ea7c165bac336140bcf08f84758ab752769799be"
        );

        Ok(())
    }

    #[test]
    fn github_api_request_no_token() {
        let github_api = GitHubApi::new(None);
        let api_result = github_api.request(&build_repository());

        assert!(api_result.is_err());
    }
}
