use std::{fmt::Display, str::FromStr};

use serde::{Deserialize, Serialize};
use types::Request;

#[derive(Copy, Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Format {
    Http,
    Curl,
}

impl Display for Format {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Format::Http => write!(f, "http"),
            Format::Curl => write!(f, "curl"),
        }
    }
}

impl FromStr for Format {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "http" => Ok(Self::Http),
            "curl" => Ok(Self::Curl),
            _ => Err(format!("Unknown format: {s}")),
        }
    }
}

pub fn export(request: &Request, format: Format) -> String {
    match format {
        Format::Http => {
            format!("{}", request)
        }
        Format::Curl => {
            let verb = if request.verb == "GET" {
                "".to_string()
            } else {
                format!("-X {} ", request.verb)
            };

            let target = &request.target;

            let h = if request.headers.is_empty() {
                None
            } else {
                Some(format!(
                    "{}",
                    &request
                        .headers
                        .clone()
                        .into_iter()
                        .map(|x| format!(r#"-H "{}: {}""#, x.0, x.1))
                        .collect::<Vec<String>>()
                        .join(" ")
                ))
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
                (None, None) => format!(""),
            };

            format!(
                "curl {}{} --http{}{}",
                verb, target, request.http_version, the_rest
            )
        }
    }
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use types::Request;

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
        Request::get("/", "1.1", HashMap::new()),
        crate::Format::Curl,
        "curl / --http1.1"
    );

    export_test!(
        format_to_curl_get_request_with_single_header,
        Request::get(
            "/",
            "1.1",
            HashMap::from([("test".to_string(), "value".to_string())])
        ),
        crate::Format::Curl,
        "curl / --http1.1 -H \"test: value\""
    );

    export_test!(
        format_to_curl_post_request,
        Request::post("/", "1.1", HashMap::new(), Some("")),
        crate::Format::Curl,
        "curl -X POST / --http1.1"
    );

    export_test!(
        format_to_curl_post_request_with_single_header,
        Request::post(
            "/",
            "1.1",
            HashMap::from([("test".to_string(), "value".to_string())]),
            None
        ),
        crate::Format::Curl,
        "curl -X POST / --http1.1 -H \"test: value\""
    );

    export_test!(
        format_to_curl_post_request_with_single_header_and_body,
        Request::post(
            "/",
            "1.1",
            HashMap::from([("test".to_string(), "value".to_string())]),
            Some("testing")
        ),
        crate::Format::Curl,
        "curl -X POST / --http1.1 -H \"test: value\" -d 'testing'"
    );

    export_test!(
        format_to_http_get_request,
        Request::get("/", "1.1", HashMap::new()),
        crate::Format::Http,
        "GET / HTTP/1.1\n"
    );

    export_test!(
        format_to_http_post_request,
        Request::post("/", "1.1", HashMap::new(), Some("[1, 2, 3]\n")),
        crate::Format::Http,
        "POST / HTTP/1.1\n\n[1, 2, 3]\n"
    );
}