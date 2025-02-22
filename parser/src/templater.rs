use std::collections::HashMap;

use errors::{ReqlangError, ResolverError};
use span::{Spanned, NO_SPAN};
use types::{ParsedRequestFile, ReferenceType, TemplatedRequestFile};

use crate::{
    ast,
    parser::{parse, parse_request, parse_response},
};

/// Template a request file string into a [TemplatedRequestFile].
pub fn template(
    reqfile_string: &str,
    env: Option<&str>,
    prompts: &HashMap<String, String>,
    secrets: &HashMap<String, String>,
    provider_values: &HashMap<String, String>,
) -> Result<TemplatedRequestFile, Vec<Spanned<ReqlangError>>> {
    let ast = ast::Ast::new(reqfile_string);
    let parsed_reqfile = parse(&ast)?;

    if let Some(env) = env {
        match &parsed_reqfile.config {
            Some((config, span)) => {
                if config.envs().is_empty() {
                    return Err(vec![(
                        ResolverError::NoEnvironmentsDefined(env.to_string()).into(),
                        span.clone(),
                    )]);
                }
            }
            None => {
                return Err(vec![(
                    ResolverError::NoEnvironmentsDefined(env.to_string()).into(),
                    0..0,
                )]);
            }
        }

        if !parsed_reqfile.envs().contains(&env.to_string()) {
            return Err(vec![(
                ResolverError::InvalidEnvError(env.to_string()).into(),
                parsed_reqfile.config.map(|x| x.1).unwrap_or_default(),
            )]);
        }
    }

    let mut templating_errors: Vec<Spanned<ReqlangError>> = vec![];

    let reqfile: &ParsedRequestFile = &parsed_reqfile;

    let prompts = prompts.clone();
    let required_prompts = parsed_reqfile.prompts();
    let missing_prompts = required_prompts
        .iter()
        .filter(|prompt| !prompts.contains_key(*prompt))
        .map(|prompt| ResolverError::PromptValueNotPassed(prompt.clone()).into())
        .map(|err| (err, NO_SPAN))
        .collect::<Vec<Spanned<ReqlangError>>>();

    templating_errors.extend(missing_prompts);

    let secrets = secrets.clone();
    let required_secrets = parsed_reqfile.secrets();
    let missing_secrets = required_secrets
        .iter()
        .filter(|secret| !secrets.contains_key(*secret))
        .map(|secret| ResolverError::SecretValueNotPassed(secret.clone()).into())
        .map(|err| (err, NO_SPAN))
        .collect::<Vec<Spanned<ReqlangError>>>();

    templating_errors.extend(missing_secrets);

    if !templating_errors.is_empty() {
        return Err(templating_errors);
    }

    // Gather list of template references along with each reference's type
    //
    // e.g. ("{{:var_name}}", ReferenceType::Variable("var_name"))
    let template_refs_to_replace: Vec<(String, ReferenceType)> = reqfile
        .refs
        .iter()
        .map(|(template_reference, _)| {
            (format!("{template_reference}"), template_reference.clone())
        })
        .collect();

    // Replace template references with the resolved values
    let templated_input = {
        let mut input = reqfile_string.to_string();

        // If the environment is provided, use it to resolve template references
        let vars = match env {
            Some(env) => reqfile.env(env).unwrap_or_default(),
            None => {
                // TODO: validate if environments are defined in the request file
                HashMap::new()
            }
        };

        // Replace template references with the resolved values
        for (template_ref, ref_type) in &template_refs_to_replace {
            let value = match ref_type {
                ReferenceType::Variable(name) => vars.get(name),
                ReferenceType::Prompt(name) => prompts.get(name),
                ReferenceType::Secret(name) => secrets.get(name),
                ReferenceType::Provider(name) => provider_values.get(name),
                _ => None,
            };

            // If reference can not be resolved, keep the template reference as is
            if value.is_some() {
                input = input.replace(template_ref, value.unwrap());
            }
        }

        input
    };

    let ast = ast::Ast::new(&templated_input);
    let request = ast.request().cloned().expect("should have a request");
    let response = ast.response().cloned();

    // Parse the templated request
    let request = {
        let (request, request_span) = request;
        parse_request(&(request, request_span.clone())).unwrap().0
    };

    // Parse the templated response
    let response = parse_response(&response).map(|x| x.unwrap().0);

    Ok(TemplatedRequestFile { request, response })
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use errors::{ReqlangError, ResolverError};
    use span::NO_SPAN;
    use types::{
        http::{HttpRequest, HttpResponse, HttpStatusCode},
        TemplatedRequestFile,
    };

    use crate::templater::template;

    macro_rules! templater_test {
        ($test_name:ident, $reqfile_string:expr, $env:expr, $prompts:expr, $secrets:expr, $provider_values: expr, $result:expr) => {
            #[test]
            fn $test_name() {
                let templated_reqfile = template(
                    &$reqfile_string,
                    $env,
                    &$prompts,
                    &$secrets,
                    $provider_values,
                );

                ::pretty_assertions::assert_eq!($result, templated_reqfile);
            }
        };
    }

    static REQFILE: &str = r#"
```%config
vars = ["query_value"]
secrets = ["api_key"]

[envs]
[envs.dev]
query_value = "{{?test_value}}"

[envs.prod]
query_value = "{{?test_value}}"

[prompts]
test_value = ""
expected_response_body = ""
```

```%request
POST /?query={{:query_value}} HTTP/1.1
x-test: {{?test_value}}
x-api-key: {{!api_key}}

[1, 2, 3]

```

```%response
HTTP/1.1 200 OK

{{?expected_response_body}}

```
        "#;

    templater_test!(
        full_request_file,
        REQFILE,
        Some("dev"),
        HashMap::from([
            ("test_value".to_string(), "test_value_value".to_string()),
            (
                "expected_response_body".to_string(),
                "expected_response_body_value".to_string()
            )
        ]),
        HashMap::from([("api_key".to_string(), "api_key_value".to_string())]),
        &HashMap::default(),
        Ok(TemplatedRequestFile {
            request: HttpRequest {
                verb: "POST".into(),
                target: "/?query=test_value_value".to_string(),
                http_version: "1.1".into(),
                headers: vec![
                    ("x-test".to_string(), "test_value_value".to_string()),
                    ("x-api-key".to_string(), "api_key_value".to_string()),
                ],
                body: Some("[1, 2, 3]\n\n\n".to_string())
            },
            response: Some(HttpResponse {
                http_version: "1.1".into(),
                status_code: HttpStatusCode::new(200),
                status_text: "OK".to_string(),
                headers: vec![],
                body: Some("expected_response_body_value\n\n\n".to_string())
            }),
        })
    );

    templater_test!(
        missing_secret_input,
        REQFILE,
        Some("dev"),
        HashMap::from([
            ("test_value".to_string(), "test_value_value".to_string()),
            (
                "expected_response_body".to_string(),
                "expected_response_body_value".to_string()
            )
        ]),
        HashMap::default(),
        &HashMap::default(),
        Err(vec![(
            ReqlangError::ResolverError(errors::ResolverError::SecretValueNotPassed(
                "api_key".to_string()
            )),
            NO_SPAN
        )])
    );

    templater_test!(
        missing_prompt_input,
        REQFILE,
        Some("dev"),
        HashMap::from([("test_value".to_string(), "test_value_value".to_string()),]),
        HashMap::from([("api_key".to_string(), "api_key_value".to_string())]),
        &HashMap::default(),
        Err(vec![(
            ReqlangError::ResolverError(errors::ResolverError::PromptValueNotPassed(
                "expected_response_body".to_string()
            )),
            NO_SPAN
        )])
    );

    templater_test!(
        nested_references_in_config_not_supported,
        textwrap::dedent(
            "
            ```%config
            vars = [\"query_value\", \"copy\"]
            secrets = [\"api_key\"]

            envs.dev.query_value = \"{{!api_key}}\"
            envs.dev.copy = \"{{:query_value}}\"
            ```

            ```%request
            GET https://example.com/?query={{:copy}} HTTP/1.1
            ```
            "
        ),
        Some("dev"),
        HashMap::new(),
        HashMap::from([("api_key".to_string(), "api_key_value".to_string())]),
        &HashMap::default(),
        Ok(TemplatedRequestFile {
            request: HttpRequest {
                verb: "GET".into(),
                target: "https://example.com/?query={{!api_key}}".to_string(),
                http_version: "1.1".into(),
                headers: vec![],
                body: Some("".to_string())
            },
            response: None,
        })
    );

    templater_test!(
        resolve_env_with_no_config,
        textwrap::dedent(
            "
            ```%request
            GET https://example.com/ HTTP/1.1
            ```
            "
        ),
        Some("dev"),
        HashMap::new(),
        HashMap::new(),
        &HashMap::default(),
        Err(vec![(
            ResolverError::NoEnvironmentsDefined("dev".to_string()).into(),
            0..0
        )])
    );

    templater_test!(
        resolve_env_with_config_but_no_envs,
        textwrap::dedent(
            "
            ```%config
            ```

            ```%request
            GET https://example.com/ HTTP/1.1
            ```
            "
        ),
        Some("dev"),
        HashMap::new(),
        HashMap::new(),
        &HashMap::default(),
        Err(vec![(
            ResolverError::NoEnvironmentsDefined("dev".to_string()).into(),
            12..12
        )])
    );

    templater_test!(
        resolve_env_with_config_and_envs_but_none_defined,
        textwrap::dedent(
            "
            ```%config
            [envs]
            ```

            ```%request
            GET https://example.com/ HTTP/1.1
            ```
            "
        ),
        Some("dev"),
        HashMap::new(),
        HashMap::new(),
        &HashMap::default(),
        Err(vec![(
            ResolverError::NoEnvironmentsDefined("dev".to_string()).into(),
            12..19
        )])
    );

    templater_test!(
        resolve_env_with_config_and_envs_but_invalid_env,
        textwrap::dedent(
            "
            ```%config
            vars = [\"foo\"]

            [envs]
            [envs.test]
            foo = \"bar\"
            ```

            ```%request
            GET https://example.com/?value={{:foo}} HTTP/1.1
            ```
            "
        ),
        Some("dev"),
        HashMap::new(),
        HashMap::new(),
        &HashMap::default(),
        Err(vec![(
            ResolverError::InvalidEnvError("dev".to_string()).into(),
            12..59
        )])
    );
}
