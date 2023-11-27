use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::debug;

#[derive(Serialize)]
pub struct GraphQLQuery {
	pub query: &'static str,
	pub variables: String,
}

impl GraphQLQuery {
	pub fn new(query: &'static str, variables: String) -> Self {
		debug!("GraphQL variables json: {}", variables);

		Self { query, variables }
	}
}

#[derive(Deserialize, Serialize, Debug)]
pub struct GraphQLResponse<T>
where
	Option<T>: DeserializeOwned,
{
	pub data: Option<T>,
	pub errors: Option<Vec<GraphQLError>>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct GraphQLError {
	pub message: String,
}
