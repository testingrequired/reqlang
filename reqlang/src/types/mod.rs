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
            "{{{{{}}}}}",
            match self {
                ReferenceType::Variable(name) => format!(":{name}"),
                ReferenceType::Prompt(name) => format!("?{name}"),
                ReferenceType::Secret(name) => format!("!{name}"),
                ReferenceType::Provider(name) => format!("@{name}"),
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

    /// The secret names declared in the config
    pub fn secrets(&self) -> Vec<String> {
        self.config
            .as_ref()
            .map(|(config, _)| config.secrets())
            .unwrap_or_default()
    }
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
    pub vars: Option<Vec<String>>,
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
            Some(envs) => envs.to_vec(),
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

    pub fn env(&self, env: impl Into<String>) -> Option<HashMap<String, String>> {
        self.envs
            .as_ref()
            .unwrap_or(&HashMap::new())
            .get(&env.into())
            .cloned()
    }

    /// The prompt names declared
    pub fn prompts(&self) -> Vec<String> {
        self.prompts
            .as_ref()
            .map(|prompts| prompts.iter().map(|prompt| prompt.name.clone()).collect())
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
            .unwrap_or_default();

        let envs: Vec<String> = value.envs();

        let prompts: Vec<String> = value.prompts();

        let secrets = value.secrets();

        Self {
            vars,
            envs,
            prompts,
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
            types::{ParsedConfig, ParsedConfigPrompt, ParsedRequestFile, http::HttpRequest},
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
            };

            let expected: Vec<&str> = vec![];

            assert_eq!(expected, reqfile.secrets());
        }

        #[test]
        fn get_envs_when_config_is_defined() {
            let reqfile = ParsedRequestFile {
                config: Some((
                    ParsedConfig {
                        vars: Some(vec!["var".to_string()]),
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
                        vars: Some(vec!["var".to_string()]),
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
            };

            let empty: Vec<String> = Vec::new();

            assert_eq!(empty, reqfile.envs());
        }

        #[test]
        fn get_envs_when_config_is_defined_empty_b() {
            let reqfile = ParsedRequestFile {
                config: Some((
                    ParsedConfig {
                        vars: Some(vec!["var".to_string()]),
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
            };

            let empty: Vec<String> = Vec::new();

            assert_eq!(empty, reqfile.envs());
        }

        #[test]
        fn get_envs_when_config_is_defined_but_envs_none() {
            let reqfile = ParsedRequestFile {
                config: Some((
                    ParsedConfig {
                        vars: Some(vec!["var".to_string()]),
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
            };

            let empty: Vec<String> = Vec::new();

            assert_eq!(empty, reqfile.envs());
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
