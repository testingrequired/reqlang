use std::collections::HashMap;

use parser::RequestFileParser;
use resolver::RequestFileResolver;
use types::ResolvedRequestFile;

mod parser;
mod resolver;

/// Parse a string in to a request file and resolve template values
pub fn parse(
    input: &str,
    env: &str,
    prompts: HashMap<String, String>,
    secrets: HashMap<String, String>,
) -> Result<ResolvedRequestFile, &'static str> {
    let reqfile = RequestFileParser::parse_string(input);

    if let Err(err) = reqfile {
        return Err(err);
    }

    let reqfile = reqfile.unwrap();

    RequestFileResolver::resolve_request_file(&reqfile, env, &prompts, &secrets)
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use types::{Request, Response};

    use crate::parse;

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

        let mut secrets = HashMap::new();
        secrets.insert("api_key".to_string(), "api_key_value".to_string());

        let mut prompts = HashMap::new();
        prompts.insert("test_value".to_string(), "test_value_value".to_string());

        let resolved_reqfile = parse(&reqfile, "dev", prompts, secrets).unwrap();

        assert_eq!("dev", resolved_reqfile.config.env);

        let mut expected_resolved_vars = HashMap::new();
        expected_resolved_vars.insert(
            "base_url".to_string(),
            "https://dev.example.com".to_string(),
        );
        assert_eq!(expected_resolved_vars, resolved_reqfile.config.vars);

        let mut expected_resolved_prompts = HashMap::new();
        expected_resolved_prompts.insert("test_value".to_string(), "test_value_value".to_string());
        assert_eq!(expected_resolved_prompts, resolved_reqfile.config.prompts);

        let mut expected_resolved_secrets = HashMap::new();
        expected_resolved_secrets.insert("api_key".to_string(), "api_key_value".to_string());
        assert_eq!(expected_resolved_secrets, resolved_reqfile.config.secrets);

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
