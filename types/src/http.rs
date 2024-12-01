use std::{collections::HashMap, fmt::Display};

use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct HttpVerb(String);

impl HttpVerb {
    pub fn get() -> Self {
        "GET".into()
    }

    pub fn post() -> Self {
        "POST".into()
    }
}

impl From<String> for HttpVerb {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for HttpVerb {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}

impl Display for HttpVerb {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct HttpVersion(String);

impl HttpVersion {
    pub fn one_point_one() -> Self {
        "1.1".into()
    }
}

impl Default for HttpVersion {
    fn default() -> Self {
        HttpVersion::one_point_one()
    }
}

impl From<String> for HttpVersion {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for HttpVersion {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}

impl Display for HttpVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// HTTP Request
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct HttpRequest {
    pub verb: HttpVerb,
    pub target: String,
    pub http_version: HttpVersion,
    pub headers: Vec<(String, String)>,
    pub body: Option<String>,
}
impl HttpRequest {
    pub fn new(
        verb: impl Into<HttpVerb>,
        target: impl Into<String>,
        http_version: impl Into<HttpVersion>,
        headers: Vec<(String, String)>,
        body: Option<String>,
    ) -> Self {
        Self {
            verb: verb.into(),
            target: target.into(),
            http_version: http_version.into(),
            headers,
            body,
        }
    }

    pub fn get(
        target: impl Into<String>,
        http_version: impl Into<HttpVersion>,
        headers: Vec<(String, String)>,
    ) -> Self {
        HttpRequest::new(
            HttpVerb::get(),
            target,
            http_version,
            headers,
            Some("".to_owned()),
        )
    }

    pub fn post(
        target: impl Into<String>,
        http_version: impl Into<HttpVersion>,
        headers: Vec<(String, String)>,
        body: Option<&str>,
    ) -> Self {
        HttpRequest::new(
            HttpVerb::post(),
            target,
            http_version,
            headers,
            body.map(|x| x.to_string()),
        )
    }

    pub fn with_header(&mut self, key: &str, value: &str) -> &mut Self {
        self.headers.push((key.to_string(), value.to_string()));

        self
    }
}

impl Display for HttpRequest {
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
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct HttpResponse {
    pub http_version: HttpVersion,
    pub status_code: String,
    pub status_text: String,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
}
