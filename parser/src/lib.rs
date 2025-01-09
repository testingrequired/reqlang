use std::collections::HashMap;

use errors::ReqlangError;
use export::Format;
use span::Spanned;
use templater::template as template_reqfile;
use types::TemplatedRequestFile;

pub use parser::parse;

mod parser;
mod templater;

pub const TEMPLATE_REFERENCE_PATTERN: &str = r"\{\{([:?!@]{1})([a-zA-Z][_a-zA-Z0-9.]+)\}\}";

/// Parse a string in to a request file, resolve values, and template the request/response
pub fn template(
    reqfile_string: &str,
    env: &str,
    prompts: &HashMap<String, String>,
    secrets: &HashMap<String, String>,
    provider_values: HashMap<String, String>,
) -> Result<TemplatedRequestFile, Vec<Spanned<ReqlangError>>> {
    let parsed_reqfile = parse(reqfile_string)?;

    template_reqfile(
        reqfile_string,
        &parsed_reqfile,
        env,
        prompts.clone(),
        secrets.clone(),
        provider_values,
    )
}

/// Export a request file in to another format
pub fn export(
    reqfile_string: &str,
    env: &str,
    prompts: &HashMap<String, String>,
    secrets: &HashMap<String, String>,
    provider_values: HashMap<String, String>,
    format: Format,
) -> Result<String, Vec<Spanned<ReqlangError>>> {
    let templated_reqfile = template(reqfile_string, env, prompts, secrets, provider_values)?;

    Ok(export::export(&templated_reqfile.request, format))
}

#[cfg(test)]
mod parserlib {
    use std::collections::HashMap;

    use types::{
        http::{HttpRequest, HttpResponse, HttpStatusCode},
        ParsedConfig, ParsedRequestFile, ReferenceType, TemplatedRequestFile,
    };

    use pretty_assertions::assert_eq;

    use crate::{parse, template};

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
                    188..287
                ),
                response: Some((
                    HttpResponse {
                        http_version: "1.1".into(),
                        status_code: HttpStatusCode::new(200),
                        status_text: "OK".to_string(),
                        headers: HashMap::new(),
                        body: Some("{{?expected_response_body}}\n\n".to_string())
                    },
                    291..337
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
