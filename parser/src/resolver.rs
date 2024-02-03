use std::collections::HashMap;

use errors::ReqlangError;
use span::{Spanned, NO_SPAN};
use types::{
    ResolvedRequestFile, ResolvedRequestFileConfig, UnresolvedRequestFile,
    UnresolvedRequestFileConfig,
};

/// Resolve env vars, prompts and secrets in a request file
pub struct RequestFileResolver {}

impl RequestFileResolver {
    pub fn new() -> Self {
        Self {}
    }

    pub fn resolve_request_file(
        reqfile: &UnresolvedRequestFile,
        env: &str,
        prompts: &HashMap<String, String>,
        secrets: &HashMap<String, String>,
    ) -> Result<ResolvedRequestFile, Vec<Spanned<ReqlangError>>> {
        RequestFileResolver::new().resolve(reqfile, env, prompts, secrets)
    }

    pub fn resolve(
        &self,
        reqfile: &UnresolvedRequestFile,
        env: &str,
        prompts: &HashMap<String, String>,
        secrets: &HashMap<String, String>,
    ) -> Result<ResolvedRequestFile, Vec<Spanned<ReqlangError>>> {
        let env_names = reqfile.env_names();

        if let Some((config, span)) = &reqfile.config {
            if !env_names.contains(&&env.to_owned()) {
                return Err(vec![(
                    ReqlangError::ResolverError(errors::ResolverError::InvalidEnvError(
                        env.to_string(),
                    )),
                    span.clone(),
                )]);
            }

            let mut missing_inputs = vec![];

            if let Some(config_prompts) = &config.prompts {
                let keys = config_prompts.keys();

                for key in keys {
                    if !prompts.contains_key(key) {
                        missing_inputs.push((
                            ReqlangError::ResolverError(
                                errors::ResolverError::PromptValueNotPassed(key.to_string()),
                            ),
                            NO_SPAN,
                        ));
                    }
                }
            }

            if let Some(config_secrets) = &config.secrets {
                for config_secret in config_secrets {
                    if !secrets.contains_key(config_secret) {
                        missing_inputs.push((
                            ReqlangError::ResolverError(
                                errors::ResolverError::SecretValueNotPassed(
                                    config_secret.to_string(),
                                ),
                            ),
                            NO_SPAN,
                        ));
                    }
                }
            }

            if !missing_inputs.is_empty() {
                return Err(missing_inputs);
            }
        }

        Ok(ResolvedRequestFile {
            config: (
                ResolvedRequestFileConfig {
                    env: env.to_string(),
                    vars: self.resolve_vars_from_envs(reqfile, env),
                    prompts: prompts.clone(),
                    secrets: secrets.clone(),
                },
                reqfile.config.as_ref().map_or(NO_SPAN, |x| x.1.clone()),
            ),
            request: reqfile.request.clone(),
            response: reqfile.response.clone(),
            refs: reqfile.refs.clone(),
        })
    }

    fn resolve_vars_from_envs(
        &self,
        reqfile: &UnresolvedRequestFile,
        env: &str,
    ) -> HashMap<String, String> {
        let vars = reqfile
            .config
            .clone()
            .unwrap_or((UnresolvedRequestFileConfig::default(), NO_SPAN))
            .clone()
            .0
            .envs
            .unwrap_or_default()
            .get(env)
            .unwrap_or(&HashMap::new())
            .clone();

        vars
    }
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use errors::ReqlangError;
    use span::NO_SPAN;
    use types::{ReferenceType, Request, ResolvedRequestFile, ResolvedRequestFileConfig, Response};

    use crate::{parser::RequestFileParser, resolver::RequestFileResolver};

    macro_rules! resolver_test {
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

                assert_eq!(resolved_reqfile, $result);
            }
        };
    }

    resolver_test!(
        prompt_value_not_passed,
        concat!(
            "---\n",
            "POST /?query={{?query_value}} HTTP/1.1\n",
            "\n",
            "---\n",
            "---\n",
            "[prompts]\n",
            "query_value = \"\"\n",
            "\n",
            "[envs]\n",
            "[envs.dev]\n",
            "\n",
            "---\n"
        ),
        "dev",
        HashMap::new(),
        HashMap::new(),
        Err(vec![(
            ReqlangError::ResolverError(errors::ResolverError::PromptValueNotPassed(
                "query_value".to_string()
            )),
            NO_SPAN
        )])
    );

    resolver_test!(
        secret_value_not_passed,
        concat!(
            "---\n",
            "POST /?query={{!query_value}} HTTP/1.1\n",
            "\n",
            "---\n",
            "---\n",
            "secrets = [\"query_value\"]\n",
            "\n",
            "[envs]\n",
            "[envs.dev]\n",
            "\n",
            "---\n"
        ),
        "dev",
        HashMap::new(),
        HashMap::new(),
        Err(vec![(
            ReqlangError::ResolverError(errors::ResolverError::SecretValueNotPassed(
                "query_value".to_string()
            )),
            NO_SPAN
        )])
    );

    resolver_test!(
        full_request_file_dev,
        concat!(
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
                    secrets: HashMap::from([("api_key".to_string(), "api_key_value".to_string())])
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
        })
    );

    resolver_test!(
        full_request_file_prod,
        concat!(
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
        ),
        "prod",
        HashMap::from([
            ("test_value".to_string(), "test_value_value".to_string()),
            (
                "expected_response_body".to_string(),
                "expected_response_body_value".to_string()
            )
        ]),
        HashMap::from([("api_key".to_string(), "api_key_value".to_string())]),
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
                    env: "prod".to_string(),
                    vars: HashMap::from([("query_value".to_string(), "prod_value".to_string())]),
                    prompts: HashMap::from([
                        ("test_value".to_string(), "test_value_value".to_string()),
                        (
                            "expected_response_body".to_string(),
                            "expected_response_body_value".to_string()
                        )
                    ]),
                    secrets: HashMap::from([("api_key".to_string(), "api_key_value".to_string())])
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
        })
    );

    resolver_test!(
        full_request_file_invalid_env,
        concat!(
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
        ),
        "invalid_env",
        HashMap::from([("test_value".to_string(), "test_value_value".to_string())]),
        HashMap::from([("api_key".to_string(), "api_key_value".to_string())]),
        Err(vec![(
            ReqlangError::ResolverError(errors::ResolverError::InvalidEnvError(
                "invalid_env".to_string()
            )),
            157..342
        )])
    );
}
