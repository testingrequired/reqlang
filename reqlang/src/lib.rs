pub mod assert_response;
pub mod ast;
pub mod diagnostics;
pub mod errors;
pub mod export;
pub mod extract_codeblocks;
pub mod fetch;
pub mod parser;
pub mod prelude;
pub mod span;
pub mod str_idxpos;
pub mod templater;
pub mod types;

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use pretty_assertions::assert_eq;

    use crate::{
        ast::Ast,
        parser::parse,
        templater::template,
        types::{
            ParsedConfig, ParsedConfigPrompt, ParsedRequestFile, ReferenceType,
            TemplatedRequestFile,
            http::{HttpRequest, HttpResponse, HttpStatusCode},
        },
    };

    #[test]
    fn parse_full_request_file() {
        let ast = Ast::from(textwrap::dedent(
            "
            ```%config
            vars = [\"query_value\"]
            secrets = [\"api_key\"]

            [envs.dev]
            query_value = \"dev_value\"
            [envs.prod]
            query_value = \"prod_value\"

            [[prompts]]
            name = \"test_value\"

            [[prompts]]
            name = \"expected_response_body\"

            ```

            ```%request
            POST /?query={{:query_value}} HTTP/1.1
            x-test: {{?test_value}}
            x-api-key: {{!api_key}}

            [1, 2, 3]
            ```

            ```%response
            HTTP/1.1 200 OK

            {{?expected_response_body}}

            ```
            ",
        ));

        let reqfile = parse(&ast);

        assert_eq!(
            Ok(ParsedRequestFile {
                request: (
                    HttpRequest {
                        verb: "POST".into(),
                        target: "/?query={{:query_value}}".to_string(),
                        http_version: "1.1".into(),
                        headers: vec![
                            ("x-test".to_string(), "{{?test_value}}".to_string()),
                            ("x-api-key".to_string(), "{{!api_key}}".to_string()),
                        ],
                        body: Some("[1, 2, 3]\n\n".to_string())
                    },
                    230..327
                ),
                response: Some((
                    HttpResponse {
                        http_version: "1.1".into(),
                        status_code: HttpStatusCode::new(200),
                        status_text: "OK".to_string(),
                        headers: vec![],
                        body: Some("{{?expected_response_body}}\n\n\n".to_string())
                    },
                    346..391
                )),
                config: Some((
                    ParsedConfig {
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
                        prompts: Some(vec![
                            ParsedConfigPrompt {
                                name: "test_value".to_string(),
                                description: None,
                                default: None,
                            },
                            ParsedConfigPrompt {
                                name: "expected_response_body".to_string(),
                                description: None,
                                default: None,
                            }
                        ]),
                        secrets: Some(vec!["api_key".to_string()]),
                        auth: None
                    },
                    12..212
                )),
                refs: vec![
                    (ReferenceType::Variable("query_value".to_string()), 230..327),
                    (ReferenceType::Prompt("test_value".to_string()), 230..327),
                    (ReferenceType::Secret("api_key".to_string()), 230..327),
                    (
                        ReferenceType::Prompt("expected_response_body".to_string()),
                        346..391
                    )
                ],
            }),
            reqfile
        );
    }

    #[test]
    fn template_full_request_file() {
        let templated_reqfile = template(
            &textwrap::dedent(
                "
                ```%config
                vars = [\"query_value\"]
                secrets = [\"api_key\"]

                [envs.dev]
                query_value = \"dev_value\"
                [envs.prod]
                query_value = \"prod_value\"

                [[prompts]]
                name = \"test_value\"

                [[prompts]]
                name = \"expected_response_body\"

                ```

                ```%request
                POST /?query={{:query_value}} HTTP/1.1
                x-test: {{?test_value}}
                x-api-key: {{!api_key}}

                [1, 2, 3]
                ```

                ```%response
                HTTP/1.1 200 OK

                {{?expected_response_body}}

                ```
                ",
            ),
            Some("dev"),
            &HashMap::from([
                ("test_value".to_string(), "test_value_value".to_string()),
                (
                    "expected_response_body".to_string(),
                    "expected_response_body_value".to_string(),
                ),
            ]),
            &HashMap::from([("api_key".to_string(), "api_key_value".to_string())]),
            &HashMap::default(),
        );

        assert_eq!(
            Ok(TemplatedRequestFile {
                request: HttpRequest {
                    verb: "POST".into(),
                    target: "/?query=dev_value".to_string(),
                    http_version: "1.1".into(),
                    headers: vec![
                        ("x-test".to_string(), "test_value_value".to_string()),
                        ("x-api-key".to_string(), "api_key_value".to_string()),
                    ],
                    body: Some("[1, 2, 3]\n\n".to_string())
                },
                response: Some(HttpResponse {
                    http_version: "1.1".into(),
                    status_code: HttpStatusCode::new(200),
                    status_text: "OK".to_string(),
                    headers: vec![],
                    body: Some("expected_response_body_value\n\n\n".to_string())
                }),
            }),
            templated_reqfile
        );
    }
}
