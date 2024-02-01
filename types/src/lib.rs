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
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
}

impl Request {
    pub fn get(target: &str, http_version: &str, headers: HashMap<String, String>) -> Self {
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
        headers: HashMap<String, String>,
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
        self.headers.insert(key.to_string(), value.to_string());

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
            (Some(headers), None) => format!("{headers}"),
            (None, Some(body)) => format!("\n{body}"),
            (None, None) => format!(""),
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
    mod request_display {
        use std::collections::HashMap;

        use crate::Request;

        #[test]
        fn post_request() {
            let req = Request::post(
                "/",
                "1.1",
                HashMap::from([("host".to_string(), "https://example.com".to_string())]),
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
                HashMap::from([("host".to_string(), "https://example.com".to_string())]),
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
                headers: HashMap::new(),
                body: None,
            };

            assert_eq!(concat!("GET / HTTP/1.1\n"), format!("{req}"));
        }
    }
}
