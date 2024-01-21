use serde::{de::DeserializeOwned, Deserialize, Serialize};

#[derive(Serialize)]
pub struct GraphQLQuery {
	pub query: &'static str,
	pub variables: String,
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
