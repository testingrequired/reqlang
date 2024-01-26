use serde::Deserialize;
use span::Spanned;
use std::collections::HashMap;
use std::fmt::Display;

/// HTTP Request
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Request {
    pub verb: String,
    pub target: String,
    pub http_version: String,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
}

impl Display for Request {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let headers = self
            .headers
            .clone()
            .into_iter()
            .map(|x| format!("{}: {}", x.0, x.1))
            .collect::<Vec<String>>()
            .join("\n");

        let headers = if headers.is_empty() {
            "".to_string()
        } else {
            format!("{}\n\n", &headers)[..].to_string()
        };

        let body = match &self.body {
            Some(x) => format!("{x}\n\n"),
            None => "".to_string(),
        };

        write!(
            f,
            "{} {} HTTP/{}\n{}{}",
            self.verb, self.target, self.http_version, headers, body
        )
    }
}

/// HTTP Response
#[derive(Clone, Debug, PartialEq, Default)]
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
#[derive(Clone, Debug, PartialEq)]
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
#[derive(Clone, Debug, PartialEq, Default)]
pub struct UnresolvedRequestFile {
    pub config: Option<Spanned<UnresolvedRequestFileConfig>>,
    pub request: Spanned<Request>,
    pub response: Option<Spanned<Response>>,

    pub config_refs: Vec<Spanned<ReferenceType>>,
    pub request_refs: Vec<Spanned<ReferenceType>>,
    pub response_refs: Vec<Spanned<ReferenceType>>,
}

#[derive(Clone, Debug, PartialEq, Default, Deserialize)]
pub struct UnresolvedRequestFileConfig {
    pub vars: Option<Vec<String>>,
    pub envs: Option<HashMap<String, HashMap<String, String>>>,
    pub prompts: Option<HashMap<String, Option<String>>>,
    pub secrets: Option<Vec<String>>,
}

/// A resolved request file with resolved environmental, prompts and secrets values.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct ResolvedRequestFile {
    pub request: Spanned<Request>,
    pub response: Option<Spanned<Response>>,
    pub config: Spanned<ResolvedRequestFileConfig>,

    pub config_refs: Vec<Spanned<ReferenceType>>,
    pub request_refs: Vec<Spanned<ReferenceType>>,
    pub response_refs: Vec<Spanned<ReferenceType>>,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct ResolvedRequestFileConfig {
    pub env: String,
    pub vars: HashMap<String, String>,
    pub prompts: HashMap<String, String>,
    pub secrets: HashMap<String, String>,
}

#[cfg(test)]
mod tests {
    mod request_display {
        use std::collections::HashMap;

        use crate::Request;

        #[test]
        fn post_request() {
            let req = Request {
                verb: "POST".to_string(),
                target: "/".to_string(),
                http_version: "1.1".to_string(),
                headers: HashMap::from([("host".to_string(), "https://example.com".to_string())]),
                body: Some("[1, 2, 3]".to_string()),
            };

            assert_eq!(
                format!("{req}"),
                concat!(
                    "POST / HTTP/1.1\n",
                    "host: https://example.com\n\n",
                    "[1, 2, 3]\n\n"
                )
            );
        }

        #[test]
        fn get_request() {
            let req = Request {
                verb: "GET".to_string(),
                target: "/".to_string(),
                http_version: "1.1".to_string(),
                headers: HashMap::from([("host".to_string(), "https://example.com".to_string())]),
                body: None,
            };

            assert_eq!(
                format!("{req}"),
                concat!("GET / HTTP/1.1\n", "host: https://example.com\n\n")
            );
        }

        #[test]
        fn get_request_no_headers() {
            let req = Request {
                verb: "GET".to_string(),
                target: "/".to_string(),
                http_version: "1.1".to_string(),
                headers: HashMap::new(),
                body: None,
            };

            assert_eq!(format!("{req}"), concat!("GET / HTTP/1.1\n"));
        }
    }
}
