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

        let request = documents.get(1).map(|x| x.trim().to_string()).unwrap();
        let response = documents
            .get(2)
            .map(|x| x.trim().to_string())
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

        let _ = req.parse(_request.as_bytes());

        Request {
            verb: req.method.unwrap().to_string(),
            target: req.path.unwrap().to_string(),
            http_version: format!("1.{}", req.version.unwrap().to_string()),
            ..Default::default()
        }
    }

    fn parse_response(&self, _response: Option<String>) -> Option<Response> {
        None
    }
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use types::{Request, UnresolvedRequestFileConfig};

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
            "GET / HTTP/1.1\n",
            "host: {{:base_url}}\n",
            "x-test: {{?test_value}}\n",
            "x-api-key: {{!api_key}}\n",
            "---\n",
            "HTTP/1.1 200 OK\n",
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

        let mut expected_headers = HashMap::new();
        expected_headers.insert("host".to_string(), "{{:base_url}}".to_string());
        expected_headers.insert("x-test".to_string(), "{{?test_value}}".to_string());
        expected_headers.insert("x-api-key".to_string(), "{{!api_key}}".to_string());

        assert_eq!(
            Request {
                verb: "GET".to_string(),
                target: "/".to_string(),
                http_version: "1.1".to_string(),
                headers: expected_headers,
                body: None
            },
            unresolved_reqfile.request
        );

        assert_eq!(None, unresolved_reqfile.response);

        let expected_vars = vec!["base_url".to_string()];

        let mut expected_envs = HashMap::new();

        let mut dev_env = HashMap::new();

        dev_env.insert("base_url".to_string(), "http://dev.example.com".to_string());

        let mut prod_env = HashMap::new();

        prod_env.insert("base_url".to_string(), "http://example.com".to_string());

        expected_envs.insert("dev".to_string(), dev_env);
        expected_envs.insert("prod".to_string(), prod_env);

        let mut expected_prompts = HashMap::new();
        expected_prompts.insert("test_value".to_string(), None);

        let expected_secrets = vec!["api_key".to_string()];

        assert_eq!(
            Some(UnresolvedRequestFileConfig {
                vars: expected_vars,
                envs: expected_envs,
                prompts: expected_prompts,
                secrets: expected_secrets
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
