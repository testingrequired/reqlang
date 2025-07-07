use std::{fmt::Display, str::FromStr};

use crate::types::http::{HttpRequest, HttpResponse};
use serde::{Deserialize, Serialize};

/// Supported formats to export request files to
#[derive(Copy, Clone, PartialEq, Eq, Debug, Serialize, Deserialize, Default)]
pub enum RequestFormat {
    /// Export as an HTTP Request message
    HttpMessage,
    /// Export as a curl command
    CurlCommand,
    /// Export as a JSON object
    #[default]
    Json,
}

impl Display for RequestFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RequestFormat::HttpMessage => write!(f, "http"),
            RequestFormat::CurlCommand => write!(f, "curl"),
            RequestFormat::Json => write!(f, "json"),
        }
    }
}

impl FromStr for RequestFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "http" => Ok(Self::HttpMessage),
            "curl" => Ok(Self::CurlCommand),
            "json" => Ok(Self::Json),
            _ => Err(format!("Unknown format: {s}")),
        }
    }
}

/// Export an [HttpRequest] in a specified [RequestFormat].
pub fn export(request: &HttpRequest, format: RequestFormat) -> String {
    match format {
        // HTTP Request message
        RequestFormat::HttpMessage => {
            format!("{request}")
        }
        // Curl command
        RequestFormat::CurlCommand => {
            let request_verb_flag = match request.verb.0.as_str() {
                "GET" => String::new(),
                verb => format!("-X {verb} "),
            };

            let request_url = &request.target;

            let header_args = if request.headers.is_empty() {
                None
            } else {
                Some(
                    request
                        .headers
                        .clone()
                        .into_iter()
                        .map(|x| format!(r#"-H "{}: {}""#, x.0, x.1))
                        .collect::<Vec<String>>()
                        .join(" ")
                        .to_string(),
                )
            };

            let body_arg = request.body.clone().and_then(|x| {
                if x.is_empty() {
                    None
                } else {
                    Some(format!("-d '{x}'"))
                }
            });

            let headers_and_body_args = match (&header_args, &body_arg) {
                (Some(headers), Some(body)) => format!(" {headers} {body}"),
                (Some(headers), None) => format!(" {headers}"),
                (None, Some(body)) => format!(" {body}"),
                (None, None) => String::new(),
            };

            format!(
                "curl {}{} --http{}{} -v",
                request_verb_flag, request_url, request.http_version, headers_and_body_args
            )
        }
        RequestFormat::Json => serde_json::to_string_pretty(request).unwrap(),
    }
}

/// Supported formats to export request files to
#[derive(Copy, Clone, PartialEq, Eq, Debug, Serialize, Deserialize, Default)]
pub enum ResponseFormat {
    /// Export as an HTTP Response message
    HttpMessage,
    /// Export as a JSON object
    #[default]
    Json,
    Body,
}

impl Display for ResponseFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResponseFormat::HttpMessage => write!(f, "http"),
            ResponseFormat::Json => write!(f, "json"),
            ResponseFormat::Body => write!(f, "body"),
        }
    }
}

impl FromStr for ResponseFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "http" => Ok(Self::HttpMessage),
            "json" => Ok(Self::Json),
            "body" => Ok(Self::Body),
            _ => Err(format!("Unknown format: {s}")),
        }
    }
}

/// Export an [HttpResponse] in a specified [ResponseFormat].
pub fn export_response(response: &HttpResponse, format: ResponseFormat) -> String {
    match format {
        ResponseFormat::HttpMessage => format!("{response}"),
        ResponseFormat::Json => serde_json::to_string_pretty(response).unwrap(),
        ResponseFormat::Body => response.clone().body.unwrap_or_default(),
    }
}

#[cfg(test)]
mod test {
    use crate::types::http::{HttpRequest, HttpResponse, HttpStatusCode, HttpVersion};

    use super::{RequestFormat, ResponseFormat, export, export_response};

    macro_rules! export_test {
        ($test_name:ident, $request:expr, $format:expr, $expected:expr) => {
            #[test]
            fn $test_name() {
                let actual = export(&$request, $format);
                assert_eq!($expected, actual);
            }
        };
    }

    macro_rules! export_response_test {
        ($test_name:ident, $response:expr, $format:expr, $expected:expr) => {
            #[test]
            fn $test_name() {
                let actual = export_response(&$response, $format);
                assert_eq!($expected, actual);
            }
        };
    }

    export_test!(
        format_to_curl_get_request,
        HttpRequest::get("/", "1.1", vec![]),
        RequestFormat::CurlCommand,
        "curl / --http1.1 -v"
    );

    export_test!(
        format_to_curl_get_request_with_single_header,
        HttpRequest::get("/", "1.1", vec![("test".to_string(), "value".to_string())]),
        RequestFormat::CurlCommand,
        "curl / --http1.1 -H \"test: value\" -v"
    );

    export_test!(
        format_to_curl_post_request,
        HttpRequest::post("/", "1.1", vec![], Some("")),
        RequestFormat::CurlCommand,
        "curl -X POST / --http1.1 -v"
    );

    export_test!(
        format_to_curl_post_request_with_single_header,
        HttpRequest::post(
            "/",
            "1.1",
            vec![("test".to_string(), "value".to_string())],
            None
        ),
        RequestFormat::CurlCommand,
        "curl -X POST / --http1.1 -H \"test: value\" -v"
    );

    export_test!(
        format_to_curl_post_request_with_single_header_and_body,
        HttpRequest::post(
            "/",
            "1.1",
            vec![("test".to_string(), "value".to_string())],
            Some("testing")
        ),
        RequestFormat::CurlCommand,
        "curl -X POST / --http1.1 -H \"test: value\" -d 'testing' -v"
    );

    export_test!(
        format_to_http_get_request,
        HttpRequest::get("/", "1.1", vec![]),
        RequestFormat::HttpMessage,
        "GET / HTTP/1.1\n"
    );

    export_test!(
        format_to_http_post_request,
        HttpRequest::post("/", "1.1", vec![], Some("[1, 2, 3]\n")),
        RequestFormat::HttpMessage,
        "POST / HTTP/1.1\n\n[1, 2, 3]\n"
    );

    export_response_test!(
        format_response_to_http,
        HttpResponse {
            http_version: HttpVersion::one_point_one(),
            status_code: HttpStatusCode::new(200),
            status_text: "OK".into(),
            headers: vec![],
            body: Some("".to_owned())
        },
        ResponseFormat::HttpMessage,
        "HTTP/1.1 200 OK\n"
    );

    export_response_test!(
        format_response_to_http_with_headers,
        HttpResponse {
            http_version: HttpVersion::one_point_one(),
            status_code: HttpStatusCode::new(200),
            status_text: "OK".into(),
            headers: vec![
                ("x-value".to_string(), "123".to_string()),
                ("content-type".to_string(), "application/json".to_string())
            ],
            body: Some("".to_owned())
        },
        ResponseFormat::HttpMessage,
        "HTTP/1.1 200 OK\nx-value: 123\ncontent-type: application/json\n"
    );

    export_response_test!(
        format_response_to_return_the_body,
        HttpResponse {
            http_version: HttpVersion::one_point_one(),
            status_code: HttpStatusCode::new(200),
            status_text: "OK".into(),
            headers: vec![],
            body: Some("response body\n".to_owned())
        },
        ResponseFormat::Body,
        "response body\n"
    );
}
