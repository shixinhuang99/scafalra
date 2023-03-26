#![allow(dead_code)]

pub struct Repository {
    pub owner: String,
    pub name: String,
    pub subdir: Option<String>,
    pub query: Option<Query>,
}

pub enum Query {
    BRANCH(String),
    TAG(String),
    COMMIT(String),
}
