use http::{HttpRequest, HttpResponse};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::fmt::Display;
use ts_rs::TS;

use crate::span::Spanned;

pub mod http;

/// Template reference in a request file
///
/// Syntax: `{{:variable}}`, `{{?prompt}}`, `{{!secret}}`
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum ReferenceType {
    Variable(String),
    Prompt(String),
    Secret(String),
    Provider(String),
    Unknown(String),
}

impl Display for ReferenceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                ReferenceType::Variable(name) => format!("{{{{:{name}}}}}"),
                ReferenceType::Prompt(name) => format!("{{{{?{name}}}}}"),
                ReferenceType::Secret(name) => format!("{{{{!{name}}}}}"),
                ReferenceType::Provider(name) => format!("{{{{@{name}}}}}"),
                ReferenceType::Unknown(name) => format!("???{name}???"),
            }
        )
    }
}

/// Request file parsed from a string input
///
/// All template references are still in place
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ParsedRequestFile {
    pub config: Option<Spanned<ParsedConfig>>,
    pub request: Spanned<HttpRequest>,
    pub response: Option<Spanned<HttpResponse>>,
    pub refs: Vec<Spanned<ReferenceType>>,
    pub exprs: Vec<Spanned<String>>,
}

impl ParsedRequestFile {
    /// The variable names declared in the config
    pub fn vars(&self) -> Vec<String> {
        self.config
            .as_ref()
            .map(|(config, _)| config.vars())
            .unwrap_or_default()
    }

    /// The environment names defined in the config
    pub fn envs(&self) -> Vec<String> {
        self.config
            .as_ref()
            .map(|(config, _)| config.envs())
            .unwrap_or_default()
    }

    pub fn env(&self, env: impl Into<String>) -> Option<HashMap<String, String>> {
        self.config.as_ref().and_then(|(config, _)| config.env(env))
    }

    /// The prompt names declared in the config
    pub fn prompts(&self) -> Vec<String> {
        self.config
            .as_ref()
            .map(|(config, _)| config.prompts())
            .unwrap_or_default()
    }

    pub fn required_prompts(&self) -> Vec<String> {
        self.config
            .as_ref()
            .map(|(config, _)| config.required_prompts())
            .unwrap_or_default()
    }

    pub fn optional_prompts(&self) -> Vec<String> {
        self.config
            .as_ref()
            .map(|(config, _)| config.optional_prompts())
            .unwrap_or_default()
    }

    pub fn default_prompt_values(&self) -> HashMap<String, String> {
        let all_prompts = self.config.as_ref().map_or(vec![], |(config, _)| {
            config.prompts.clone().unwrap_or_default()
        });

        let mut all_prompts_map: HashMap<String, String> = HashMap::new();

        for prompt in all_prompts.iter() {
            if prompt.default.is_some() {
                all_prompts_map.insert(
                    prompt.name.clone(),
                    prompt.default.as_ref().unwrap().clone(),
                );
            }
        }

        all_prompts_map
    }

    pub fn default_variable_values(&self) -> HashMap<String, String> {
        let mut default_values = HashMap::new();

        let default_values_pairs: Vec<(String, String)> = self
            .config
            .clone()
            .unwrap_or_default()
            .0
            .vars
            .clone()
            .unwrap_or_default()
            .iter()
            .map(|x| (x.name.clone(), x.default.clone().unwrap_or_default()))
            .collect();

        for (key, value) in &default_values_pairs {
            default_values.insert(key.clone(), value.clone());
        }

        default_values
    }

    /// The secret names declared in the config
    pub fn secrets(&self) -> Vec<String> {
        self.config
            .as_ref()
            .map(|(config, _)| config.secrets())
            .unwrap_or_default()
    }
}

/// A parsed variable definition
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ParsedConfigVariable {
    pub name: String,
    pub default: Option<String>,
}

/// A parsed prompt definition
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ParsedConfigPrompt {
    pub name: String,
    pub description: Option<String>,
    pub default: Option<String>,
}

/// Request file config parsed from a string input
///
/// All template references are still in place
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ParsedConfig {
    /// The variable names declared in the config
    pub vars: Option<Vec<ParsedConfigVariable>>,
    /// Environments with values
    ///
    /// These values match the variable names in the config
    pub envs: Option<HashMap<String, HashMap<String, String>>>,
    /// The prompt names declared in the config
    pub prompts: Option<Vec<ParsedConfigPrompt>>,
    /// The secret names declared in the config
    pub secrets: Option<Vec<String>>,
    pub auth: Option<HashMap<String, HashMap<String, String>>>,
}

impl ParsedConfig {
    /// The variable names declared
    pub fn vars(&self) -> Vec<String> {
        match &self.vars {
            Some(vars) => vars.iter().map(|var| var.clone().name).collect(),
            None => vec![],
        }
    }

    /// The enviroment names defined
    pub fn envs(&self) -> Vec<String> {
        match &self.envs {
            Some(envs) => Vec::from_iter(envs.keys().cloned()),
            None => vec![],
        }
    }

    /// Get variables with values by environment name
    pub fn env(&self, env: impl Into<String>) -> Option<HashMap<String, String>> {
        let mut default_values = HashMap::new();

        let default_values_pairs: Vec<(String, String)> = self
            .vars
            .clone()
            .unwrap_or_default()
            .iter()
            .filter(|x| x.default.is_some())
            .map(|x| (x.name.clone(), x.default.clone().unwrap_or_default()))
            .collect();

        for (key, value) in &default_values_pairs {
            default_values.insert(key.clone(), value.clone());
        }

        let env_values_map = self
            .envs
            .as_ref()
            .unwrap_or(&HashMap::new())
            .get(&env.into())
            .cloned();

        match env_values_map {
            Some(env_values_map) => {
                let mut merged = HashMap::new();
                merged.extend(env_values_map);
                merged.extend(default_values);
                Some(merged)
            }
            None => env_values_map,
        }
    }

    /// The prompt names declared
    pub fn prompts(&self) -> Vec<String> {
        self.prompts
            .as_ref()
            .map(|prompts| prompts.iter().map(|prompt| prompt.name.clone()).collect())
            .unwrap_or_default()
    }

    /// A list of prompt names that don't have a default value defined
    pub fn required_prompts(&self) -> Vec<String> {
        self.prompts
            .as_ref()
            .map(|prompts| {
                prompts
                    .iter()
                    .filter_map(|prompt| {
                        if prompt.default.is_none() {
                            Some(prompt.name.clone())
                        } else {
                            None
                        }
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// A list of prompt names that do have a default value defined
    pub fn optional_prompts(&self) -> Vec<String> {
        self.prompts
            .as_ref()
            .map(|prompts| {
                prompts
                    .iter()
                    .filter_map(|prompt| {
                        if prompt.default.is_some() {
                            Some(prompt.name.clone())
                        } else {
                            None
                        }
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// The secret names declared
    pub fn secrets(&self) -> Vec<String> {
        match &self.secrets {
            Some(secrets) => secrets.to_vec(),
            None => vec![],
        }
    }
}

/// Parameters sent from the client to execute a request.
///
/// This is useful for language server clients
#[derive(Debug, Deserialize, Serialize, Default, TS)]
#[ts(export)]
pub struct RequestParamsFromClient {
    /// The text content of the request file
    pub reqfile: String,
    pub env: Option<String>,
    pub vars: HashMap<String, String>,
    pub prompts: HashMap<String, String>,
    pub secrets: HashMap<String, String>,
}

impl From<Value> for RequestParamsFromClient {
    fn from(params_value: Value) -> Self {
        let reqfile = params_value
            .get("reqfile")
            .expect("Should be present")
            .as_str()
            .expect("Should be a string")
            .to_string();

        let env = params_value
            .get("env")
            .and_then(|x| x.as_str())
            .map(|x| x.to_string());

        let vars_from_params = params_value
            .get("vars")
            .map(|v| v.as_object().expect("Should be there"))
            .expect("Should be there");

        let mut vars: HashMap<String, String> = HashMap::default();

        for (key, value) in vars_from_params {
            vars.insert(
                key.to_string(),
                value.as_str().expect("Should be a string").to_string(),
            );
        }

        let prompts_from_params = params_value
            .get("prompts")
            .map(|v| v.as_object().expect("Should be there"))
            .expect("Should be there");

        let mut prompts: HashMap<String, String> = HashMap::default();

        for (key, value) in prompts_from_params {
            prompts.insert(
                key.to_string(),
                value.as_str().expect("Should be a string").to_string(),
            );
        }

        let secrets_from_params = params_value
            .get("secrets")
            .map(|v| v.as_object().expect("Should be there"))
            .expect("Should be there");

        let mut secrets: HashMap<String, String> = HashMap::default();

        for (key, value) in secrets_from_params {
            secrets.insert(
                key.to_string(),
                value.as_str().expect("Should be a string").to_string(),
            );
        }

        RequestParamsFromClient {
            reqfile,
            env,
            vars,
            prompts,
            secrets,
        }
    }
}

/// A simplified version of a [ParsedRequestFile]
///
/// This is useful for language server clients
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, TS)]
#[ts(export)]
pub struct ParseResult {
    pub vars: Vec<String>,
    pub envs: Vec<String>,
    pub prompts: Vec<String>,
    pub required_prompts: Vec<String>,
    pub optional_prompts: Vec<String>,
    pub default_prompt_values: HashMap<String, String>,
    pub secrets: Vec<String>,
    pub request: HttpRequest,
    pub full: ParsedRequestFile,
}

impl From<ParsedRequestFile> for ParseResult {
    fn from(value: ParsedRequestFile) -> Self {
        let vars = value
            .config
            .clone()
            .unwrap_or_default()
            .0
            .vars
            .unwrap_or_default()
            .iter()
            .map(|x| x.name.clone())
            .collect();

        let envs: Vec<String> = value.envs();

        let prompts: Vec<String> = value.prompts();
        let required_prompts = value.required_prompts();
        let optional_prompts = value.optional_prompts();
        let default_prompt_values = value.default_prompt_values();

        let secrets = value.secrets();

        Self {
            vars,
            envs,
            prompts,
            required_prompts,
            optional_prompts,
            default_prompt_values,
            secrets,
            request: value.clone().request.0,
            full: value,
        }
    }
}

/// A templated request file.
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct TemplatedRequestFile {
    pub request: HttpRequest,
    pub response: Option<HttpResponse>,
}

#[cfg(test)]
mod tests {
    mod parsed_reqfile {
        use std::{collections::HashMap, vec};

        use crate::{
            span::NO_SPAN,
            types::{
                ParsedConfig, ParsedConfigPrompt, ParsedConfigVariable, ParsedRequestFile,
                ReferenceType, http::HttpRequest,
            },
        };

        #[test]
        fn get_prompt_names_when_defined() {
            let reqfile = ParsedRequestFile {
                config: Some((
                    ParsedConfig {
                        vars: None,
                        envs: None,
                        prompts: Some(vec![ParsedConfigPrompt {
                            name: "key".to_string(),
                            description: None,
                            default: Some("value".to_string()),
                        }]),
                        secrets: None,
                        auth: None,
                    },
                    NO_SPAN,
                )),
                request: (HttpRequest::get("/", "1.1", vec![]), NO_SPAN),
                response: None,
                refs: vec![],
                exprs: vec![],
            };

            assert_eq!(vec!["key"], reqfile.prompts());
        }

        #[test]
        fn get_prompt_names_when_config_defined_without_prompts() {
            let reqfile = ParsedRequestFile {
                config: Some((
                    ParsedConfig {
                        vars: None,
                        envs: None,
                        prompts: None,
                        secrets: None,
                        auth: None,
                    },
                    NO_SPAN,
                )),
                request: (HttpRequest::get("/", "1.1", vec![]), NO_SPAN),
                response: None,
                refs: vec![],
                exprs: vec![],
            };

            let expected: Vec<&str> = vec![];

            assert_eq!(expected, reqfile.prompts());
        }

        #[test]
        fn get_prompt_names_when_config_undefined() {
            let reqfile = ParsedRequestFile {
                config: None,
                request: (HttpRequest::get("/", "1.1", vec![]), NO_SPAN),
                response: None,
                refs: vec![],
                exprs: vec![],
            };

            let expected: Vec<&str> = vec![];

            assert_eq!(expected, reqfile.prompts());
        }

        #[test]
        fn get_secret_names_when_defined() {
            let reqfile = ParsedRequestFile {
                config: Some((
                    ParsedConfig {
                        vars: None,
                        envs: None,
                        prompts: None,
                        secrets: Some(vec!["secret_name".to_owned()]),
                        auth: None,
                    },
                    NO_SPAN,
                )),
                request: (HttpRequest::get("/", "1.1", vec![]), NO_SPAN),
                response: None,
                refs: vec![],
                exprs: vec![],
            };

            assert_eq!(vec!["secret_name"], reqfile.secrets());
        }

        #[test]
        fn get_secret_names_when_config_defined_without_prompts() {
            let reqfile = ParsedRequestFile {
                config: Some((
                    ParsedConfig {
                        vars: None,
                        envs: None,
                        prompts: None,
                        secrets: None,
                        auth: None,
                    },
                    NO_SPAN,
                )),
                request: (HttpRequest::get("/", "1.1", vec![]), NO_SPAN),
                response: None,
                refs: vec![],
                exprs: vec![],
            };

            let expected: Vec<&str> = vec![];

            assert_eq!(expected, reqfile.secrets());
        }

        #[test]
        fn get_secret_names_when_config_undefined() {
            let reqfile = ParsedRequestFile {
                config: None,
                request: (HttpRequest::get("/", "1.1", vec![]), NO_SPAN),
                response: None,
                refs: vec![],
                exprs: vec![],
            };

            let expected: Vec<&str> = vec![];

            assert_eq!(expected, reqfile.secrets());
        }

        #[test]
        fn get_envs_when_config_is_defined() {
            let reqfile = ParsedRequestFile {
                config: Some((
                    ParsedConfig {
                        vars: Some(vec![ParsedConfigVariable {
                            name: "var".to_string(),
                            default: None,
                        }]),
                        envs: Some(HashMap::from([
                            (
                                "dev".to_string(),
                                HashMap::from([("var".to_string(), "dev_value".to_string())]),
                            ),
                            (
                                "prod".to_string(),
                                HashMap::from([("var".to_string(), "prod_value".to_string())]),
                            ),
                        ])),
                        prompts: None,
                        secrets: None,
                        auth: None,
                    },
                    NO_SPAN,
                )),
                request: (HttpRequest::get("/", "1.1", vec![]), NO_SPAN),
                response: None,
                refs: vec![],
                exprs: vec![],
            };

            let mut actual = reqfile.envs();

            actual.sort();

            assert_eq!(vec!["dev", "prod"], actual);
        }

        #[test]
        fn get_envs_when_config_is_defined_empty() {
            let reqfile = ParsedRequestFile {
                config: Some((
                    ParsedConfig {
                        vars: Some(vec![ParsedConfigVariable {
                            name: "var".to_string(),
                            default: None,
                        }]),
                        envs: Some(HashMap::new()),
                        prompts: None,
                        secrets: None,
                        auth: None,
                    },
                    NO_SPAN,
                )),
                request: (HttpRequest::get("/", "1.1", vec![]), NO_SPAN),
                response: None,
                refs: vec![],
                exprs: vec![],
            };

            let empty: Vec<String> = Vec::new();

            assert_eq!(empty, reqfile.envs());
        }

        #[test]
        fn get_envs_when_config_is_defined_empty_b() {
            let reqfile = ParsedRequestFile {
                config: Some((
                    ParsedConfig {
                        vars: Some(vec![ParsedConfigVariable {
                            name: "var".to_string(),
                            default: None,
                        }]),
                        envs: None,
                        prompts: None,
                        secrets: None,
                        auth: None,
                    },
                    NO_SPAN,
                )),
                request: (HttpRequest::get("/", "1.1", vec![]), NO_SPAN),
                response: None,
                refs: vec![],
                exprs: vec![],
            };

            let empty: Vec<String> = Vec::new();

            assert_eq!(empty, reqfile.envs());
        }

        #[test]
        fn get_envs_when_config_is_defined_but_envs_none() {
            let reqfile = ParsedRequestFile {
                config: Some((
                    ParsedConfig {
                        vars: Some(vec![ParsedConfigVariable {
                            name: "var".to_string(),
                            default: None,
                        }]),
                        envs: None,
                        prompts: None,
                        secrets: None,
                        auth: None,
                    },
                    NO_SPAN,
                )),
                request: (HttpRequest::get("/", "1.1", vec![]), NO_SPAN),
                response: None,
                refs: vec![],
                exprs: vec![],
            };

            let empty: Vec<String> = Vec::new();

            assert_eq!(empty, reqfile.envs());
        }

        #[test]
        fn get_envs_when_config_is_missing() {
            let reqfile = ParsedRequestFile {
                config: None,
                request: (HttpRequest::get("/", "1.1", vec![]), NO_SPAN),
                response: None,
                refs: vec![],
                exprs: vec![],
            };

            let empty: Vec<String> = Vec::new();

            assert_eq!(empty, reqfile.envs());
        }

        #[test]
        fn get_default_variable_value() {
            let reqfile = ParsedRequestFile {
                config: Some((
                    ParsedConfig {
                        vars: Some(vec![ParsedConfigVariable {
                            name: "foo".to_string(),
                            default: Some("123".to_string()),
                        }]),
                        envs: Some(HashMap::from([("test".to_string(), HashMap::new())])),
                        prompts: None,
                        secrets: None,
                        auth: None,
                    },
                    NO_SPAN,
                )),
                request: (
                    HttpRequest::get(
                        "/",
                        "1.1",
                        vec![("x-foo".to_string(), "{{:foo}}".to_string())],
                    ),
                    NO_SPAN,
                ),
                response: None,
                refs: vec![(ReferenceType::Variable("foo".to_string()), NO_SPAN)],
                exprs: vec![],
            };

            assert_eq!(
                Some(HashMap::from([("foo".to_string(), "123".to_string())])),
                reqfile.env("test")
            );
        }
    }

    mod request_display {
        use crate::types::http::HttpRequest;

        #[test]
        fn post_request() {
            let req = HttpRequest::post(
                "/",
                "1.1",
                vec![("host".to_string(), "https://example.com".to_string())],
                Some("[1, 2, 3]\n"),
            );

            assert_eq!(
                concat!(
                    "POST / HTTP/1.1\n",
                    "host: https://example.com\n\n",
                    "[1, 2, 3]\n"
                ),
                format!("{req}"),
            );
        }

        #[test]
        fn get_request() {
            let req = HttpRequest::get(
                "/",
                "1.1",
                vec![("host".to_string(), "https://example.com".to_string())],
            );

            assert_eq!(
                concat!("GET / HTTP/1.1\n", "host: https://example.com\n"),
                format!("{req}"),
            );
        }

        #[test]
        fn get_request_no_headers() {
            let req = HttpRequest::get("/", "1.1", Vec::default());

            assert_eq!(concat!("GET / HTTP/1.1\n"), format!("{req}"));
        }
    }
}
