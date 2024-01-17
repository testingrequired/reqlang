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
    pub headers: Vec<(String, String)>,
    pub body: Option<String>,
}

/// A request file with an environement applied and values (vars, prompts, secrets)
/// resolved
#[derive(Clone, Debug, PartialEq, Default)]
pub struct RequestFile {
    pub config: RequestFileConfig,
    pub request: String,
    pub response: Option<String>,
}

/// A request file config with an environement applied and values (vars, prompts, secrets)
/// resolved
#[derive(Clone, Debug, PartialEq, Default)]
pub struct RequestFileConfig {
    pub env: String,
    pub vars: HashMap<String, String>,
    pub prompts: HashMap<String, String>,
    pub secrets: HashMap<String, String>,
}

/// A template file without an environement set or templating applied
#[derive(Clone, Debug, PartialEq, Default)]
pub struct TemplateFileUntemplated {
    pub config: TemplateFileUntemplatedConfig,
}

/// A template file config without an environement set or templating applied
#[derive(Clone, Debug, PartialEq, Default)]
pub struct TemplateFileUntemplatedConfig {
    pub template: Option<String>,
}

/// A template file with an environement applied and values (vars, prompts, secrets)
/// resolved
#[derive(Clone, Debug, PartialEq, Default)]
pub struct TemplateFile {
    pub config: TemplateFileConfig,
}

/// A template file config with an environement applied and values (vars, prompts, secrets)
/// resolved
#[derive(Clone, Debug, PartialEq, Default)]
pub struct TemplateFileConfig {
    pub env: String,
    pub vars: HashMap<String, String>,
    pub prompts: HashMap<String, String>,
    pub secrets: HashMap<String, String>,
}
