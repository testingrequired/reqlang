use std::collections::HashMap;

use reqlang_expr::prelude::*;

use crate::{
    ast::Ast,
    errors::{ReqlangError, ResolverError},
    parser::{parse, parse_request, parse_response},
    span::{NO_SPAN, Spanned},
    types::{ParsedRequestFile, ReferenceType, TemplatedRequestFile},
};

/// Template a request file string into a [TemplatedRequestFile].
pub fn template(
    reqfile_string: &str,
    env: Option<&str>,
    prompts: &HashMap<String, String>,
    secrets: &HashMap<String, String>,
    provider_values: &HashMap<String, String>, // TODO: Wire this up
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

    // Validate all required prompt values were passed
    {
        let missing_prompts_errs = {
            parsed_reqfile
                .required_prompts()
                .into_iter()
                .filter(|prompt| !prompts.contains_key(prompt))
                .map(|prompt| ResolverError::PromptValueNotPassed(prompt.clone()).into())
                .map(|err| (err, NO_SPAN))
                .collect::<Vec<Spanned<ReqlangError>>>()
        };

        templating_errors.extend(missing_prompts_errs);
    };

    // Validate all required secret values were passed
    {
        let missing_secrets_errs = parsed_reqfile
            .secrets()
            .into_iter()
            .filter(|secret| !secrets.contains_key(secret))
            .map(|secret| ResolverError::SecretValueNotPassed(secret.clone()).into())
            .map(|err| (err, NO_SPAN))
            .collect::<Vec<Spanned<ReqlangError>>>();

        templating_errors.extend(missing_secrets_errs);
    };

    if !templating_errors.is_empty() {
        return Err(templating_errors);
    }

    // Replace template references with the resolved values
    let templated_input = {
        let mut input = reqfile_string.to_string();

        // Gather list of template references along with each reference's type
        //
        // e.g. ("{{:var_name}}", ReferenceType::Variable("var_name"))
        let template_refs_to_replace: Vec<(String, ReferenceType, Span)> = reqfile
            .refs
            .iter()
            .map(|(template_reference, template_reference_span)| {
                (
                    format!("{template_reference}"),
                    template_reference.clone(),
                    template_reference_span.clone(),
                )
            })
            .collect();

        // If the environment is provided, use it to resolve template references
        let default_variable_values = parsed_reqfile.default_variable_values();
        let vars = match env {
            Some(env) => reqfile.env(env).unwrap_or(default_variable_values),
            None => HashMap::new(),
        };

        let mut compiler_env = CompileTimeEnv::new(
            reqfile.vars(),
            reqfile.prompts(),
            reqfile.secrets(),
            provider_values.keys().cloned().collect(),
        );

        let mut runtime_env = {
            let var_values: Vec<String> = match env {
                Some(env) => reqfile
                    .vars()
                    .iter()
                    .map(|x| {
                        let config = parsed_reqfile.config.clone().unwrap().0.env(env).unwrap();
                        let value = config
                            .get(&x.clone().clone())
                            .unwrap_or(vars.get(&x.clone().clone()).unwrap());

                        value.clone()
                    })
                    .collect(),
                None => provider_values.values().cloned().collect(),
            };

            let prompt_values = {
                let default_prompt_values = parsed_reqfile.default_prompt_values();

                reqfile
                    .prompts()
                    .iter()
                    .map(|x| {
                        let value = prompts.get(x).cloned().unwrap_or(
                            default_prompt_values
                                .get(x)
                                .cloned()
                                .unwrap_or_default()
                                .clone(),
                        );

                        value.clone()
                    })
                    .collect()
            };

            let secret_values: Vec<Option<String>> = reqfile
                .secrets()
                .iter()
                .map(|x| secrets.get(x).cloned())
                .collect();

            RuntimeEnv {
                vars: var_values.clone(),
                prompts: prompt_values,
                secrets: secret_values.iter().filter_map(|x| x.clone()).collect(),
                client_context: provider_values
                    .values()
                    .cloned()
                    .map(Value::String)
                    .collect(),
            }
        };

        // Set client context `@env` if an environment was provided
        if let Some(env) = env {
            let env_context_index = compiler_env.add_to_client_context("env");
            runtime_env.add_to_client_context(env_context_index, Value::String(env.to_string()));
        }

        let mut vm = Vm::new();

        for (_, ref_type, ref_span) in &template_refs_to_replace {
            match reqlang_expr::parser::parse(&ref_type.lookup_name()) {
                Ok(expr) => match compile(&mut (expr, ref_span.clone()), &compiler_env) {
                    Ok(bytecode) => {
                        let result = vm.interpret(bytecode.into(), &compiler_env, &runtime_env);

                        let replacement_string = match result {
                            Ok(expr_value) => expr_value
                                .get_string()
                                .expect("should be string")
                                .to_string(),
                            Err(expr_errs) => {
                                templating_errors.push((
                                    ReqlangError::ResolverError(
                                        ResolverError::ExpressionEvaluationError(
                                            ref_type.lookup_name().clone(),
                                            format!("{expr_errs:#?}"),
                                        ),
                                    ),
                                    ref_span.clone(),
                                ));

                                // In the case of an error, replacement string
                                // is the original string
                                ref_type.lookup_name().clone()
                            }
                        };

                        let x = format!("{{{{{}}}}}", ref_type.lookup_name());

                        input = input.replace(&x, &replacement_string);
                    }
                    Err(expr_errs) => {
                        templating_errors.push((
                            ReqlangError::ResolverError(ResolverError::ExpressionEvaluationError(
                                ref_type.lookup_name().clone(),
                                format!("{expr_errs:#?}"),
                            )),
                            ref_span.clone(),
                        ));
                    }
                },
                Err(expr_errs) => {
                    templating_errors.push((
                        ReqlangError::ResolverError(ResolverError::ExpressionEvaluationError(
                            ref_type.lookup_name().clone(),
                            format!("{expr_errs:#?}"),
                        )),
                        ref_span.clone(),
                    ));
                }
            }
        }

        let items = &reqfile.exprs.to_vec();
        for (expr_ref, expr_span) in items {
            match reqlang_expr::parser::parse(&format!("({expr_ref})")) {
                Ok(expr) => match compile(&mut (expr, expr_span.clone()), &compiler_env) {
                    Ok(bytecode) => {
                        let result = vm.interpret(bytecode.into(), &compiler_env, &runtime_env);

                        let replacement_string = match result {
                            Ok(expr_value) => expr_value
                                .get_string()
                                .expect("should be string")
                                .to_string(),
                            Err(expr_errs) => {
                                templating_errors.push((
                                    ReqlangError::ResolverError(
                                        ResolverError::ExpressionEvaluationError(
                                            expr_ref.clone(),
                                            format!("{expr_errs:#?}"),
                                        ),
                                    ),
                                    expr_span.clone(),
                                ));

                                // In the case of an error, replacement string
                                // is the original string
                                expr_ref.clone()
                            }
                        };

                        let x = format!("{{({expr_ref})}}");

                        input = input.replace(&x, &replacement_string);
                    }
                    Err(expr_errs) => {
                        templating_errors.push((
                            ReqlangError::ResolverError(ResolverError::ExpressionEvaluationError(
                                expr_ref.clone(),
                                format!("{expr_errs:#?}"),
                            )),
                            expr_span.clone(),
                        ));
                    }
                },
                Err(expr_errs) => {
                    templating_errors.push((
                        ReqlangError::ResolverError(ResolverError::ExpressionEvaluationError(
                            expr_ref.clone(),
                            format!("{expr_errs:#?}"),
                        )),
                        expr_span.clone(),
                    ));
                }
            }
        }

        if !templating_errors.is_empty() {
            return Err(templating_errors.clone());
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

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use crate::{
        errors::{ReqlangError, ResolverError},
        span::NO_SPAN,
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
            ReqlangError::ResolverError(ResolverError::SecretValueNotPassed("api_key".to_string())),
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
            ReqlangError::ResolverError(ResolverError::PromptValueNotPassed(
                "expected_response_body".to_string()
            )),
            NO_SPAN
        )])
    );

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

    templater_test!(
        use_default_prompt_value_if_defined_and_no_prompt_passed,
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
        HashMap::new(),
        HashMap::new(),
        &HashMap::default(),
        Ok(TemplatedRequestFile {
            request: HttpRequest {
                verb: "GET".into(),
                target: "https://example.com/?query=123".to_string(),
                http_version: "1.1".into(),
                headers: vec![],
                body: Some("".to_string())
            },
            response: None,
        })
    );

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
