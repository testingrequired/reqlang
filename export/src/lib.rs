use std::{fmt::Display, str::FromStr};

use serde::{Deserialize, Serialize};
use types::http::HttpRequest;

/// Supported formats to export request files to
#[derive(Copy, Clone, PartialEq, Eq, Debug, Serialize, Deserialize, Default)]
pub enum Format {
    /// Export as an HTTP Request message
    #[default]
    HttpMessage,
    /// Export as a curl command
    CurlCommand,
    /// Export as a JSON object
    Json,
}

impl Display for Format {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Format::HttpMessage => write!(f, "http"),
            Format::CurlCommand => write!(f, "curl"),
            Format::Json => write!(f, "json"),
        }
    }
}

impl FromStr for Format {
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

/// Export an [HttpRequest] in a specified [Format].
pub fn export(request: &HttpRequest, format: Format) -> String {
    match format {
        // HTTP Request message
        Format::HttpMessage => {
            format!("{}", request)
        }
        // Curl command
        Format::CurlCommand => {
            let request_verb_flag = match request.verb.0.as_str() {
                "GET" => String::new(),
                verb => format!("-X {} ", verb),
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
        Format::Json => serde_json::to_string_pretty(request).unwrap(),
    }
}

#[cfg(test)]
mod test {
    use types::http::HttpRequest;

    use crate::export;

    macro_rules! export_test {
        ($test_name:ident, $request:expr, $format:expr, $expected:expr) => {
            #[test]
            fn $test_name() {
                let actual = export(&$request, $format);
                assert_eq!($expected, actual);
            }
        };
    }

    export_test!(
        format_to_curl_get_request,
        HttpRequest::get("/", "1.1", vec![]),
        crate::Format::CurlCommand,
        "curl / --http1.1 -v"
    );

    export_test!(
        format_to_curl_get_request_with_single_header,
        HttpRequest::get("/", "1.1", vec![("test".to_string(), "value".to_string())]),
        crate::Format::CurlCommand,
        "curl / --http1.1 -H \"test: value\" -v"
    );

    export_test!(
        format_to_curl_post_request,
        HttpRequest::post("/", "1.1", vec![], Some("")),
        crate::Format::CurlCommand,
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
        crate::Format::CurlCommand,
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
        crate::Format::CurlCommand,
        "curl -X POST / --http1.1 -H \"test: value\" -d 'testing' -v"
    );

    export_test!(
        format_to_http_get_request,
        HttpRequest::get("/", "1.1", vec![]),
        crate::Format::HttpMessage,
        "GET / HTTP/1.1\n"
    );

    export_test!(
        format_to_http_post_request,
        HttpRequest::post("/", "1.1", vec![], Some("[1, 2, 3]\n")),
        crate::Format::HttpMessage,
        "POST / HTTP/1.1\n\n[1, 2, 3]\n"
    );
}
