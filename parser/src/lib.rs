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

        let resolved_reqfile = parse(
            &reqfile,
            "dev",
            HashMap::from([("test_value".to_string(), "test_value_value".to_string())]),
            HashMap::from([("api_key".to_string(), "api_key_value".to_string())]),
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
