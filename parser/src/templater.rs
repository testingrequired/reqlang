use std::collections::HashMap;

use errors::{ReqlangError, ResolverError};
use span::{Spanned, NO_SPAN};
use types::{ParsedRequestFile, ReferenceType, TemplatedRequestFile};

use crate::{
    parser::{parse, parse_request, parse_response},
    splitter::split,
};

/// Template a request file string into a [TemplatedRequestFile].
pub fn template(
    reqfile_string: &str,
    env: &str,
    prompts: &HashMap<String, String>,
    secrets: &HashMap<String, String>,
    provider_values: &HashMap<String, String>,
) -> Result<TemplatedRequestFile, Vec<Spanned<ReqlangError>>> {
    let parsed_reqfile = parse(reqfile_string)?;

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
        let vars = reqfile.env(env).unwrap_or_default();

        for (template_ref, ref_type) in &template_refs_to_replace {
            let value = match ref_type {
                ReferenceType::Variable(name) => Some(vars.get(name).unwrap().to_owned()),
                ReferenceType::Prompt(name) => Some(prompts.get(name).unwrap().to_owned()),
                ReferenceType::Secret(name) => Some(secrets.get(name).unwrap().to_owned()),
                ReferenceType::Provider(name) => provider_values.get(name).cloned(),
                _ => None,
            };

            input = input.replace(template_ref, &value.unwrap_or(template_ref.clone()));
        }

        input
    };

    // Split the templated input to pull out the request and response parts
    let reqfile_split = split(&templated_input).unwrap();

    // Parse the templated request
    let request = {
        let (request, request_span) = reqfile_split.request;
        parse_request(&(request, request_span.clone())).unwrap().0
    };

    // Parse the templated response
    let response = parse_response(&reqfile_split.response).map(|x| x.unwrap().0);

    Ok(TemplatedRequestFile { request, response })
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use errors::ReqlangError;
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
        "dev",
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
                headers: HashMap::new(),
                body: Some("expected_response_body_value\n\n\n".to_string())
            }),
        })
    );

    templater_test!(
        missing_secret_input,
        REQFILE,
        "dev",
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
        "dev",
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
        "dev",
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
}
