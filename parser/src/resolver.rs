use std::collections::HashMap;

use errors::ReqlangError;
use types::{
    ResolvedRequestFile, ResolvedRequestFileConfig, UnresolvedRequestFile,
    UnresolvedRequestFileConfig,
};

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
    ) -> Result<ResolvedRequestFile, ReqlangError> {
        RequestFileResolver::new().resolve(reqfile, env, prompts, secrets)
    }

    pub fn resolve(
        &self,
        reqfile: &UnresolvedRequestFile,
        env: &str,
        prompts: &HashMap<String, String>,
        secrets: &HashMap<String, String>,
    ) -> Result<ResolvedRequestFile, ReqlangError> {
        Ok(ResolvedRequestFile {
            config: ResolvedRequestFileConfig {
                env: env.to_string(),
                vars: self.resolve_vars_from_envs(reqfile, env),
                prompts: prompts.clone(),
                secrets: secrets.clone(),
            },
            request: reqfile.request.clone(),
            response: reqfile.response.clone(),
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
            .unwrap_or(UnresolvedRequestFileConfig::default())
            .clone()
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

    use types::{Request, Response, UnresolvedRequestFileConfig};

    use crate::{parser::RequestFileParser, resolver::RequestFileResolver};

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
            "HTTP/1.1 200 OK\n\n",
            "---\n",
            "vars = [\"base_url\"]\n",
            "secrets = [\"api_key\"]\n",
            "[envs]\n",
            "[envs.dev]\n",
            "base_url = \"https://dev.example.com\"\n",
            "\n",
            "[envs.prod]\n",
            "base_url = \"https://example.com\"\n",
            "\n",
            "[prompts]\n",
            "test_value = \"\"",
            "\n",
            "---\n"
        );

        let unresolved_reqfile = RequestFileParser::parse_string(&reqfile);

        assert_eq!(unresolved_reqfile.is_ok(), true);

        let unresolved_reqfile = unresolved_reqfile.unwrap();

        assert_eq!(
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
            unresolved_reqfile.request
        );

        assert_eq!(
            Some(Response {
                http_version: "1.1".to_string(),
                status_code: "200".to_string(),
                status_text: "OK".to_string(),
                headers: HashMap::new(),
                body: Some("".to_string())
            }),
            unresolved_reqfile.response
        );

        assert_eq!(
            Some(UnresolvedRequestFileConfig {
                vars: Some(vec!["base_url".to_string()]),
                envs: Some(HashMap::from([
                    (
                        "dev".to_string(),
                        HashMap::from([(
                            "base_url".to_string(),
                            "https://dev.example.com".to_string()
                        )])
                    ),
                    (
                        "prod".to_string(),
                        HashMap::from([(
                            "base_url".to_string(),
                            "https://example.com".to_string()
                        )])
                    ),
                ])),
                prompts: Some(HashMap::from([(
                    "test_value".to_string(),
                    Some("".to_string())
                )])),
                secrets: Some(vec!["api_key".to_string()])
            }),
            unresolved_reqfile.config
        );

        let resolved_reqfile = RequestFileResolver::resolve_request_file(
            &unresolved_reqfile,
            "dev",
            &HashMap::from([("test_value".to_string(), "test_value_value".to_string())]),
            &HashMap::from([("api_key".to_string(), "api_key_value".to_string())]),
        )
        .unwrap();

        assert_eq!("dev", resolved_reqfile.config.env);

        assert_eq!(
            HashMap::from([(
                "base_url".to_string(),
                "https://dev.example.com".to_string()
            )]),
            resolved_reqfile.config.vars
        );

        assert_eq!(
            HashMap::from([("test_value".to_string(), "test_value_value".to_string())]),
            resolved_reqfile.config.prompts
        );

        assert_eq!(
            HashMap::from([("api_key".to_string(), "api_key_value".to_string())]),
            resolved_reqfile.config.secrets
        );

        assert_eq!(
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
            resolved_reqfile.request
        );

        assert_eq!(
            Some(Response {
                http_version: "1.1".to_string(),
                status_code: "200".to_string(),
                status_text: "OK".to_string(),
                headers: HashMap::new(),
                body: Some("".to_string())
            }),
            resolved_reqfile.response
        );
    }
}
