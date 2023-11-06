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
	pub release_assets_url: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct ReleaseResponseData {
	repository: RepositoryData,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct RepositoryData {
	latest_release: LatestRelease,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct LatestRelease {
	release_assets: ReleaseAssets,
}

#[derive(Deserialize, Debug)]
struct ReleaseAssets {
	nodes: Vec<Node>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Node {
	download_url: String,
}

impl From<ReleaseResponseData> for ReleaseQueryResult {
	fn from(value: ReleaseResponseData) -> Self {
		let target = if cfg!(all(target_arch = "aarch64", target_os = "macos"))
		{
			Some("aarch64-apple-darwin")
		} else if cfg!(all(target_arch = "x86_64", target_os = "macos")) {
			Some("x86_64-apple-darwin")
		} else if cfg!(all(target_arch = "aarch64", target_os = "linux")) {
			Some("aarch64-unknown-linux-gnu")
		} else if cfg!(all(target_arch = "x86_64", target_os = "linux")) {
			Some("x86_64-unknown-linux-gnu")
		} else if cfg!(all(target_arch = "aarch64", target_os = "windows")) {
			Some("aarch64-pc-windows-msvc")
		} else if cfg!(all(target_arch = "x86_64", target_os = "windows")) {
			Some("x86_64-pc-windows-msvc")
		} else {
			None
		};

		if let Some(target) = target {
			let node = value
				.repository
				.latest_release
				.release_assets
				.nodes
				.iter()
				.find(|v| v.download_url.contains(target));

			return Self {
				release_assets_url: node.map(|v| v.download_url.clone()),
			};
		}

		Self {
			release_assets_url: None,
		}
	}
}

pub fn build_release_query() -> GraphQLQuery {
	GraphQLQuery::new(RELEASE_QUERY, ReleaseVariables::new().to_json())
}
