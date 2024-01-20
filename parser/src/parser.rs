use std::collections::HashMap;
use types::{Request, Response, UnresolvedRequestFile, UnresolvedRequestFileConfig};

pub struct RequestFileParser {}

impl RequestFileParser {
    pub fn new() -> Self {
        Self {}
    }

    pub fn parse_string(input: &str) -> Result<UnresolvedRequestFile, &'static str> {
        RequestFileParser::new().parse(input)
    }

    /// Parse a string in to an request file with unresolved template values.
    pub fn parse(&self, input: &str) -> Result<UnresolvedRequestFile, &'static str> {
        let split = self.split(input).and_then(|x| {
            Ok(UnresolvedRequestFile {
                config: self.parse_config(x.config),
                request: self.parse_request(x.request),
                response: self.parse_response(x.response),
            })
        });

        split
    }

    fn split(&self, input: &str) -> Result<RequestFileSplitUp, &'static str> {
        if input.is_empty() {
            return Err("Request file is an empty file");
        }

        let documents: Vec<&str> = input.split(DELIMITER).collect();

        if documents.len() < 2 {
            return Err("Request file has no document dividers");
        }

        if documents.len() > 5 {
            return Err("Request file has too many document dividers");
        }

        let request = documents.get(1).map(|x| x.to_string()).unwrap();
        let response = documents
            .get(2)
            .map(|x| x.trim_start().to_string())
            .filter(|x| !x.is_empty());
        let config = documents
            .get(3)
            .map(|x| x.trim().to_string())
            .filter(|x| !x.is_empty());

        Ok(RequestFileSplitUp {
            request,
            response,
            config,
        })
    }

    fn parse_config(&self, _config: Option<String>) -> Option<UnresolvedRequestFileConfig> {
        _config.map(|c| toml::from_str(&c).unwrap())
    }

    fn parse_request(&self, _request: String) -> Request {
        let mut headers = [httparse::EMPTY_HEADER; 64];
        let mut req = httparse::Request::new(&mut headers);

        let size_minus_body = match req.parse(_request.as_bytes()).unwrap() {
            httparse::Status::Complete(x) => x,
            httparse::Status::Partial => 0,
        };

        let body = &_request[size_minus_body..];

        let mut mapped_headers = HashMap::new();

        req.headers
            .into_iter()
            .filter(|x| !x.name.is_empty())
            .for_each(|x| {
                mapped_headers.insert(
                    x.name.to_string(),
                    std::str::from_utf8(x.value).unwrap().to_string(),
                );
            });

        Request {
            verb: req.method.unwrap().to_string(),
            target: req.path.unwrap().to_string(),
            http_version: format!("1.{}", req.version.unwrap().to_string()),
            headers: mapped_headers,
            body: Some(body.to_string()),
        }
    }

    fn parse_response(&self, _response: Option<String>) -> Option<Response> {
        let mut headers = [httparse::EMPTY_HEADER; 64];
        let mut res = httparse::Response::new(&mut headers);

        let response = match _response {
            Some(x) => x,
            None => return None,
        };

        let size_minus_body = match res.parse(response.as_bytes()).unwrap() {
            httparse::Status::Complete(x) => x,
            httparse::Status::Partial => 0,
        };

        let body = &response[size_minus_body..];

        let mut mapped_headers = HashMap::new();

        res.headers
            .into_iter()
            .filter(|x| !x.name.is_empty())
            .for_each(|x| {
                mapped_headers.insert(
                    x.name.to_string(),
                    std::str::from_utf8(x.value).unwrap().to_string(),
                );
            });

        Some(Response {
            http_version: format!("1.{}", res.version.unwrap().to_string()),
            status_code: res.code.unwrap().to_string(),
            status_text: res.reason.unwrap().to_string(),
            headers: mapped_headers,
            body: Some(body.to_string()),
        })
    }
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use types::{Request, Response, UnresolvedRequestFileConfig};

    use crate::parser::RequestFileParser;

    macro_rules! splitter_test {
        ($test_name:ident, $reqfile:expr, $result:expr) => {
            #[test]
            fn $test_name() {
                assert_eq!($result, RequestFileParser::parse_string(&$reqfile));
            }
        };
    }

    splitter_test!(empty, "", Err("Request file is an empty file"));

    splitter_test!(
        no_doc_dividers,
        "GET http://example.com HTTP/1.1\n",
        Err("Request file has no document dividers")
    );

    splitter_test!(
        too_many_doc_dividers,
        concat!(
            "---\n",
            "GET http://example.com HTTP/1.1\n",
            "---\n",
            "---\n",
            "---\n",
            "---\n"
        ),
        Err("Request file has too many document dividers")
    );

    #[test]
    fn full_request_file() {
        let reqfile = concat!(
            "---\n",
            "POST / HTTP/1.1\n",
            "host: {{:base_url}}\n",
            "x-test: {{?test_value}}\n",
            "x-api-key: {{!api_key}}\n",
            "\n",
            "[1, 2, 3]\n",
            "\n",
            "---\n",
            "HTTP/1.1 200 OK\n\n",
            "---\n",
            "vars = [\"base_url\"]\n",
            "secrets = [\"api_key\"]",
            "\n",
            "[envs]\n",
            "[envs.dev]\n",
            "base_url = \"https://dev.example.com\"\n",
            "\n",
            "[envs.prod]\n",
            "base_url = \"https://example.com\"\n",
            "\n",
            "[prompts]\n",
            "test_value = \"\"",
            "\n",
            "---\n"
        );

        let parser = RequestFileParser::new();

        let unresolved_reqfile = parser.parse(&reqfile);
        assert_eq!(unresolved_reqfile.is_ok(), true);
        let unresolved_reqfile = unresolved_reqfile.unwrap();

        assert_eq!(
            Request {
                verb: "POST".to_string(),
                target: "/".to_string(),
                http_version: "1.1".to_string(),
                headers: HashMap::from([
                    ("host".to_string(), "{{:base_url}}".to_string()),
                    ("x-test".to_string(), "{{?test_value}}".to_string()),
                    ("x-api-key".to_string(), "{{!api_key}}".to_string()),
                ]),
                body: Some("[1, 2, 3]\n\n".to_string())
            },
            unresolved_reqfile.request
        );

        assert_eq!(
            Some(Response {
                http_version: "1.1".to_string(),
                status_code: "200".to_string(),
                status_text: "OK".to_string(),
                headers: HashMap::new(),
                body: Some("".to_string())
            }),
            unresolved_reqfile.response
        );

        assert_eq!(
            Some(UnresolvedRequestFileConfig {
                vars: Some(vec!["base_url".to_string()]),
                envs: Some(HashMap::from([
                    (
                        "dev".to_string(),
                        HashMap::from([(
                            "base_url".to_string(),
                            "https://dev.example.com".to_string()
                        )])
                    ),
                    (
                        "prod".to_string(),
                        HashMap::from([(
                            "base_url".to_string(),
                            "https://example.com".to_string()
                        )])
                    ),
                ])),
                prompts: Some(HashMap::from([(
                    "test_value".to_string(),
                    Some("".to_string())
                )])),
                secrets: Some(vec!["api_key".to_string()])
            }),
            unresolved_reqfile.config
        );
    }
}

/// Delimiter used to split request files
const DELIMITER: &str = "---";

struct RequestFileSplitUp {
    request: String,
    response: Option<String>,
    config: Option<String>,
}
