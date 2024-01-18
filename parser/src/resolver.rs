use std::collections::HashMap;

use types::{ResolvedRequestFile, ResolvedRequestFileConfig, UnresolvedRequestFile};

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
    ) -> Result<ResolvedRequestFile, &'static str> {
        RequestFileResolver::new().resolve(reqfile, env, prompts, secrets)
    }

    pub fn resolve(
        &self,
        reqfile: &UnresolvedRequestFile,
        env: &str,
        prompts: &HashMap<String, String>,
        secrets: &HashMap<String, String>,
    ) -> Result<ResolvedRequestFile, &'static str> {
        Ok(ResolvedRequestFile {
            config: ResolvedRequestFileConfig {
                env: env.to_string(),
                vars: HashMap::new(),
                prompts: prompts.clone(),
                secrets: secrets.clone(),
            },
            request: reqfile.request.clone(),
            response: None,
        })
    }
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use types::{Request, UnresolvedRequestFileConfig};

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
            "HTTP/1.1 200 OK\n",
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

        let mut expected_headers = HashMap::new();
        expected_headers.insert("host".to_string(), "{{:base_url}}".to_string());
        expected_headers.insert("x-test".to_string(), "{{?test_value}}".to_string());
        expected_headers.insert("x-api-key".to_string(), "{{!api_key}}".to_string());

        assert_eq!(
            Request {
                verb: "POST".to_string(),
                target: "/".to_string(),
                http_version: "1.1".to_string(),
                headers: expected_headers,
                body: Some("[1, 2, 3]\n\n".to_string())
            },
            unresolved_reqfile.request
        );

        assert_eq!(None, unresolved_reqfile.response);

        let expected_vars = vec!["base_url".to_string()];

        let mut expected_envs = HashMap::new();

        let mut dev_env = HashMap::new();

        dev_env.insert(
            "base_url".to_string(),
            "https://dev.example.com".to_string(),
        );

        let mut prod_env = HashMap::new();

        prod_env.insert("base_url".to_string(), "https://example.com".to_string());

        expected_envs.insert("dev".to_string(), dev_env);
        expected_envs.insert("prod".to_string(), prod_env);

        let mut expected_prompts = HashMap::new();
        expected_prompts.insert("test_value".to_string(), Some("".to_string()));

        let expected_secrets = vec!["api_key".to_string()];

        assert_eq!(
            Some(UnresolvedRequestFileConfig {
                vars: expected_vars,
                envs: expected_envs,
                prompts: expected_prompts,
                secrets: expected_secrets
            }),
            unresolved_reqfile.config
        );

        let mut prompts = HashMap::new();
        prompts.insert("test_value".to_string(), "test_value_value".to_string());
        let mut secrets = HashMap::new();
        secrets.insert("api_key".to_string(), "api_key_value".to_string());

        let resolved_reqfile = RequestFileResolver::resolve_request_file(
            &unresolved_reqfile,
            "dev",
            &prompts,
            &secrets,
        )
        .unwrap();

        assert_eq!("dev", resolved_reqfile.config.env);

        let mut expected_resolved_vars = HashMap::new();
        expected_resolved_vars.insert("base_url".to_string(), "http://dev.example.com".to_string());
        assert_eq!(expected_resolved_vars, resolved_reqfile.config.vars);

        let mut expected_resolved_prompts = HashMap::new();
        expected_resolved_prompts.insert("test_value".to_string(), "test_value_value".to_string());
        assert_eq!(expected_resolved_prompts, resolved_reqfile.config.prompts);

        let mut expected_resolved_secrets = HashMap::new();
        expected_resolved_secrets.insert("api_key".to_string(), "api_key_value".to_string());
        assert_eq!(expected_resolved_secrets, resolved_reqfile.config.secrets);

        assert_eq!(unresolved_reqfile.request, resolved_reqfile.request);
        assert_eq!(unresolved_reqfile.response, resolved_reqfile.response);
    }
}
