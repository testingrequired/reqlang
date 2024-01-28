use errors::ReqlangError;
use span::Spanned;
use types::{ReferenceType, ResolvedRequestFile, TemplatedRequestFile};

use crate::parser::RequestFileParser;

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

        for (template_ref, ref_type) in template_refs_to_replace {
            let value = match ref_type {
                ReferenceType::Variable(name) => reqfile.config.0.vars.get(&name),
                ReferenceType::Prompt(name) => reqfile.config.0.prompts.get(&name),
                ReferenceType::Secret(name) => reqfile.config.0.secrets.get(&name),
                ReferenceType::Unknown(_) => unreachable!(),
            };

            input = input.replace(
                &template_ref,
                value.unwrap_or(&String::from("COULD NOT FIND TEMPLATE VALUE")),
            );
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
