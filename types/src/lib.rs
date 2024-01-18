use serde::Deserialize;
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Document {
    pub request: Request,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Request {
    pub verb: String,
    pub target: String,
    pub http_version: String,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Response {
    pub http_version: String,
    pub status_code: String,
    pub status_text: String,
    pub headers: Vec<(String, String)>,
    pub body: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct UnresolvedRequestFile {
    pub config: Option<UnresolvedRequestFileConfig>,
    pub request: Request,
    pub response: Option<Response>,
}

#[derive(Clone, Debug, PartialEq, Default, Deserialize)]
pub struct UnresolvedRequestFileConfig {
    pub vars: Vec<String>,
    pub envs: HashMap<String, HashMap<String, String>>,
    pub prompts: HashMap<String, Option<String>>,
    pub secrets: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct ResolvedRequestFile {
    pub config: ResolvedRequestFileConfig,
    pub request: Request,
    pub response: Option<Response>,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct ResolvedRequestFileConfig {
    pub env: String,
    pub vars: HashMap<String, String>,
    pub prompts: HashMap<String, String>,
    pub secrets: HashMap<String, String>,
}
