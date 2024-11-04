use errors::ReqlangError;
use span::Spanned;
use types::{ReferenceType, ResolvedRequestFile, TemplatedRequestFile};

use crate::parser::RequestFileParser;
use crate::split;

use oauth2::basic::BasicClient;
use oauth2::reqwest::http_client;
use oauth2::{AuthUrl, ClientId, ClientSecret, Scope, TokenResponse, TokenUrl};

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
        input: &str,
        reqfile: &ResolvedRequestFile,
    ) -> Result<TemplatedRequestFile, Vec<Spanned<ReqlangError>>> {
        let template_refs_to_replace: Vec<(String, ReferenceType)> = reqfile
            .refs
            .clone()
            .into_iter()
            .map(|(template_reference, _)| (format!("{template_reference}"), template_reference))
            .collect();

        let mut input = input.to_string();

        for (template_ref, ref_type) in &template_refs_to_replace {
            let value: Option<String> = match ref_type {
                ReferenceType::Variable(name) => {
                    Some(reqfile.config.0.vars.get(name).unwrap().to_owned())
                }
                ReferenceType::Prompt(name) => {
                    Some(reqfile.config.0.prompts.get(name).unwrap().to_owned())
                }
                ReferenceType::Secret(name) => {
                    Some(reqfile.config.0.secrets.get(name).unwrap().to_owned())
                }
                _ => None,
            };

            input = input.replace(template_ref, &value.unwrap_or(template_ref.clone()));
        }

        let reqfile = &split(&input).unwrap();

        for (template_ref, ref_type) in &template_refs_to_replace {
            let value: Option<String> = match ref_type {
                ReferenceType::Provider(name) => match name.as_str() {
                    "auth.oauth2.access_token" => {
                        let (config, _) = RequestFileParser::parse_config(&reqfile.config)
                            .unwrap()
                            .unwrap();
                        let auth = &config.auth.clone().unwrap();
                        let oauth2 = &auth.get("oauth2").unwrap();

                        let client_id = oauth2.get("client_id").unwrap().to_owned();
                        let client_secret = oauth2.get("client_secret").unwrap().to_owned();
                        let authorize_url = oauth2.get("authorize_url").unwrap().to_owned();
                        let access_token_url = oauth2.get("access_token_url").unwrap().to_owned();
                        let scopes: Vec<Scope> = oauth2
                            .get("scopes")
                            .unwrap()
                            .replace(',', " ")
                            .split(' ')
                            .map(|x| Scope::new(x.to_owned()))
                            .collect();

                        let client = BasicClient::new(
                            ClientId::new(client_id),
                            Some(ClientSecret::new(client_secret)),
                            AuthUrl::new(authorize_url).unwrap(),
                            Some(TokenUrl::new(access_token_url).unwrap()),
                        );

                        let token_result = client
                            .exchange_client_credentials()
                            .add_scopes(scopes)
                            .request(http_client)
                            .unwrap();

                        let access_token = token_result.access_token().secret().to_owned();

                        Some(access_token)
                    }
                    _ => None,
                },
                _ => None,
            };

            input = input.replace(template_ref, &value.unwrap_or(template_ref.clone()));
        }

        let split = RequestFileParser::split(&input).unwrap();

        let request =
            RequestFileParser::parse_request(&(split.request.0, reqfile.request.1.clone()))
                .unwrap();
        let response = RequestFileParser::parse_response(&split.response).map(|x| x.unwrap().0);

        Ok(TemplatedRequestFile {
            request: request.0,
            response,
        })
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

                // assert_eq!(unresolved_reqfile.is_ok(), true);

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
            "vars = [\"query_value\"]\n",
            "secrets = [\"api_key\"]",
            "\n",
            "[envs]\n",
            "[envs.dev]\n",
            "query_value = \"{{?test_value}}\"\n",
            "\n",
            "[envs.prod]\n",
            "query_value = \"{{?test_value}}\"\n",
            "\n",
            "[prompts]\n",
            "test_value = \"\"\n",
            "expected_response_body = \"\"\n",
            "\n",
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
                target: "/?query=test_value_value".to_string(),
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
        })
    );

    templater_test!(
        nested_references_in_config_not_supported,
        concat!(
            "vars = [\"query_value\", \"copy\"]\n",
            "secrets = [\"api_key\"]",
            "\n",
            "envs.dev.query_value = \"{{!api_key}}\"\n",
            "envs.dev.copy = \"{{:query_value}}\"\n",
            "\n",
            "---\n",
            "GET /?query={{:copy}} HTTP/1.1\n\n",
            "---\n",
            "---\n"
        ),
        "dev",
        HashMap::new(),
        HashMap::from([("api_key".to_string(), "api_key_value".to_string())]),
        Ok(TemplatedRequestFile {
            request: Request {
                verb: "GET".to_string(),
                target: "/?query={{!api_key}}".to_string(),
                http_version: "1.1".to_string(),
                headers: vec![],
                body: Some("".to_string())
            },
            response: None,
        })
    );
}
