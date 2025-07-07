use std::collections::HashMap;

use regex::Regex;
use reqlang_expr::prelude::*;

use crate::{
    ast::Ast,
    errors::{ReqlangError, ResolverError},
    parser::{TEMPLATE_EXPR_REFERENCE_PATTERN_INNER, parse, parse_request, parse_response},
    span::{NO_SPAN, Spanned},
    types::{ParsedRequestFile, ReferenceType, TemplatedRequestFile},
};

/// Template a request file string into a [TemplatedRequestFile].
pub fn template(
    reqfile_string: &str,
    env: Option<&str>,
    prompts: &HashMap<String, String>,
    secrets: &HashMap<String, String>,
    provider_values: &HashMap<String, String>,
) -> Result<TemplatedRequestFile, Vec<Spanned<ReqlangError>>> {
    let ast = Ast::from(reqfile_string);
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

    // let mut var_values = reqfile.vars().iter().map(|x| match env {
    //     Some(env) => {
    //         let config = parsed_reqfile.config.unwrap().0.env(None).unwrap();
    //         let value = config.get(x).unwrap();
    //     }
    //     None => todo!(),
    // });

    let var_values = match env {
        Some(env) => reqfile
            .vars()
            .iter()
            .map(|x| {
                let config = parsed_reqfile.config.clone().unwrap().0.env(env).unwrap();
                let value = config.get(x).unwrap();

                value.clone()
            })
            .collect(),
        None => vec![],
    };

    let prompt_values: Vec<String> = reqfile
        .prompts()
        .iter()
        .map(|x| prompts.get(x).unwrap().clone())
        .collect();

    let secret_values: Vec<String> = reqfile
        .secrets()
        .iter()
        .map(|x| secrets.get(x).unwrap().clone())
        .collect();

    let mut runtime_env = RuntimeEnv {
        vars: var_values.clone(),
        prompts: prompt_values.clone(),
        secrets: secret_values.clone(),
        client_context: vec![],
    };

    let default_variable_values = parsed_reqfile.default_variable_values();

    let required_prompts = parsed_reqfile.required_prompts();
    let default_prompt_values = parsed_reqfile.default_prompt_values();
    let missing_prompts = required_prompts
        .into_iter()
        .filter(|prompt| !prompts.contains_key(prompt))
        .map(|prompt| ResolverError::PromptValueNotPassed(prompt.clone()).into())
        .map(|err| (err, NO_SPAN))
        .collect::<Vec<Spanned<ReqlangError>>>();

    templating_errors.extend(missing_prompts);

    let required_secrets = parsed_reqfile.secrets();
    let missing_secrets = required_secrets
        .into_iter()
        .filter(|secret| !secrets.contains_key(secret))
        .map(|secret| ResolverError::SecretValueNotPassed(secret.clone()).into())
        .map(|err| (err, NO_SPAN))
        .collect::<Vec<Spanned<ReqlangError>>>();

    templating_errors.extend(missing_secrets);

    if !templating_errors.is_empty() {
        return Err(templating_errors);
    }

    let expr_refs_to_replace: Vec<Spanned<String>> = reqfile.exprs.to_vec();

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

        let mut compiler_env =
            CompileTimeEnv::new(reqfile.vars(), reqfile.prompts(), reqfile.secrets(), vec![]);

        let env_context_index = compiler_env.add_to_client_context("env");
        runtime_env.add_to_client_context(
            env_context_index,
            Value::String(env.unwrap_or_default().to_string()),
        );

        let mut vm = Vm::new();

        for (expr_ref, expr_span) in &expr_refs_to_replace {
            let expr_source = parse_inner_expr(expr_ref);

            match reqlang_expr::parser::parse(&expr_source) {
                Ok(expr) => match compile(&mut (expr, expr_span.clone()), &compiler_env) {
                    Ok(compiled_expr) => {
                        let result =
                            vm.interpret(compiled_expr.into(), &compiler_env, &runtime_env);

                        let replacement_string = match result {
                            Ok(value) => value.get_string().expect("should be string").to_string(),
                            Err(_interpreter_err) => {
                                templating_errors.push((
                                    ReqlangError::ResolverError(
                                        ResolverError::ExpressionEvaluationError(
                                            expr_ref.clone(),
                                            "".to_string(),
                                        ),
                                    ),
                                    expr_span.clone(),
                                ));

                                expr_ref.clone()
                            }
                        };

                        input = input.replace(expr_ref, &replacement_string);
                    }
                    Err(expr_err) => {
                        templating_errors.push((
                            ReqlangError::ResolverError(ResolverError::ExpressionEvaluationError(
                                expr_ref.clone(),
                                format!("{expr_err:#?}"),
                            )),
                            expr_span.clone(),
                        ));
                    }
                },
                Err(expr_err) => {
                    templating_errors.push((
                        ReqlangError::ResolverError(ResolverError::ExpressionEvaluationError(
                            expr_ref.clone(),
                            format!("{expr_err:#?}"),
                        )),
                        expr_span.clone(),
                    ));
                }
            };
        }

        if !templating_errors.is_empty() {
            return Err(templating_errors);
        }

        // Replace template references with the resolved values
        for (template_ref, ref_type) in &template_refs_to_replace {
            let value = match ref_type {
                ReferenceType::Variable(name) => {
                    vars.get(name).or(default_variable_values.get(name))
                }
                ReferenceType::Prompt(name) => {
                    prompts.get(name).or(default_prompt_values.get(name))
                }
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

    let ast = Ast::from(&templated_input);
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

pub fn parse_inner_expr(input: &str) -> String {
    let re = Regex::new(TEMPLATE_EXPR_REFERENCE_PATTERN_INNER).unwrap();

    let mut captured_exprs: Vec<String> = vec![];

    for (_, [expr]) in re.captures_iter(input).map(|cap| cap.extract()) {
        captured_exprs.push(expr.to_string());
    }

    captured_exprs
        .first()
        .expect("should have captured the inner expression")
        .clone()
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use crate::{
        errors::ResolverError,
        templater::template,
        types::{
            TemplatedRequestFile,
            http::{HttpRequest, HttpResponse, HttpStatusCode},
        },
    };

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
secrets = ["api_key"]

[[vars]]
name = "query_value"

[envs]
[envs.dev]
query_value = "{{?test_value}}"

[envs.prod]
query_value = "{{?test_value}}"

[[prompts]]
name = "test_value"

[[prompts]]
name = "expected_response_body"
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

    // templater_test!(
    //     missing_secret_input,
    //     REQFILE,
    //     Some("dev"),
    //     HashMap::from([
    //         ("test_value".to_string(), "test_value_value".to_string()),
    //         (
    //             "expected_response_body".to_string(),
    //             "expected_response_body_value".to_string()
    //         )
    //     ]),
    //     HashMap::default(),
    //     &HashMap::default(),
    //     Err(vec![(
    //         ReqlangError::ResolverError(ResolverError::SecretValueNotPassed("api_key".to_string())),
    //         NO_SPAN
    //     )])
    // ); TODO: Uncomment

    // templater_test!(
    //     missing_prompt_input,
    //     REQFILE,
    //     Some("dev"),
    //     HashMap::from([("test_value".to_string(), "test_value_value".to_string()),]),
    //     HashMap::from([("api_key".to_string(), "api_key_value".to_string())]),
    //     &HashMap::default(),
    //     Err(vec![(
    //         ReqlangError::ResolverError(ResolverError::PromptValueNotPassed(
    //             "expected_response_body".to_string()
    //         )),
    //         NO_SPAN
    //     )])
    // ); TODO: Uncomment

    templater_test!(
        nested_references_in_config_not_supported,
        textwrap::dedent(
            r#"
            ```%config
            secrets = ["api_key"]

            [[vars]]
            name = "query_value"

            [[vars]]
            name = "copy"

            [envs.dev]
            query_value = "{{!api_key}}"
            copy = "{{:query_value}}"
            ```

            ```%request
            GET https://example.com/?query={{:copy}} HTTP/1.1
            ```
            "#
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
            12..18
        )])
    );

    templater_test!(
        resolve_env_with_config_and_envs_but_invalid_env,
        textwrap::dedent(
            r#"
            ```%config
            [[vars]]
            name = "foo"

            [envs]
            [envs.test]
            foo = "bar"
            ```

            ```%request
            GET https://example.com/?value={{:foo}} HTTP/1.1
            ```
            "#
        ),
        Some("dev"),
        HashMap::new(),
        HashMap::new(),
        &HashMap::default(),
        Err(vec![(
            ResolverError::InvalidEnvError("dev".to_string()).into(),
            12..65
        )])
    );

    // templater_test!(
    //     use_default_prompt_value_if_defined_and_no_prompt_passed,
    //     textwrap::dedent(
    //         "
    //         ```%config
    //         [[prompts]]
    //         name = \"value\"
    //         default = \"123\"
    //         ```

    //         ```%request
    //         GET https://example.com/?query={{?value}} HTTP/1.1
    //         ```
    //         "
    //     ),
    //     None,
    //     HashMap::new(),
    //     HashMap::new(),
    //     &HashMap::default(),
    //     Ok(TemplatedRequestFile {
    //         request: HttpRequest {
    //             verb: "GET".into(),
    //             target: "https://example.com/?query=123".to_string(),
    //             http_version: "1.1".into(),
    //             headers: vec![],
    //             body: Some("".to_string())
    //         },
    //         response: None,
    //     })
    // ); TODO: Uncomment

    templater_test!(
        use_input_prompt_value_if_defined_prompt_value_defined_and_input_prompt_passed,
        textwrap::dedent(
            "
            ```%config
            [[prompts]]
            name = \"value\"
            default = \"123\"
            ```

            ```%request
            GET https://example.com/?query={{?value}} HTTP/1.1
            ```
            "
        ),
        None,
        HashMap::from([("value".to_string(), "456".to_string()),]),
        HashMap::new(),
        &HashMap::default(),
        Ok(TemplatedRequestFile {
            request: HttpRequest {
                verb: "GET".into(),
                target: "https://example.com/?query=456".to_string(),
                http_version: "1.1".into(),
                headers: vec![],
                body: Some("".to_string())
            },
            response: None,
        })
    );

    templater_test!(
        use_default_variable_value,
        textwrap::dedent(
            r#"
            ```%config
            [[vars]]
            name = "foo"
            default = "123"

            [[vars]]
            name = "bar"

            [envs.test]
            bar = "456"
            ```

            ```%request
            GET https://example.com/?query={{:foo}}{{:bar}} HTTP/1.1
            ```
            "#
        ),
        Some("test"),
        HashMap::new(),
        HashMap::new(),
        &HashMap::default(),
        Ok(TemplatedRequestFile {
            request: HttpRequest {
                verb: "GET".into(),
                target: "https://example.com/?query=123456".to_string(),
                http_version: "1.1".into(),
                headers: vec![],
                body: Some("".to_string())
            },
            response: None,
        })
    );
}
