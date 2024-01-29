use std::{fmt::Display, str::FromStr};

use types::Request;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Format {
    Http,
    Curl,
    Javascript,
    Powershell,
}

impl Display for Format {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Format::Http => write!(f, "http"),
            Format::Curl => write!(f, "curl"),
            Format::Javascript => write!(f, "javascript"),
            Format::Powershell => write!(f, "powershell"),
        }
    }
}

impl FromStr for Format {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "http" => Ok(Self::Http),
            "curl" => Ok(Self::Curl),
            "javascript" => Ok(Self::Javascript),
            "powershell" => Ok(Self::Powershell),
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
                format!("-X {}", request.verb)
            };

            let target = &request.target;
            let headers: String = request
                .headers
                .iter()
                .map(|x| format!(r#"-H "{}: {}""#, x.0, x.1))
                .collect::<Vec<String>>()
                .join(" ");

            let data = match &request.body {
                Some(body) => match body.is_empty() {
                    true => "".to_string(),
                    false => format!("-d '{body}'"),
                },
                None => "".to_string(),
            };

            format!(
                "curl {} {} --http{} {} {}",
                verb, target, request.http_version, headers, data
            )
        }
        Format::Powershell => {
            let headers: Vec<String> = request
                .headers
                .iter()
                .map(|x| format!(r#"'{}' = '{}'"#, x.0, x.1))
                .collect();

            let header_values = format!("{}", headers.join("; "));

            let header_arg = if headers.is_empty() {
                ""
            } else {
                "-Headers $headers"
            };

            let body_arg = if request.body.is_some() {
                "-Body $body"
            } else {
                ""
            };

            let body_value = &request.body.clone().unwrap_or_default();

            format!(
                "$headers = @{{ {} }}\n$body = '{}'\nInvoke-RestMethod -HttpVersion {} -Uri {} -Method {} {} {}",
                header_values,
                body_value,
                request.http_version,
                request.target,
                request.verb,
                header_arg,
                body_arg
            )
        }
        Format::Javascript => {
            format!("Exporting to javascript isn't support yet")
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
        "curl  / --http1.1  "
    );

    export_test!(
        format_to_powershell_get_request,
        Request::get("/", "1.1", HashMap::new()),
        crate::Format::Powershell,
        concat!(
            "$headers = @{  }\n",
            "$body = ''\n",
            "Invoke-RestMethod -HttpVersion 1.1 -Uri / -Method GET  "
        )
    );

    export_test!(
        format_to_http_get_request,
        Request::get("/", "1.1", HashMap::new()),
        crate::Format::Http,
        "GET / HTTP/1.1\n"
    );
}
