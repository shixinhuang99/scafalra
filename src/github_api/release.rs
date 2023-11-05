use serde::{Deserialize, Serialize};

use super::gql_query_response::{GraphQLQuery, ToJson};

const RELEASE_QUERY: &str = r"
query ($name: String!, $owner: String!) {
	repository(name: $name, owner: $owner) {
	  latestRelease {
		releaseAssets(first: 6) {
		  nodes {
			downloadUrl
		  }
		}
	  }
	}
}";

#[derive(Serialize)]
struct ReleaseVariables {
	name: &'static str,
	owner: &'static str,
}

impl ReleaseVariables {
	pub fn new() -> Self {
		ReleaseVariables {
			name: "scafalra",
			owner: "shixinhuang99",
		}
	}
}

impl ToJson for ReleaseVariables {}

#[derive(Debug)]
pub struct ReleaseQueryResult {
	pub release_assets_url: String,
}

#[derive(Deserialize, Debug)]
pub struct ReleaseResponseData {
	repository: RepositoryData,
}

#[derive(Deserialize, Debug)]
struct RepositoryData {
	#[serde(rename = "latestRelease")]
	latest_release: LatestRelease,
}

#[derive(Deserialize, Debug)]
struct LatestRelease {
	#[serde(rename = "releaseAssets")]
	release_assets: ReleaseAssets,
}

#[derive(Deserialize, Debug)]
struct ReleaseAssets {
	nodes: Vec<Node>,
}

#[derive(Deserialize, Debug)]
struct Node {
	#[serde(rename = "downloadUrl")]
	download_url: String,
}

impl From<ReleaseResponseData> for ReleaseQueryResult {
	fn from(value: ReleaseResponseData) -> Self {
		!unimplemented!()
	}
}

pub fn build_release_query() -> GraphQLQuery {
	GraphQLQuery::new(RELEASE_QUERY, ReleaseVariables::new().to_json())
}
