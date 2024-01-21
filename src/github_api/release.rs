use semver::Version;
use serde::{Deserialize, Serialize};

use super::gql::GraphQLQuery;
use crate::{
	json::ToJson,
	utils::{SELF_TARGET, SELF_VERSION},
};

static RELEASE_GQL: &str = include_str!("release.gql");

#[derive(Serialize)]
struct ReleaseVariables {
	name: &'static str,
	owner: &'static str,
}

impl ToJson for ReleaseVariables {}

#[derive(Debug)]
pub struct Release {
	pub assets_url: String,
	pub can_update: bool,
	pub version: Version,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct ReleaseResponseData {
	repository: RepositoryData,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
struct RepositoryData {
	latest_release: LatestRelease,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
struct LatestRelease {
	release_assets: ReleaseAssets,
}

#[derive(Deserialize, Serialize, Debug)]
struct ReleaseAssets {
	nodes: Vec<Node>,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Node {
	download_url: String,
}

fn parse_ver(ver: &str) -> Version {
	Version::parse(ver).expect("Must be a valid SemVer")
}

impl From<ReleaseResponseData> for Release {
	fn from(value: ReleaseResponseData) -> Self {
		let node = value
			.repository
			.latest_release
			.release_assets
			.nodes
			.into_iter()
			.find(|v| v.download_url.contains(SELF_TARGET))
			.expect("Should find a matching release");

		let Node {
			download_url,
		} = node;
		let self_ver = parse_ver(SELF_VERSION);
		let release_ver = parse_ver(
			download_url
				.split('-')
				.nth(1)
				.expect("Release assets' names must adhere to the format"),
		);

		Self {
			assets_url: download_url,
			can_update: release_ver > self_ver,
			version: release_ver,
		}
	}
}

pub struct ReleaseQuery {
	query: &'static str,
	variables: ReleaseVariables,
}

impl ReleaseQuery {
	pub fn new() -> Self {
		Self {
			query: RELEASE_GQL,
			variables: ReleaseVariables {
				name: "scafalra",
				owner: "shixinhuang99",
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
pub fn mock_release_response_json(url: &str, ver: &str) -> String {
	use crate::github_api::gql::GraphQLResponse;

	let mut data = ReleaseResponseData {
		repository: RepositoryData {
			latest_release: LatestRelease {
				release_assets: ReleaseAssets {
					nodes: Vec::new(),
				},
			},
		},
	};

	let target_list: [&str; 5] = [
		"x86_64-unknown-linux-gnu.tar.gz",
		"x86_64-apple-darwin.tar.gz",
		"x86_64-pc-windows-msvc.zip",
		"aarch64-unknown-linux-gnu.tar.gz",
		"aarch64-apple-darwin.tar.gz",
	];

	target_list.iter().for_each(|target| {
		data.repository
			.latest_release
			.release_assets
			.nodes
			.push(Node {
				download_url: format!("{}/scafalra-{}-{}", url, ver, target),
			});
	});

	let response: GraphQLResponse<ReleaseResponseData> = GraphQLResponse {
		data: Some(data),
		errors: None,
	};

	serde_json::to_string(&response).unwrap()
}
