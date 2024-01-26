use std::collections::HashMap;

use errors::ReqlangError;
use parser::RequestFileParser;
use resolver::RequestFileResolver;
use span::Spanned;
use types::ResolvedRequestFile;

mod parser;
mod resolver;

/// Parse a string in to a request file and resolve template values
pub fn parse(
    input: &str,
    env: &str,
    prompts: HashMap<String, String>,
    secrets: HashMap<String, String>,
) -> Result<ResolvedRequestFile, Vec<Spanned<ReqlangError>>> {
    let reqfile = RequestFileParser::parse_string(input);

    if let Err(err) = reqfile {
        return Err(err);
    }

    let reqfile = reqfile.unwrap();

    RequestFileResolver::resolve_request_file(&reqfile, env, &prompts, &secrets)
}

#[cfg(test)]
mod parserlib {
    use std::collections::HashMap;

    use types::{ReferenceType, Request, ResolvedRequestFile, ResolvedRequestFileConfig, Response};

    use crate::parse;

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
        );

        let resolved_reqfile = parse(
            &reqfile,
            "dev",
            HashMap::from([("test_value".to_string(), "test_value_value".to_string())]),
            HashMap::from([("api_key".to_string(), "api_key_value".to_string())]),
        );

        assert_eq!(
            Ok(ResolvedRequestFile {
                request: (
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
                    4..100
                ),
                response: Some((
                    Response {
                        http_version: "1.1".to_string(),
                        status_code: "200".to_string(),
                        status_text: "OK".to_string(),
                        headers: HashMap::new(),
                        body: Some("{{?expected_response_body}}\n\n".to_string())
                    },
                    104..150
                )),
                config: (
                    ResolvedRequestFileConfig {
                        env: "dev".to_string(),
                        vars: HashMap::from([(
                            "base_url".to_string(),
                            "https://dev.example.com".to_string()
                        )]),
                        prompts: HashMap::from([(
                            "test_value".to_string(),
                            "test_value_value".to_string()
                        )]),
                        secrets: HashMap::from([(
                            "api_key".to_string(),
                            "api_key_value".to_string()
                        )])
                    },
                    154..353
                ),
                request_refs: vec![
                    (ReferenceType::Variable("base_url".to_string()), 4..100),
                    (ReferenceType::Prompt("test_value".to_string()), 4..100),
                    (ReferenceType::Secret("api_key".to_string()), 4..100)
                ],
                response_refs: vec![(
                    ReferenceType::Prompt("expected_response_body".to_string()),
                    104..150
                )],
                config_refs: vec![],
            }),
            resolved_reqfile
        );
    }
}
