pub use diagnostics;
pub use errors;
pub use export;
pub use parser::parse;
pub use parser::resolve;
pub use parser::template;
pub use span::*;
pub use types::*;

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use types::{
        ReferenceType, Request, ResolvedRequestFile, ResolvedRequestFileConfig, Response,
        TemplatedRequestFile, UnresolvedRequestFile, UnresolvedRequestFileConfig,
    };

    use pretty_assertions::assert_eq;

    use crate::{parse, resolve, template};

    const REQFILE_STRING: &str = concat!(
        "vars = [\"query_value\"]\n",
        "secrets = [\"api_key\"]",
        "\n",
        "[envs]\n",
        "[envs.dev]\n",
        "query_value = \"dev_value\"\n",
        "\n",
        "[envs.prod]\n",
        "query_value = \"prod_value\"\n",
        "\n",
        "[prompts]\n",
        "test_value = \"\"\n",
        "expected_response_body = \"\"\n",
        "---\n",
        "POST /?query={{:query_value}} HTTP/1.1\n",
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
    );

    #[test]
    fn parse_full_request_file() {
        let reqfile = parse(REQFILE_STRING);

        assert_eq!(
            Ok(UnresolvedRequestFile {
                request: (
                    Request {
                        verb: "POST".to_string(),
                        target: "/?query={{:query_value}}".to_string(),
                        http_version: "1.1".to_string(),
                        headers: vec![
                            ("x-test".to_string(), "{{?test_value}}".to_string()),
                            ("x-api-key".to_string(), "{{!api_key}}".to_string()),
                        ],
                        body: Some("[1, 2, 3]\n\n".to_string())
                    },
                    188..287
                ),
                response: Some((
                    Response {
                        http_version: "1.1".to_string(),
                        status_code: "200".to_string(),
                        status_text: "OK".to_string(),
                        headers: HashMap::new(),
                        body: Some("{{?expected_response_body}}\n\n".to_string())
                    },
                    291..337
                )),
                config: Some((
                    UnresolvedRequestFileConfig {
                        vars: Some(vec!["query_value".to_string()]),
                        envs: Some(HashMap::from([
                            (
                                "prod".to_string(),
                                HashMap::from([(
                                    "query_value".to_string(),
                                    "prod_value".to_string()
                                )])
                            ),
                            (
                                "dev".to_string(),
                                HashMap::from([(
                                    "query_value".to_string(),
                                    "dev_value".to_string()
                                )])
                            ),
                        ])),
                        prompts: Some(HashMap::from([
                            ("expected_response_body".to_string(), Some("".to_string())),
                            ("test_value".to_string(), Some("".to_string())),
                        ])),
                        secrets: Some(vec!["api_key".to_string()]),
                        auth: None
                    },
                    0..184
                )),
                refs: vec![
                    (ReferenceType::Variable("query_value".to_string()), 188..287),
                    (ReferenceType::Prompt("test_value".to_string()), 188..287),
                    (ReferenceType::Secret("api_key".to_string()), 188..287),
                    (
                        ReferenceType::Prompt("expected_response_body".to_string()),
                        291..337
                    )
                ],
            }),
            reqfile
        );
    }

    #[test]
    fn resolve_full_request_file() {
        let resolved_reqfile = resolve(
            REQFILE_STRING,
            "dev",
            &HashMap::from([
                ("test_value".to_string(), "test_value_value".to_string()),
                (
                    "expected_response_body".to_string(),
                    "expected_response_body_value".to_string(),
                ),
            ]),
            &HashMap::from([("api_key".to_string(), "api_key_value".to_string())]),
        );

        assert_eq!(
            Ok(ResolvedRequestFile {
                request: (
                    Request {
                        verb: "POST".to_string(),
                        target: "/?query={{:query_value}}".to_string(),
                        http_version: "1.1".to_string(),
                        headers: vec![
                            ("x-test".to_string(), "{{?test_value}}".to_string()),
                            ("x-api-key".to_string(), "{{!api_key}}".to_string()),
                        ],
                        body: Some("[1, 2, 3]\n\n".to_string())
                    },
                    188..287
                ),
                response: Some((
                    Response {
                        http_version: "1.1".to_string(),
                        status_code: "200".to_string(),
                        status_text: "OK".to_string(),
                        headers: HashMap::new(),
                        body: Some("{{?expected_response_body}}\n\n".to_string())
                    },
                    291..337
                )),
                config: (
                    ResolvedRequestFileConfig {
                        env: "dev".to_string(),
                        vars: HashMap::from([("query_value".to_string(), "dev_value".to_string())]),
                        prompts: HashMap::from([
                            (
                                "expected_response_body".to_string(),
                                "expected_response_body_value".to_string()
                            ),
                            ("test_value".to_string(), "test_value_value".to_string()),
                        ]),
                        secrets: HashMap::from([(
                            "api_key".to_string(),
                            "api_key_value".to_string()
                        )]),
                        auth: None
                    },
                    0..184
                ),
                refs: vec![
                    (ReferenceType::Variable("query_value".to_string()), 188..287),
                    (ReferenceType::Prompt("test_value".to_string()), 188..287),
                    (ReferenceType::Secret("api_key".to_string()), 188..287),
                    (
                        ReferenceType::Prompt("expected_response_body".to_string()),
                        291..337
                    )
                ],
            }),
            resolved_reqfile
        );
    }

    #[test]
    fn template_full_request_file() {
        let templated_reqfile = template(
            REQFILE_STRING,
            "dev",
            &HashMap::from([
                ("test_value".to_string(), "test_value_value".to_string()),
                (
                    "expected_response_body".to_string(),
                    "expected_response_body_value".to_string(),
                ),
            ]),
            &HashMap::from([("api_key".to_string(), "api_key_value".to_string())]),
            HashMap::default(),
        );

        assert_eq!(
            Ok(TemplatedRequestFile {
                request: Request {
                    verb: "POST".to_string(),
                    target: "/?query=dev_value".to_string(),
                    http_version: "1.1".to_string(),
                    headers: vec![
                        ("x-test".to_string(), "test_value_value".to_string()),
                        ("x-api-key".to_string(), "api_key_value".to_string()),
                    ],
                    body: Some("[1, 2, 3]\n\n".to_string())
                },
                response: Some(Response {
                    http_version: "1.1".to_string(),
                    status_code: "200".to_string(),
                    status_text: "OK".to_string(),
                    headers: HashMap::new(),
                    body: Some("expected_response_body_value\n\n".to_string())
                }),
            }),
            templated_reqfile
        );
    }
}
