use semver::Version;
use serde::{Deserialize, Serialize};

use super::gql::GraphQLQuery;
use crate::{
	json::ToJson,
	utils::{get_self_target, get_self_version},
};

const RELEASE_GQL: &str = include_str!("release.gql");

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
		let target = get_self_target();

		let node = value
			.repository
			.latest_release
			.release_assets
			.nodes
			.iter()
			.find(|v| v.download_url.contains(target))
			.expect("Should find a matching release");

		let Node { download_url } = node;
		let self_ver = parse_ver(get_self_version());
		let release_ver = parse_ver(
			download_url
				.split('-')
				.nth(1)
				.expect("Release assets' names must adhere to the format"),
		);

		Self {
			assets_url: download_url.clone(),
			can_update: release_ver > self_ver,
			version: release_ver,
		}
	}
}

pub fn build_release_query() -> GraphQLQuery {
	GraphQLQuery::new(RELEASE_GQL, ReleaseVariables::new().to_json())
}

#[cfg(test)]
pub fn mock_release_response_json(url: &str, ver: &str) -> String {
	let mut data = ReleaseResponseData {
		repository: RepositoryData {
			latest_release: LatestRelease {
				release_assets: ReleaseAssets {
					nodes: vec![
						Node {
							download_url: "x86_64-unknown-linux-gnu.tar.gz"
								.to_string(),
						},
						Node {
							download_url: "x86_64-pc-windows-msvc.zip"
								.to_string(),
						},
						Node {
							download_url: "aarch64-apple-darwin.tar.gz"
								.to_string(),
						},
						Node {
							download_url: "x86_64-apple-darwin.tar.gz"
								.to_string(),
						},
						Node {
							download_url: "aarch64-unknown-linux-gnu.tar.gz"
								.to_string(),
						},
					],
				},
			},
		},
	};

	data.repository
		.latest_release
		.release_assets
		.nodes
		.iter_mut()
		.for_each(|v| {
			v.download_url =
				format!("{}/scafalra-{}-{}", url, ver, v.download_url)
		});

	let data = serde_json::to_string::<ReleaseResponseData>(&data).unwrap();

	format!(r#"{{ "data": {} }}"#, data)
}
