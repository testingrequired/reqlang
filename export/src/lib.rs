use std::{fmt::Display, str::FromStr};

use serde::{Deserialize, Serialize};
use types::http::{HttpRequest, HttpVerb};

#[derive(Copy, Clone, PartialEq, Eq, Debug, Serialize, Deserialize, Default)]
pub enum Format {
    Http,
    Curl,
    #[default]
    CurlScript,
}

impl Display for Format {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Format::Http => write!(f, "http"),
            Format::Curl => write!(f, "curl"),
            Format::CurlScript => write!(f, "curl_script"),
        }
    }
}

impl FromStr for Format {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "http" => Ok(Self::Http),
            "curl" => Ok(Self::Curl),
            "curl_script" => Ok(Self::CurlScript),
            _ => Err(format!("Unknown format: {s}")),
        }
    }
}

pub fn export(request: &HttpRequest, format: Format) -> String {
    match format {
        Format::Http => {
            format!("{}", request)
        }
        Format::Curl => {
            let verb = if request.verb == HttpVerb::get() {
                "".to_string()
            } else {
                format!("-X {} ", request.verb)
            };

            let target = &request.target;

            let h = if request.headers.is_empty() {
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

            let b = request.body.clone().and_then(|x| {
                if x.is_empty() {
                    None
                } else {
                    Some(format!("-d '{x}'"))
                }
            });

            let the_rest = match (&h, &b) {
                (Some(headers), Some(body)) => format!(" {headers} {body}"),
                (Some(headers), None) => format!(" {headers}"),
                (None, Some(body)) => format!(" {body}"),
                (None, None) => String::new(),
            };

            format!(
                "curl {}{} --http{}{} -v",
                verb, target, request.http_version, the_rest
            )
        }
        Format::CurlScript => {
            let shebang = "#!/usr/bin/env bash";
            let curl = export(request, Format::Curl);

            format!("{shebang}\n\n{curl}")
        }
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
        crate::Format::Curl,
        "curl / --http1.1 -v"
    );

    export_test!(
        format_to_curl_get_request_with_single_header,
        HttpRequest::get("/", "1.1", vec![("test".to_string(), "value".to_string())]),
        crate::Format::Curl,
        "curl / --http1.1 -H \"test: value\" -v"
    );

    export_test!(
        format_to_curl_post_request,
        HttpRequest::post("/", "1.1", vec![], Some("")),
        crate::Format::Curl,
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
        crate::Format::Curl,
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
        crate::Format::Curl,
        "curl -X POST / --http1.1 -H \"test: value\" -d 'testing' -v"
    );

    export_test!(
        format_to_http_get_request,
        HttpRequest::get("/", "1.1", vec![]),
        crate::Format::Http,
        "GET / HTTP/1.1\n"
    );

    export_test!(
        format_to_http_post_request,
        HttpRequest::post("/", "1.1", vec![], Some("[1, 2, 3]\n")),
        crate::Format::Http,
        "POST / HTTP/1.1\n\n[1, 2, 3]\n"
    );

    export_test!(
        format_to_curl_script_get_request,
        HttpRequest::get("/", "1.1", vec![]),
        crate::Format::CurlScript,
        "#!/usr/bin/env bash\n\ncurl / --http1.1 -v"
    );
}
