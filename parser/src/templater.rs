use errors::ReqlangError;
use span::Spanned;
use types::{ResolvedRequestFile, TemplatedRequestFile};

pub struct RequestFileTemplater {}

impl RequestFileTemplater {
    pub fn new() -> Self {
        Self {}
    }

    /// Template a request file with the resolved values
    pub fn template_reqfile(
        input: &str,
        reqfile: &ResolvedRequestFile,
    ) -> Result<TemplatedRequestFile, Vec<Spanned<ReqlangError>>> {
        let templater = RequestFileTemplater::new();

        templater.template(input, reqfile)
    }

    /// Template a request file with the resolved values
    pub fn template(
        &self,
        _input: &str,
        _reqfile: &ResolvedRequestFile,
    ) -> Result<TemplatedRequestFile, Vec<Spanned<ReqlangError>>> {
        Ok(TemplatedRequestFile::default())
    }
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use types::{Request, Response, TemplatedRequestFile};

    use crate::{
        parser::RequestFileParser, resolver::RequestFileResolver, templater::RequestFileTemplater,
    };

    macro_rules! templater_test {
        ($test_name:ident, $reqfile:expr, $env:expr, $prompts:expr, $secrets:expr, $result:expr) => {
            #[test]
            fn $test_name() {
                let unresolved_reqfile = RequestFileParser::parse_string(&$reqfile);

                assert_eq!(unresolved_reqfile.is_ok(), true);

                let unresolved_reqfile = unresolved_reqfile.unwrap();

                let resolved_reqfile = RequestFileResolver::resolve_request_file(
                    &unresolved_reqfile,
                    $env,
                    &$prompts,
                    &$secrets,
                );

                assert_eq!(resolved_reqfile.is_ok(), true);

                let resolved_reqfile = resolved_reqfile.unwrap();

                let templated_reqfile =
                    RequestFileTemplater::template_reqfile(&$reqfile, &resolved_reqfile);

                assert_eq!(templated_reqfile, $result);
            }
        };
    }

    templater_test!(
        full_request_file,
        concat!(
            "---\n",
            "POST / HTTP/1.1\n",
            "host: {{:base_url}}\n",
            "x-test: {{?test_value}}\n",
            "x-api-key: {{!api_key}}\n",
            "\n",
            "[1, 2, 3]\n",
            "\n",
            "---\n",
            "HTTP/1.1 200 OK\n",
            "\n",
            "{{?expected_response_body}}\n",
            "\n",
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
            "test_value = \"\"\n",
            "expected_response_body = \"\"\n",
            "\n",
            "---\n"
        ),
        "dev",
        HashMap::from([
            ("test_value".to_string(), "test_value_value".to_string()),
            (
                "expected_response_body".to_string(),
                "expected_response_body_value".to_string()
            )
        ]),
        HashMap::from([("api_key".to_string(), "api_key_value".to_string())]),
        Ok(TemplatedRequestFile {
            request: Request {
                verb: "POST".to_string(),
                target: "/".to_string(),
                http_version: "1.1".to_string(),
                headers: HashMap::from([
                    ("host".to_string(), "https://dev.example.com".to_string()),
                    ("x-test".to_string(), "test_value_value".to_string()),
                    ("x-api-key".to_string(), "api_key_value".to_string()),
                ]),
                body: Some("[1, 2, 3]\n\n".to_string())
            },
            response: Some(Response {
                http_version: "1.1".to_string(),
                status_code: "200".to_string(),
                status_text: "OK".to_string(),
                headers: HashMap::new(),
                body: Some("expected_response_body_value\n\n".to_string())
            }),
        })
    );
}
