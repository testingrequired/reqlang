pub use assert_response;
pub use diagnostics;
pub use errors;
pub use errors::ReqlangError;
pub use export::*;
pub use parser::parse;
pub use parser::template;
pub use reqlang_fetch::*;
pub use span::*;
pub use types::http::*;
pub use types::*;

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use pretty_assertions::assert_eq;

    use crate::{
        parse, template, HttpRequest, HttpResponse, HttpStatusCode, ParsedConfig,
        ParsedRequestFile, ReferenceType, TemplatedRequestFile,
    };

    #[test]
    fn parse_full_request_file() {
        let reqfile = parse(&textwrap::dedent(
            "
            ```%config
            vars = [\"query_value\"]
            secrets = [\"api_key\"]

            [envs.dev]
            query_value = \"dev_value\"
            [envs.prod]
            query_value = \"prod_value\"

            [prompts]
            test_value = \"\"
            expected_response_body = \"\"

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
                    195..308
                ),
                response: Some((
                    HttpResponse {
                        http_version: "1.1".into(),
                        status_code: HttpStatusCode::new(200),
                        status_text: "OK".to_string(),
                        headers: HashMap::new(),
                        body: Some("{{?expected_response_body}}\n\n".to_string())
                    },
                    310..372
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
                        prompts: Some(HashMap::from([
                            ("expected_response_body".to_string(), Some("".to_string())),
                            ("test_value".to_string(), Some("".to_string())),
                        ])),
                        secrets: Some(vec!["api_key".to_string()]),
                        auth: None
                    },
                    1..193
                )),
                refs: vec![
                    (ReferenceType::Variable("query_value".to_string()), 195..308),
                    (ReferenceType::Prompt("test_value".to_string()), 195..308),
                    (ReferenceType::Secret("api_key".to_string()), 195..308),
                    (
                        ReferenceType::Prompt("expected_response_body".to_string()),
                        310..372
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

                [prompts]
                test_value = \"\"
                expected_response_body = \"\"

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
            "dev",
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
                    headers: HashMap::new(),
                    body: Some("expected_response_body_value\n\n".to_string())
                }),
            }),
            templated_reqfile
        );
    }
}
