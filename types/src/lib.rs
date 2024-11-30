use http::{HttpRequest, HttpResponse};
use serde::{Deserialize, Serialize};
use span::Spanned;
use std::collections::HashMap;
use std::fmt::Display;
use ts_rs::TS;

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

/// An unresolved request file represents the raw parsed request file without and resolving environmental, prompts or secrets.
///
/// This is before templating has been applied as well.
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct UnresolvedRequestFile {
    pub config: Option<Spanned<UnresolvedRequestFileConfig>>,
    pub request: Spanned<HttpRequest>,
    pub response: Option<Spanned<HttpResponse>>,

    pub refs: Vec<Spanned<ReferenceType>>,
}

impl UnresolvedRequestFile {
    pub fn var_names(&self) -> Vec<&String> {
        match &self.config {
            Some((config, _)) => match &config.vars {
                Some(envs) => envs.iter().collect(),
                None => vec![],
            },
            None => vec![],
        }
    }

    pub fn env_names(&self) -> Vec<&String> {
        match &self.config {
            Some((config, _)) => match &config.envs {
                Some(envs) => Vec::from_iter(envs.keys()),
                None => vec![],
            },
            None => vec![],
        }
    }

    pub fn prompt_names(&self) -> Vec<&String> {
        let prompt_names = match &self.config {
            Some((config, _)) => match &config.prompts {
                Some(prompts) => prompts.keys().collect(),
                None => vec![],
            },
            None => vec![],
        };

        prompt_names
    }

    pub fn secret_names(&self) -> Vec<&String> {
        let prompt_names = match &self.config {
            Some((config, _)) => match &config.secrets {
                Some(prompts) => prompts.iter().collect(),
                None => vec![],
            },
            None => vec![],
        };

        prompt_names
    }
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct UnresolvedRequestFileConfig {
    pub vars: Option<Vec<String>>,
    pub envs: Option<HashMap<String, HashMap<String, String>>>,
    pub prompts: Option<HashMap<String, Option<String>>>,
    pub secrets: Option<Vec<String>>,
    pub auth: Option<HashMap<String, HashMap<String, String>>>,
}

/// A resolved request file with resolved environmental, prompts and secrets values.
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ResolvedRequestFile {
    pub request: Spanned<HttpRequest>,
    pub response: Option<Spanned<HttpResponse>>,
    pub config: Spanned<ResolvedRequestFileConfig>,

    pub refs: Vec<Spanned<ReferenceType>>,
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ResolvedRequestFileConfig {
    pub env: String,
    pub vars: HashMap<String, String>,
    pub prompts: HashMap<String, String>,
    pub secrets: HashMap<String, String>,
    pub auth: Option<HashMap<String, HashMap<String, String>>>,
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
    mod unresolved_requestfile {
        use std::{collections::HashMap, vec};

        use span::NO_SPAN;

        use crate::{HttpRequest, UnresolvedRequestFile, UnresolvedRequestFileConfig};

        #[test]
        fn get_prompt_names_when_defined() {
            let reqfile = UnresolvedRequestFile {
                config: Some((
                    UnresolvedRequestFileConfig {
                        vars: None,
                        envs: None,
                        prompts: Some(HashMap::from_iter([(
                            "key".to_owned(),
                            Some("value".to_owned()),
                        )])),
                        secrets: None,
                        auth: None,
                    },
                    NO_SPAN,
                )),
                request: (HttpRequest::get("/", "1.1", vec![]), NO_SPAN),
                response: None,
                refs: vec![],
            };

            assert_eq!(vec!["key"], reqfile.prompt_names());
        }

        #[test]
        fn get_prompt_names_when_config_defined_without_prompts() {
            let reqfile = UnresolvedRequestFile {
                config: Some((
                    UnresolvedRequestFileConfig {
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

            assert_eq!(expected, reqfile.prompt_names());
        }

        #[test]
        fn get_prompt_names_when_config_undefined() {
            let reqfile = UnresolvedRequestFile {
                config: None,
                request: (HttpRequest::get("/", "1.1", vec![]), NO_SPAN),
                response: None,
                refs: vec![],
            };

            let expected: Vec<&str> = vec![];

            assert_eq!(expected, reqfile.prompt_names());
        }

        #[test]
        fn get_secret_names_when_defined() {
            let reqfile = UnresolvedRequestFile {
                config: Some((
                    UnresolvedRequestFileConfig {
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

            assert_eq!(vec!["secret_name"], reqfile.secret_names());
        }

        #[test]
        fn get_secret_names_when_config_defined_without_prompts() {
            let reqfile = UnresolvedRequestFile {
                config: Some((
                    UnresolvedRequestFileConfig {
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

            assert_eq!(expected, reqfile.secret_names());
        }

        #[test]
        fn get_secret_names_when_config_undefined() {
            let reqfile = UnresolvedRequestFile {
                config: None,
                request: (HttpRequest::get("/", "1.1", vec![]), NO_SPAN),
                response: None,
                refs: vec![],
            };

            let expected: Vec<&str> = vec![];

            assert_eq!(expected, reqfile.secret_names());
        }

        #[test]
        fn get_envs_when_config_is_defined() {
            let reqfile = UnresolvedRequestFile {
                config: Some((
                    UnresolvedRequestFileConfig {
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

            let mut actual = reqfile.env_names();

            actual.sort();

            assert_eq!(vec!["dev", "prod"], actual);
        }

        #[test]
        fn get_envs_when_config_is_defined_empty() {
            let reqfile = UnresolvedRequestFile {
                config: Some((
                    UnresolvedRequestFileConfig {
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

            let empty: Vec<&String> = Vec::new();

            assert_eq!(empty, reqfile.env_names());
        }

        #[test]
        fn get_envs_when_config_is_defined_but_envs_none() {
            let reqfile = UnresolvedRequestFile {
                config: Some((
                    UnresolvedRequestFileConfig {
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

            let empty: Vec<&String> = Vec::new();

            assert_eq!(empty, reqfile.env_names());
        }

        #[test]
        fn get_envs_when_config_is_missing() {
            let reqfile = UnresolvedRequestFile {
                config: None,
                request: (HttpRequest::get("/", "1.1", vec![]), NO_SPAN),
                response: None,
                refs: vec![],
            };

            let empty: Vec<&String> = Vec::new();

            assert_eq!(empty, reqfile.env_names());
        }
    }

    mod request_display {
        use crate::HttpRequest;

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
