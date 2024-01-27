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
            request_refs: reqfile.request_refs.clone(),
            response_refs: reqfile.response_refs.clone(),
            config_refs: reqfile.config_refs.clone(),
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
        HashMap::from([("test_value".to_string(), "test_value_value".to_string())]),
        HashMap::from([("api_key".to_string(), "api_key_value".to_string())]),
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
                    secrets: HashMap::from([("api_key".to_string(), "api_key_value".to_string())])
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
        })
    );
}
