use serde::{Deserialize, Serialize};
use span::Spanned;
use std::collections::HashMap;
use std::fmt::Display;

/// HTTP Request
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct Request {
    pub verb: String,
    pub target: String,
    pub http_version: String,
    pub headers: Vec<(String, String)>,
    pub body: Option<String>,
}

impl Request {
    pub fn get(target: &str, http_version: &str, headers: Vec<(String, String)>) -> Self {
        Request {
            verb: "GET".to_string(),
            target: target.to_string(),
            http_version: http_version.to_string(),
            headers,
            body: Some("".to_string()),
        }
    }

    pub fn post(
        target: &str,
        http_version: &str,
        headers: Vec<(String, String)>,
        body: Option<&str>,
    ) -> Self {
        Request {
            verb: "POST".to_string(),
            target: target.to_string(),
            http_version: http_version.to_string(),
            headers,
            body: body.map(|x| x.to_string()),
        }
    }

    pub fn with_header(&mut self, key: &str, value: &str) -> &mut Self {
        self.headers.push((key.to_string(), value.to_string()));

        self
    }
}

impl Display for Request {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let headers = if self.headers.is_empty() {
            None
        } else {
            Some(format!(
                "{}\n",
                self.headers
                    .clone()
                    .into_iter()
                    .map(|x| format!("{}: {}", x.0, x.1))
                    .collect::<Vec<String>>()
                    .join("\n")
                    .trim_end()
            ))
        };

        let body = self
            .body
            .clone()
            .and_then(|x| if x.is_empty() { None } else { Some(x) });

        let the_rest = match (&headers, &body) {
            (Some(headers), Some(body)) => format!("{headers}\n{body}"),
            (Some(headers), None) => headers.to_string(),
            (None, Some(body)) => format!("\n{body}"),
            (None, None) => String::new(),
        };

        write!(
            f,
            "{} {} HTTP/{}\n{}",
            self.verb, self.target, self.http_version, the_rest
        )
    }
}

/// HTTP Response
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct Response {
    pub http_version: String,
    pub status_code: String,
    pub status_text: String,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
}

/// Template reference in a request file
///
/// Syntax: `{{:variable}}`, `{{?prompt}}`, `{{!secret}}`
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ReferenceType {
    Variable(String),
    Prompt(String),
    Secret(String),
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
                ReferenceType::Unknown(name) => format!("???{name}???"),
            }
        )
    }
}

/// An unresolved request file represents the raw parsed request file without and resolving environmental, prompts or secrets.
///
/// This is before templating has been applied as well.
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct UnresolvedRequestFile {
    pub config: Option<Spanned<UnresolvedRequestFileConfig>>,
    pub request: Spanned<Request>,
    pub response: Option<Spanned<Response>>,

    pub refs: Vec<Spanned<ReferenceType>>,
}

impl UnresolvedRequestFile {
    pub fn var_names(&self) -> Vec<&String> {
        match &self.config {
            Some((config, _)) => match &config.vars {
                Some(envs) => envs.iter().map(|x| x).collect(),
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

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct UnresolvedRequestFileConfig {
    pub vars: Option<Vec<String>>,
    pub envs: Option<HashMap<String, HashMap<String, String>>>,
    pub prompts: Option<HashMap<String, Option<String>>>,
    pub secrets: Option<Vec<String>>,
}

/// A resolved request file with resolved environmental, prompts and secrets values.
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct ResolvedRequestFile {
    pub request: Spanned<Request>,
    pub response: Option<Spanned<Response>>,
    pub config: Spanned<ResolvedRequestFileConfig>,

    pub refs: Vec<Spanned<ReferenceType>>,
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct ResolvedRequestFileConfig {
    pub env: String,
    pub vars: HashMap<String, String>,
    pub prompts: HashMap<String, String>,
    pub secrets: HashMap<String, String>,
}

/// A templated request file.
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct TemplatedRequestFile {
    pub request: Request,
    pub response: Option<Response>,
}

#[cfg(test)]
mod tests {
    mod unresolved_requestfile {
        use std::{collections::HashMap, vec};

        use span::NO_SPAN;

        use crate::{Request, UnresolvedRequestFile, UnresolvedRequestFileConfig};

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
                    },
                    NO_SPAN,
                )),
                request: (Request::get("/", "1.1", vec![]), NO_SPAN),
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
                    },
                    NO_SPAN,
                )),
                request: (Request::get("/", "1.1", vec![]), NO_SPAN),
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
                request: (Request::get("/", "1.1", vec![]), NO_SPAN),
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
                    },
                    NO_SPAN,
                )),
                request: (Request::get("/", "1.1", vec![]), NO_SPAN),
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
                    },
                    NO_SPAN,
                )),
                request: (Request::get("/", "1.1", vec![]), NO_SPAN),
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
                request: (Request::get("/", "1.1", vec![]), NO_SPAN),
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
                    },
                    NO_SPAN,
                )),
                request: (Request::get("/", "1.1", vec![]), NO_SPAN),
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
                    },
                    NO_SPAN,
                )),
                request: (Request::get("/", "1.1", vec![]), NO_SPAN),
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
                    },
                    NO_SPAN,
                )),
                request: (Request::get("/", "1.1", vec![]), NO_SPAN),
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
                request: (Request::get("/", "1.1", vec![]), NO_SPAN),
                response: None,
                refs: vec![],
            };

            let empty: Vec<&String> = Vec::new();

            assert_eq!(empty, reqfile.env_names());
        }
    }

    mod request_display {
        use crate::Request;

        #[test]
        fn post_request() {
            let req = Request::post(
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
            let req = Request::get(
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
            let req = Request {
                verb: "GET".to_string(),
                target: "/".to_string(),
                http_version: "1.1".to_string(),
                headers: vec![],
                body: None,
            };

            assert_eq!(concat!("GET / HTTP/1.1\n"), format!("{req}"));
        }
    }
}
