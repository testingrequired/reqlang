use std::{collections::HashMap, fmt::Display};

use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct HttpVerb(pub String);

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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct HttpStatusCode(u16);

impl Default for HttpStatusCode {
    fn default() -> Self {
        panic!("Default value for HttpStatusCode is not allowed");
    }
}

impl HttpStatusCode {
    pub fn new(status_code: u16) -> Self {
        status_code.try_into().expect("Not a valid status code")
    }

    pub fn is_valid(status_code: u16) -> bool {
        (100..=599).contains(&status_code)
    }
}

impl TryFrom<u16> for HttpStatusCode {
    type Error = String;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        if Self::is_valid(value) {
            Ok(HttpStatusCode(value))
        } else {
            Err("Invalid HTTP status code".into())
        }
    }
}

impl TryFrom<String> for HttpStatusCode {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value
            .parse::<u16>()
            .expect("Should be a valid number")
            .try_into()
    }
}

/// HTTP Response
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct HttpResponse {
    pub http_version: HttpVersion,
    pub status_code: HttpStatusCode,
    pub status_text: String,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
}

#[cfg(test)]
mod tests {
    mod http_status_code {
        use std::u16;

        use rstest::rstest;

        use crate::http::HttpStatusCode;

        #[rstest]
        #[case(100)]
        #[case(101)]
        #[case(102)]
        #[case(103)]
        #[case(200)]
        #[case(201)]
        #[case(202)]
        #[case(203)]
        #[case(204)]
        #[case(205)]
        #[case(206)]
        #[case(207)]
        #[case(208)]
        #[case(226)]
        #[case(300)]
        #[case(301)]
        #[case(302)]
        #[case(303)]
        #[case(304)]
        #[case(305)]
        #[case(306)]
        #[case(307)]
        #[case(308)]
        #[case(400)]
        #[case(401)]
        #[case(402)]
        #[case(403)]
        #[case(404)]
        #[case(405)]
        #[case(406)]
        #[case(407)]
        #[case(408)]
        #[case(409)]
        #[case(410)]
        #[case(411)]
        #[case(412)]
        #[case(413)]
        #[case(414)]
        #[case(415)]
        #[case(416)]
        #[case(417)]
        #[case(418)]
        #[case(421)]
        #[case(422)]
        #[case(423)]
        #[case(424)]
        #[case(425)]
        #[case(426)]
        #[case(428)]
        #[case(429)]
        #[case(431)]
        #[case(451)]
        #[case(500)]
        #[case(501)]
        #[case(502)]
        #[case(503)]
        #[case(504)]
        #[case(505)]
        #[case(506)]
        #[case(507)]
        #[case(508)]
        #[case(510)]
        #[case(511)]
        fn valid(#[case] status_code: u16) {
            matches!(HttpStatusCode::new(status_code), HttpStatusCode(_));
        }

        #[rstest]
        #[case(u16::MIN)]
        #[case(1)]
        #[case(42)]
        #[case(99)]
        #[case(600)]
        #[case(1200)]
        #[case(9999)]
        #[case(u16::MAX)]
        #[should_panic(expected = r#"Not a valid status code: "Invalid HTTP status code""#)]
        fn invalid(#[case] status_code: u16) {
            HttpStatusCode::new(status_code);
        }
    }
}
