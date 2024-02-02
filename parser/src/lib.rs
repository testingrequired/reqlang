use std::collections::HashMap;

use errors::ReqlangError;
use export::Format;
use parser::RequestFileParser;
use resolver::RequestFileResolver;
use span::Spanned;
use templater::RequestFileTemplater;
use types::{ResolvedRequestFile, TemplatedRequestFile, UnresolvedRequestFile};

mod parser;
mod resolver;
mod templater;

pub const TEMPLATE_REFERENCE_PATTERN: &str = r"\{\{([:?!]{1})([a-zA-Z][_a-zA-Z]+)\}\}";

/// Parse a string in to a request file
pub fn parse(input: &str) -> Result<UnresolvedRequestFile, Vec<Spanned<ReqlangError>>> {
    RequestFileParser::parse_string(input)
}

/// Parse a string in to a request file and resolve template values
pub fn resolve(
    input: &str,
    env: &str,
    prompts: &HashMap<String, String>,
    secrets: &HashMap<String, String>,
) -> Result<ResolvedRequestFile, Vec<Spanned<ReqlangError>>> {
    let reqfile = RequestFileParser::parse_string(input);

    RequestFileResolver::resolve_request_file(&reqfile?, env, prompts, secrets)
}

/// Parse a string in to a request file, resolve values, and template the request/response
pub fn template(
    input: &str,
    env: &str,
    prompts: &HashMap<String, String>,
    secrets: &HashMap<String, String>,
) -> Result<TemplatedRequestFile, Vec<Spanned<ReqlangError>>> {
    let reqfile = RequestFileParser::parse_string(input);

    let reqfile = RequestFileResolver::resolve_request_file(&reqfile?, env, prompts, secrets);

    RequestFileTemplater::template_reqfile(input, &reqfile?)
}

/// Export a request file in to another format
pub fn export(
    input: &str,
    env: &str,
    prompts: &HashMap<String, String>,
    secrets: &HashMap<String, String>,
    format: Format,
) -> Result<String, Vec<Spanned<ReqlangError>>> {
    let reqfile = RequestFileParser::parse_string(input);

    let reqfile = RequestFileResolver::resolve_request_file(&reqfile?, env, prompts, secrets);

    let reqfile = RequestFileTemplater::template_reqfile(input, &reqfile?).unwrap();

    Ok(export::export(&reqfile.request, format))
}

#[cfg(test)]
mod parserlib {
    use std::collections::HashMap;

    use types::{
        ReferenceType, Request, ResolvedRequestFile, ResolvedRequestFileConfig, Response,
        TemplatedRequestFile, UnresolvedRequestFile, UnresolvedRequestFileConfig,
    };

    use crate::{parse, resolve, template};

    const REQFILE_STRING: &str = concat!(
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
        "\n",
        "---\n"
    );

    #[test]
    fn parse_full_request_file() {
        let reqfile = parse(&REQFILE_STRING);

        assert_eq!(
            Ok(UnresolvedRequestFile {
                request: (
                    Request {
                        verb: "POST".to_string(),
                        target: "/?query={{:query_value}}".to_string(),
                        http_version: "1.1".to_string(),
                        headers: HashMap::from([
                            ("x-test".to_string(), "{{?test_value}}".to_string()),
                            ("x-api-key".to_string(), "{{!api_key}}".to_string()),
                        ]),
                        body: Some("[1, 2, 3]\n\n".to_string())
                    },
                    4..103
                ),
                response: Some((
                    Response {
                        http_version: "1.1".to_string(),
                        status_code: "200".to_string(),
                        status_text: "OK".to_string(),
                        headers: HashMap::new(),
                        body: Some("{{?expected_response_body}}\n\n".to_string())
                    },
                    107..153
                )),
                config: Some((
                    UnresolvedRequestFileConfig {
                        vars: Some(vec!["query_value".to_string()]),
                        envs: Some(HashMap::from([
                            (
                                "dev".to_string(),
                                HashMap::from([(
                                    "query_value".to_string(),
                                    "dev_value".to_string()
                                )])
                            ),
                            (
                                "prod".to_string(),
                                HashMap::from([(
                                    "query_value".to_string(),
                                    "prod_value".to_string()
                                )])
                            ),
                        ])),
                        prompts: Some(HashMap::from([
                            ("test_value".to_string(), Some("".to_string())),
                            ("expected_response_body".to_string(), Some("".to_string()))
                        ])),
                        secrets: Some(vec!["api_key".to_string()])
                    },
                    157..342
                )),
                refs: vec![
                    (ReferenceType::Variable("query_value".to_string()), 4..103),
                    (ReferenceType::Prompt("test_value".to_string()), 4..103),
                    (ReferenceType::Secret("api_key".to_string()), 4..103),
                    (
                        ReferenceType::Prompt("expected_response_body".to_string()),
                        107..153
                    )
                ],
            }),
            reqfile
        );
    }

    #[test]
    fn resolve_full_request_file() {
        let resolved_reqfile = resolve(
            &REQFILE_STRING,
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
                        headers: HashMap::from([
                            ("x-test".to_string(), "{{?test_value}}".to_string()),
                            ("x-api-key".to_string(), "{{!api_key}}".to_string()),
                        ]),
                        body: Some("[1, 2, 3]\n\n".to_string())
                    },
                    4..103
                ),
                response: Some((
                    Response {
                        http_version: "1.1".to_string(),
                        status_code: "200".to_string(),
                        status_text: "OK".to_string(),
                        headers: HashMap::new(),
                        body: Some("{{?expected_response_body}}\n\n".to_string())
                    },
                    107..153
                )),
                config: (
                    ResolvedRequestFileConfig {
                        env: "dev".to_string(),
                        vars: HashMap::from([("query_value".to_string(), "dev_value".to_string())]),
                        prompts: HashMap::from([
                            ("test_value".to_string(), "test_value_value".to_string()),
                            (
                                "expected_response_body".to_string(),
                                "expected_response_body_value".to_string()
                            )
                        ]),
                        secrets: HashMap::from([(
                            "api_key".to_string(),
                            "api_key_value".to_string()
                        )])
                    },
                    157..342
                ),
                refs: vec![
                    (ReferenceType::Variable("query_value".to_string()), 4..103),
                    (ReferenceType::Prompt("test_value".to_string()), 4..103),
                    (ReferenceType::Secret("api_key".to_string()), 4..103),
                    (
                        ReferenceType::Prompt("expected_response_body".to_string()),
                        107..153
                    )
                ],
            }),
            resolved_reqfile
        );
    }

    #[test]
    fn template_full_request_file() {
        let templated_reqfile = template(
            &REQFILE_STRING,
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
            Ok(TemplatedRequestFile {
                request: Request {
                    verb: "POST".to_string(),
                    target: "/?query=dev_value".to_string(),
                    http_version: "1.1".to_string(),
                    headers: HashMap::from([
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
            }),
            templated_reqfile
        );
    }
}
