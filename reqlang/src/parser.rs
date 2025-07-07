use std::collections::HashMap;

use regex::Regex;

use crate::{
    ast::Ast,
    errors::{ParseError, ReqlangError},
    span::{NO_SPAN, Spanned},
    types::{
        ParsedConfig, ParsedRequestFile, ReferenceType,
        http::{HttpRequest, HttpResponse},
    },
};

pub const TEMPLATE_REFERENCE_PATTERN: &str = r"\{\{(.+)\}\}";
pub const TEMPLATE_REFERENCE_PATTERN_INNER: &str = r"([:?!@]{1})([a-zA-Z][_a-zA-Z0-9.]*)";

// This matches patterns for expression references e.g. {(id :var)}
pub const TEMPLATE_EXPR_REFERENCE_PATTERN: &str = r"(\{\(.*\)\})";
pub const TEMPLATE_EXPR_REFERENCE_PATTERN_INNER: &str = r"\{(\(.*\))\}";

static FORBIDDEN_REQUEST_HEADER_NAMES: &[&str] = &[
    "host",
    "accept-charset",
    "accept-encoding",
    "access-control-request-headers",
    "access-control-request-method",
    "connection",
    "content-length",
    "cookie",
    "date",
    "dnt",
    "expect",
    "keep-alive",
    "origin",
    "permission-policy",
    "te",
    "trailer",
    "transfer-encoding",
    "upgrade",
    "via",
];

/// Parse [crate::ast::Ast] into a [ParsedRequestFile]
pub fn parse(ast: &Ast) -> Result<ParsedRequestFile, Vec<Spanned<ReqlangError>>> {
    match ast.request() {
        Some(request) => {
            let mut parse_errors: Vec<Spanned<ReqlangError>> = vec![];

            let response = ast.response().cloned();
            let config = ast.config().cloned();

            // Extract template references from the request, response, and config

            let request_refs = parse_references(request);
            let response_refs = parse_references(&response.clone().unwrap_or_default());
            let config_refs = parse_references(&config.clone().unwrap_or_default());
            let mut refs: Vec<(ReferenceType, std::ops::Range<usize>)> = vec![];
            refs.extend(request_refs);
            refs.extend(response_refs);
            refs.extend(config_refs);

            // Extract expression references from the request, response, and config

            let request_exprs = parse_expressions(request);
            let response_exprs = parse_expressions(&response.clone().unwrap_or_default());
            let config_exprs = parse_expressions(&config.clone().unwrap_or_default());
            let mut exprs: Vec<Spanned<String>> = vec![];
            exprs.extend(request_exprs);
            exprs.extend(response_exprs);
            exprs.extend(config_exprs);

            // Extract template references from expression references

            for (expr, expr_span) in exprs.iter() {
                let expr_refs = parse_inner_references(&(expr.clone(), expr_span.clone()));
                refs.extend(expr_refs);
            }

            // Parse HTTP request

            let request = match parse_request(request) {
                Ok((request, span)) => {
                    for key in request.headers.iter().map(|x| &x.0) {
                        if FORBIDDEN_REQUEST_HEADER_NAMES.contains(&key.to_lowercase().as_str()) {
                            parse_errors.push((
                                ParseError::ForbiddenRequestHeaderNameError(key.to_lowercase())
                                    .into(),
                                span.clone(),
                            ))
                        }
                    }

                    Some((request, span))
                }
                Err(err) => {
                    parse_errors.extend(err);
                    None
                }
            };

            // Parse HTTP response

            let response = match parse_response(&response) {
                Some(Ok(response)) => Some(response),
                Some(Err(err)) => {
                    parse_errors.extend(err);
                    None
                }
                None => None,
            };

            let config = match parse_config(&config) {
                Some(Ok((config, config_span))) => Some((config, config_span)),
                Some(Err(err)) => {
                    parse_errors.extend(err);
                    None
                }
                None => None,
            };

            // Validate variables are defined correctly in the config

            if let Some((config, config_span)) = &config {
                let env_names = config.envs();

                for var in config.vars().iter() {
                    // If a variable is defined, then at least one environment must be defined

                    if env_names.is_empty() {
                        parse_errors.push((
                            ParseError::VariableNotDefinedInAnyEnvironment(var.to_string()).into(),
                            config_span.clone(),
                        ));
                    }

                    // Extract default values from variables that define one

                    let default_var_values = {
                        let mut default_values = HashMap::new();

                        let default_values_pairs: Vec<(String, String)> = config
                            .vars
                            .clone()
                            .unwrap_or_default()
                            .iter()
                            // Find variables that have a default value defined
                            .filter(|x| x.default.is_some())
                            // Map to key, value tuple
                            .map(|x| (x.name.clone(), x.default.clone().unwrap_or_default()))
                            .collect();

                        for (key, value) in &default_values_pairs {
                            default_values.insert(key.clone(), value.clone());
                        }

                        default_values
                    };

                    // Verify that variables without default values have a defined value in each
                    // environment

                    for env_name in env_names.iter() {
                        match &config.env(env_name) {
                            Some(env) => {
                                if !env.contains_key(var) && !default_var_values.contains_key(var) {
                                    parse_errors.push((
                                        ParseError::VariableUndefinedInEnvironment(
                                            var.clone(),
                                            env_name.clone(),
                                        )
                                        .into(),
                                        config_span.clone(),
                                    ));
                                }
                            }
                            None => todo!(),
                        }
                    }
                }
            }

            // Verify template references (variables, prompts, secrets) are defined in the config

            for (ref_type, span) in refs.iter() {
                match ref_type {
                    ReferenceType::Variable(name) => {
                        if let Some((config, _)) = &config {
                            if !config.vars().contains(name) {
                                parse_errors.push((
                                    ReqlangError::ParseError(ParseError::UndefinedReferenceError(
                                        ReferenceType::Variable(name.to_string()),
                                    )),
                                    span.clone(),
                                ));
                            }
                        } else {
                            parse_errors.push((
                                ReqlangError::ParseError(ParseError::UndefinedReferenceError(
                                    ReferenceType::Variable(name.to_string()),
                                )),
                                span.clone(),
                            ));
                        }
                    }
                    ReferenceType::Prompt(name) => {
                        if let Some((config, _)) = &config {
                            if !config.prompts().contains(name) {
                                parse_errors.push((
                                    ReqlangError::ParseError(ParseError::UndefinedReferenceError(
                                        ReferenceType::Prompt(name.to_string()),
                                    )),
                                    span.clone(),
                                ));
                            }
                        } else {
                            parse_errors.push((
                                ReqlangError::ParseError(ParseError::UndefinedReferenceError(
                                    ReferenceType::Prompt(name.to_string()),
                                )),
                                span.clone(),
                            ));
                        }
                    }
                    ReferenceType::Secret(name) => {
                        if let Some((config, _)) = &config {
                            if !config.secrets().contains(name) {
                                parse_errors.push((
                                    ReqlangError::ParseError(ParseError::UndefinedReferenceError(
                                        ReferenceType::Secret(name.to_string()),
                                    )),
                                    span.clone(),
                                ));
                            }
                        } else {
                            parse_errors.push((
                                ReqlangError::ParseError(ParseError::UndefinedReferenceError(
                                    ReferenceType::Secret(name.to_string()),
                                )),
                                span.clone(),
                            ));
                        }
                    }
                    ReferenceType::Provider(_name) => {}
                    ReferenceType::Unknown(_name) => {}
                }
            }

            // // ..

            if let Some((ref config, ref span)) = config {
                let ref_names: Vec<String> = refs
                    .clone()
                    .into_iter()
                    .map(|(x, _)| match x {
                        ReferenceType::Variable(name) => name,
                        ReferenceType::Prompt(name) => name,
                        ReferenceType::Secret(name) => name,
                        ReferenceType::Provider(name) => name,
                        ReferenceType::Unknown(name) => name,
                    })
                    .collect();

                let expr_sources: Vec<String> = exprs
                    .clone()
                    .into_iter()
                    .map(|(expr, _)| expr.clone())
                    .collect();

                for var in &config.vars() {
                    if !ref_names.contains(var) && expr_sources.iter().any(|x| x.contains(var)) {
                        parse_errors.push((
                            ReqlangError::ParseError(ParseError::UnusedValueError(
                                ReferenceType::Variable(var.clone()),
                            )),
                            span.clone(),
                        ))
                    }
                }

                for key in &config.prompts() {
                    if !ref_names.contains(key) && expr_sources.iter().any(|x| x.contains(key)) {
                        parse_errors.push((
                            ReqlangError::ParseError(ParseError::UnusedValueError(
                                ReferenceType::Prompt(key.clone()),
                            )),
                            span.clone(),
                        ))
                    }
                }

                for secret in &config.secrets() {
                    if !ref_names.contains(secret)
                        && expr_sources.iter().any(|x| x.contains(secret))
                    {
                        parse_errors.push((
                            ReqlangError::ParseError(ParseError::UnusedValueError(
                                ReferenceType::Secret(secret.clone()),
                            )),
                            span.clone(),
                        ))
                    }
                }
            }

            if !parse_errors.is_empty() {
                return Err(parse_errors);
            }

            Ok(ParsedRequestFile {
                request: request.unwrap(),
                response,
                config,
                refs,
                exprs,
            })
        }
        None => Err(vec![(ParseError::MissingRequest.into(), 0..0)]),
    }
}

pub fn parse_config(
    config: &Option<Spanned<String>>,
) -> Option<Result<Spanned<ParsedConfig>, Vec<Spanned<ReqlangError>>>> {
    config.as_ref().map(
        |(config, span)| match toml::from_str::<ParsedConfig>(config) {
            Ok(parsed_config) => Ok((parsed_config, span.clone())),
            Err(toml_err) => {
                let toml_span = toml_err.span().unwrap_or(NO_SPAN);
                let err = ReqlangError::ParseError(ParseError::InvalidConfigError {
                    message: toml_err.message().to_string(),
                });
                let err_span = span.start + toml_span.start..span.start + toml_span.end;

                Err(vec![(err, err_span)])
            }
        },
    )
}

/// Extract template references from a string
pub fn parse_references((input, span): &Spanned<String>) -> Vec<Spanned<ReferenceType>> {
    let mut captured_refs: Vec<Spanned<ReferenceType>> = vec![];

    let outer_re = Regex::new(TEMPLATE_REFERENCE_PATTERN).unwrap();
    let inner_re = Regex::new(TEMPLATE_REFERENCE_PATTERN_INNER).unwrap();
    for (_, [inner]) in outer_re.captures_iter(input).map(|cap| cap.extract()) {
        for (_, [prefix, name]) in inner_re.captures_iter(inner).map(|cap| cap.extract()) {
            captured_refs.push(match prefix {
                ":" => (ReferenceType::Variable(name.to_string()), span.to_owned()),
                "?" => (ReferenceType::Prompt(name.to_string()), span.to_owned()),
                "!" => (ReferenceType::Secret(name.to_string()), span.to_owned()),
                "@" => (ReferenceType::Provider(name.to_string()), span.to_owned()),
                _ => (ReferenceType::Unknown(name.to_string()), span.to_owned()),
            });
        }
    }

    captured_refs
}

pub fn parse_inner_references((input, span): &Spanned<String>) -> Vec<Spanned<ReferenceType>> {
    let mut captured_refs: Vec<Spanned<ReferenceType>> = vec![];

    let inner_re = Regex::new(TEMPLATE_REFERENCE_PATTERN_INNER).unwrap();
    for (_, [prefix, name]) in inner_re.captures_iter(input).map(|cap| cap.extract()) {
        captured_refs.push(match prefix {
            ":" => (ReferenceType::Variable(name.to_string()), span.to_owned()),
            "?" => (ReferenceType::Prompt(name.to_string()), span.to_owned()),
            "!" => (ReferenceType::Secret(name.to_string()), span.to_owned()),
            "@" => (ReferenceType::Provider(name.to_string()), span.to_owned()),
            _ => (ReferenceType::Unknown(name.to_string()), span.to_owned()),
        });
    }

    captured_refs
}

/// Extract template references from a string
pub fn parse_expressions((input, _span): &Spanned<String>) -> Vec<Spanned<String>> {
    let mut captured_exprs: Vec<Spanned<String>> = vec![];

    {
        let re = Regex::new(TEMPLATE_EXPR_REFERENCE_PATTERN).unwrap();
        let spans = re.capture_locations();

        for (i, (_, [expr])) in re.captures_iter(input).map(|cap| cap.extract()).enumerate() {
            let expr_span = spans.get(i).unwrap_or((0, 0));
            captured_exprs.push((expr.to_string(), expr_span.0..expr_span.1));
        }
    };

    captured_exprs
}

pub fn parse_request(
    (request, span): &Spanned<String>,
) -> Result<Spanned<HttpRequest>, Vec<Spanned<ReqlangError>>> {
    let mut parse_errors: Vec<Spanned<ReqlangError>> = vec![];

    let mut headers = [httparse::EMPTY_HEADER; 64];
    let mut req = httparse::Request::new(&mut headers);

    let request = format!("{request}\n\n");
    let parse_result = req.parse(request.as_bytes());

    if let Err(error) = parse_result {
        parse_errors.push((
            ParseError::InvalidRequestError {
                message: format!("{error}"),
            }
            .into(),
            span.clone(),
        ));
    }

    if !parse_errors.is_empty() {
        return Err(parse_errors);
    }

    if let httparse::Status::Partial = parse_result.unwrap() {
        parse_errors.push((
            ParseError::InvalidRequestError {
                message: "Unable to parse a partial request".to_string(),
            }
            .into(),
            span.clone(),
        ));
    }

    if !parse_errors.is_empty() {
        return Err(parse_errors);
    }

    let size_minus_body = parse_result.unwrap().unwrap();

    let body = &request[size_minus_body..];

    let mut mapped_headers = vec![];

    req.headers
        .iter_mut()
        .filter(|x| !x.name.is_empty())
        .for_each(|x| {
            mapped_headers.push((
                x.name.to_string(),
                std::str::from_utf8(x.value).unwrap().to_string(),
            ));
        });

    Ok((
        HttpRequest {
            verb: req.method.unwrap().into(),
            target: req.path.unwrap().to_string(),
            http_version: format!("1.{}", req.version.unwrap()).into(),
            headers: mapped_headers,
            body: Some(body.to_string()),
        },
        span.clone(),
    ))
}

pub fn parse_response(
    response: &Option<Spanned<String>>,
) -> Option<Result<Spanned<HttpResponse>, Vec<Spanned<ReqlangError>>>> {
    let mut parse_errors: Vec<Spanned<ReqlangError>> = vec![];

    let mut headers = [httparse::EMPTY_HEADER; 64];
    let mut res = httparse::Response::new(&mut headers);

    let (response, span) = match response {
        Some(x) => x,
        None => return None,
    };

    let response = format!("{response}\n\n");

    let parse_result = res.parse(response.as_bytes());

    match parse_result {
        Ok(result) => match result {
            httparse::Status::Partial => {
                parse_errors.push((
                    ParseError::InvalidRequestError {
                        message: "Unable to parse a partial response".to_string(),
                    }
                    .into(),
                    span.clone(),
                ));

                None
            }
            httparse::Status::Complete(size_minus_body) => {
                let body = &response[size_minus_body..];

                let mut mapped_headers: Vec<(String, String)> = vec![];

                res.headers
                    .iter_mut()
                    .filter(|x| !x.name.is_empty())
                    .for_each(|x| {
                        mapped_headers.push((
                            x.name.to_string(),
                            std::str::from_utf8(x.value).unwrap().to_string(),
                        ));
                    });

                Some(Ok((
                    HttpResponse {
                        http_version: format!("1.{}", res.version.unwrap()).into(),
                        status_code: res
                            .code
                            .unwrap()
                            .to_string()
                            .try_into()
                            .expect("Invalid status code in response"),
                        status_text: res.reason.unwrap().to_string(),
                        headers: mapped_headers,
                        body: Some(body.to_string()),
                    },
                    span.clone(),
                )))
            }
        },
        Err(error) => {
            parse_errors.push((
                ParseError::InvalidRequestError {
                    message: format!("{error}"),
                }
                .into(),
                span.clone(),
            ));

            None
        }
    }
}

#[cfg(test)]
mod test {
    macro_rules! parser_test {
        ($test_name:ident, $reqfile:expr, $result:expr) => {
            #[test]
            fn $test_name() {
                let ast = $crate::ast::Ast::from($reqfile);
                let result = $crate::parser::parse(&ast);
                ::pretty_assertions::assert_eq!($result, result);
            }
        };
    }

    mod invaid {
        use crate::{
            errors::{ParseError, ReqlangError},
            span::NO_SPAN,
            types::ReferenceType,
        };

        // Structure

        parser_test!(
            empty_file,
            "",
            Err(vec![(ParseError::MissingRequest.into(), NO_SPAN)])
        );

        parser_test!(
            request_outside_of_code_fences,
            "GET http://example.com HTTP/1.1\n",
            Err(vec![(ParseError::MissingRequest.into(), 0..0)]) // TODO: Change to 0..0 instead of original 0..32?
        );

        // Undefined References

        parser_test!(
            reference_undefined_variable_in_request,
            textwrap::dedent(
                "
                ```%request
                GET / HTTP/1.1
                test: {{:value}}
                ```
                "
            ),
            Err(vec![(
                ReqlangError::ParseError(ParseError::UndefinedReferenceError(
                    ReferenceType::Variable("value".to_string())
                )),
                13..44
            )])
        );

        parser_test!(
            reference_undefined_prompt_in_request,
            textwrap::dedent(
                "
                ```%request
                GET / HTTP/1.1
                test: {{?value}}
                ```
            "
            ),
            Err(vec![(
                ReqlangError::ParseError(ParseError::UndefinedReferenceError(
                    ReferenceType::Prompt("value".to_string())
                )),
                13..44
            )])
        );

        parser_test!(
            reference_undefined_secret_in_request,
            textwrap::dedent(
                "
                ```%request
                GET / HTTP/1.1
                test: {{!value}}
                ```
                "
            ),
            Err(vec![(
                ReqlangError::ParseError(ParseError::UndefinedReferenceError(
                    ReferenceType::Secret("value".to_string())
                )),
                13..44
            )])
        );

        parser_test!(
            reference_undefined_variable_in_response,
            textwrap::dedent(
                "
                ```%request
                GET / HTTP/1.1
                ```

                ```%response
                HTTP/1.1 200 OK
                test: {{:value}}
                ```
            "
            ),
            Err(vec![(
                ReqlangError::ParseError(ParseError::UndefinedReferenceError(
                    ReferenceType::Variable("value".to_string())
                )),
                46..78
            )])
        );

        parser_test!(
            reference_undefined_prompt_in_reponse,
            textwrap::dedent(
                "
                ```%request
                GET / HTTP/1.1
                ```

                ```%response
                HTTP/1.1 200 OK
                test: {{?value}}
                ```
                "
            ),
            Err(vec![(
                ReqlangError::ParseError(ParseError::UndefinedReferenceError(
                    ReferenceType::Prompt("value".to_string())
                )),
                46..78
            )])
        );

        parser_test!(
            reference_undefined_secret_in_response,
            textwrap::dedent(
                "
                ```%request
                GET / HTTP/1.1
                ```

                ```%response
                HTTP/1.1 200 OK
                test: {{!value}}
                ```
                "
            ),
            Err(vec![(
                ReqlangError::ParseError(ParseError::UndefinedReferenceError(
                    ReferenceType::Secret("value".to_string())
                )),
                46..78
            )])
        );

        // Unused Config Data

        parser_test!(
            unused_variable,
            textwrap::dedent(
                r#"
                ```%config
                [[vars]]
                name = "base_url"
                ```

                ```%request
                GET http://example.com HTTP/1.1
                ```
                "#
            ),
            Err(vec![
                (
                    ParseError::VariableNotDefinedInAnyEnvironment("base_url".to_string()).into(),
                    12..38
                ),
                (
                    ParseError::UnusedValueError(ReferenceType::Variable("base_url".to_string()))
                        .into(),
                    12..38
                )
            ])
        );

        parser_test!(
            unused_prompt,
            textwrap::dedent(
                "
                ```%config
                [[prompts]]
                name = \"base_url\"
                ```

                ```%request
                GET http://example.com HTTP/1.1
                ```
                "
            ),
            Err(vec![(
                ReqlangError::ParseError(ParseError::UnusedValueError(ReferenceType::Prompt(
                    "base_url".to_string()
                ))),
                12..41
            )])
        );

        parser_test!(
            unused_secret,
            textwrap::dedent(
                "
                ```%config
                secrets = [\"base_url\"]
                ```

                ```%request
                GET http://example.com HTTP/1.1
                ```
                "
            ),
            Err(vec![(
                ReqlangError::ParseError(ParseError::UnusedValueError(ReferenceType::Secret(
                    "base_url".to_string()
                ))),
                12..34
            )])
        );

        // Forbidden Request Headers

        parser_test!(
            forbidden_header_host,
            textwrap::dedent(
                "
                ```%request
                GET / HTTP/1.1
                host: example.com
                ```
                "
            ),
            Err(vec![(
                ReqlangError::ParseError(ParseError::ForbiddenRequestHeaderNameError(
                    "host".to_string()
                )),
                13..45
            )])
        );

        parser_test!(
            forbidden_header_host_capitalized,
            textwrap::dedent(
                "
                ```%request
                GET / HTTP/1.1
                Host: example.com
                ```
                "
            ),
            Err(vec![(
                ReqlangError::ParseError(ParseError::ForbiddenRequestHeaderNameError(
                    "host".to_string()
                )),
                13..45
            )])
        );

        parser_test!(
            forbidden_header_host_mixed_capitialization,
            textwrap::dedent(
                "
                ```%request
                GET / HTTP/1.1
                HoST: example.com
                ```
                "
            ),
            Err(vec![(
                ReqlangError::ParseError(ParseError::ForbiddenRequestHeaderNameError(
                    "host".to_string()
                )),
                13..45
            )])
        );

        parser_test!(
            forbidden_header_accept_charset,
            textwrap::dedent(
                "
                ```%request
                GET https://example.com/ HTTP/1.1
                accept-charset: value
                ```
                "
            ),
            Err(vec![(
                ReqlangError::ParseError(ParseError::ForbiddenRequestHeaderNameError(
                    "accept-charset".to_string()
                )),
                13..68
            )])
        );

        parser_test!(
            forbidden_header_accept_encoding,
            textwrap::dedent(
                "
                ```%request
                GET https://example.com/ HTTP/1.1
                accept-encoding: value
                ```
                "
            ),
            Err(vec![(
                ReqlangError::ParseError(ParseError::ForbiddenRequestHeaderNameError(
                    "accept-encoding".to_string()
                )),
                13..69
            )])
        );

        parser_test!(
            forbidden_header_acr_headers,
            textwrap::dedent(
                "
                ```%request
                GET https://example.com/ HTTP/1.1
                access-control-request-headers: value
                ```
                "
            ),
            Err(vec![(
                ReqlangError::ParseError(ParseError::ForbiddenRequestHeaderNameError(
                    "access-control-request-headers".to_string()
                )),
                13..84
            )])
        );

        parser_test!(
            forbidden_header_acr_method,
            textwrap::dedent(
                "
                ```%request
                GET https://example.com/ HTTP/1.1
                access-control-request-method: value
                ```
                "
            ),
            Err(vec![(
                ReqlangError::ParseError(ParseError::ForbiddenRequestHeaderNameError(
                    "access-control-request-method".to_string()
                )),
                13..83
            )])
        );

        parser_test!(
            forbidden_header_connection,
            textwrap::dedent(
                "
                ```%request
                GET https://example.com/ HTTP/1.1
                connection: value
                ```
                "
            ),
            Err(vec![(
                ReqlangError::ParseError(ParseError::ForbiddenRequestHeaderNameError(
                    "connection".to_string()
                )),
                13..64
            )])
        );

        parser_test!(
            forbidden_header_content_length,
            textwrap::dedent(
                "
                ```%request
                GET https://example.com/ HTTP/1.1
                content-length: value
                ```
                "
            ),
            Err(vec![(
                ReqlangError::ParseError(ParseError::ForbiddenRequestHeaderNameError(
                    "content-length".to_string()
                )),
                13..68
            )])
        );

        parser_test!(
            forbidden_header_cookie,
            textwrap::dedent(
                "
                ```%request
                GET https://example.com/ HTTP/1.1
                cookie: value
                ```
                "
            ),
            Err(vec![(
                ReqlangError::ParseError(ParseError::ForbiddenRequestHeaderNameError(
                    "cookie".to_string()
                )),
                13..60
            )])
        );

        parser_test!(
            forbidden_header_date,
            textwrap::dedent(
                "
                ```%request
                GET https://example.com/ HTTP/1.1
                date: value
                ```
                "
            ),
            Err(vec![(
                ReqlangError::ParseError(ParseError::ForbiddenRequestHeaderNameError(
                    "date".to_string()
                )),
                13..58
            )])
        );

        parser_test!(
            forbidden_header_dnt,
            textwrap::dedent(
                "
                ```%request
                GET https://example.com/ HTTP/1.1
                dnt: value
                ```
                "
            ),
            Err(vec![(
                ReqlangError::ParseError(ParseError::ForbiddenRequestHeaderNameError(
                    "dnt".to_string()
                )),
                13..57
            )])
        );

        parser_test!(
            forbidden_header_expect,
            textwrap::dedent(
                "
                ```%request
                GET https://example.com/ HTTP/1.1
                expect: value
                ```
                "
            ),
            Err(vec![(
                ReqlangError::ParseError(ParseError::ForbiddenRequestHeaderNameError(
                    "expect".to_string()
                )),
                13..60
            )])
        );

        parser_test!(
            forbidden_header_keep_alive,
            textwrap::dedent(
                "
                ```%request
                GET https://example.com/ HTTP/1.1
                keep-alive: value
                ```
                "
            ),
            Err(vec![(
                ReqlangError::ParseError(ParseError::ForbiddenRequestHeaderNameError(
                    "keep-alive".to_string()
                )),
                13..64
            )])
        );

        parser_test!(
            forbidden_header_origin,
            textwrap::dedent(
                "
                ```%request
                GET https://example.com/ HTTP/1.1
                origin: value
                ```
                "
            ),
            Err(vec![(
                ReqlangError::ParseError(ParseError::ForbiddenRequestHeaderNameError(
                    "origin".to_string()
                )),
                13..60
            )])
        );

        parser_test!(
            forbidden_header_permission_policy,
            textwrap::dedent(
                "
                ```%request
                GET https://example.com/ HTTP/1.1
                permission-policy: value
                ```
                "
            ),
            Err(vec![(
                ReqlangError::ParseError(ParseError::ForbiddenRequestHeaderNameError(
                    "permission-policy".to_string()
                )),
                13..71
            )])
        );

        parser_test!(
            forbidden_header_te,
            textwrap::dedent(
                "
                ```%request
                GET https://example.com/ HTTP/1.1
                te: value
                ```
                "
            ),
            Err(vec![(
                ReqlangError::ParseError(ParseError::ForbiddenRequestHeaderNameError(
                    "te".to_string()
                )),
                13..56
            )])
        );

        parser_test!(
            forbidden_header_trailer,
            textwrap::dedent(
                "
                ```%request
                GET https://example.com/ HTTP/1.1
                trailer: value
                ```
                "
            ),
            Err(vec![(
                ReqlangError::ParseError(ParseError::ForbiddenRequestHeaderNameError(
                    "trailer".to_string()
                )),
                13..61
            )])
        );

        parser_test!(
            forbidden_header_transfer_encoding,
            textwrap::dedent(
                "
                ```%request
                GET https://example.com/ HTTP/1.1
                transfer-encoding: value
                ```
                "
            ),
            Err(vec![(
                ReqlangError::ParseError(ParseError::ForbiddenRequestHeaderNameError(
                    "transfer-encoding".to_string()
                )),
                13..71
            )])
        );

        parser_test!(
            forbidden_header_upgrade,
            textwrap::dedent(
                "
                ```%request
                GET https://example.com/ HTTP/1.1
                upgrade: value
                ```
                "
            ),
            Err(vec![(
                ReqlangError::ParseError(ParseError::ForbiddenRequestHeaderNameError(
                    "upgrade".to_string()
                )),
                13..61
            )])
        );

        parser_test!(
            forbidden_header_via,
            textwrap::dedent(
                "
                ```%request
                GET https://example.com/ HTTP/1.1
                via: value
                ```
                "
            ),
            Err(vec![(
                ReqlangError::ParseError(ParseError::ForbiddenRequestHeaderNameError(
                    "via".to_string()
                )),
                13..57
            )])
        );

        //

        parser_test!(
            invalid_config_syntax_error_incomplete_table,
            textwrap::dedent(
                r#"
                ```%config
                [[vars]]
                name = "body"

                [envs.dev.body = 123
                ```

                ```%request
                GET https://example.com/ HTTP/1.1
                ```
                "#
            ),
            Err(vec![(
                ReqlangError::ParseError(ParseError::InvalidConfigError {
                    message: "invalid table header\nexpected `.`, `]`".to_string()
                }),
                51..52
            )])
        );

        parser_test!(
            invalid_config_syntax_error_invalid_key_name,
            textwrap::dedent(
                "
                ```%config
                /123=132
                ```

                ```%request
                GET https://example.com/ HTTP/1.1
                ```
                "
            ),
            Err(vec![(
                ReqlangError::ParseError(ParseError::InvalidConfigError {
                    message: "invalid key".to_string()
                }),
                12..13
            )])
        );
    }

    mod valid {
        use std::collections::HashMap;

        use crate::types::{
            ParsedConfig, ParsedConfigPrompt, ParsedConfigVariable, ParsedRequestFile,
            ReferenceType,
            http::{HttpRequest, HttpResponse, HttpStatusCode, HttpVerb, HttpVersion},
        };

        parser_test!(
            just_request_ends_with_single_newline,
            textwrap::dedent(
                "
                ```%request
                GET https://example.com/ HTTP/1.1
                ```
                "
            ),
            Ok(ParsedRequestFile {
                config: None,
                request: (
                    HttpRequest {
                        verb: HttpVerb::get(),
                        target: "https://example.com/".to_string(),
                        http_version: "1.1".into(),
                        headers: vec![],
                        body: Some("".to_string())
                    },
                    13..46
                ),
                response: None,
                refs: vec![],
                exprs: vec![],
            })
        );

        parser_test!(
            response_with_no_newline,
            textwrap::dedent(
                "
                ```%request
                GET http://example.com HTTP/1.1
                ```

                ```%response
                HTTP/1.1 200 OK
                ```
                "
            ),
            Ok(ParsedRequestFile {
                config: None,
                request: (
                    HttpRequest {
                        verb: HttpVerb::get(),
                        target: "http://example.com".to_string(),
                        http_version: "1.1".into(),
                        headers: vec![],
                        body: Some("".to_string())
                    },
                    13..44
                ),
                response: Some((
                    HttpResponse {
                        http_version: HttpVersion::one_point_one(),
                        status_code: HttpStatusCode::new(200),
                        status_text: "OK".to_owned(),
                        headers: vec![],
                        body: Some("".to_string())
                    },
                    63..78
                )),
                refs: vec![],
                exprs: vec![],
            })
        );

        parser_test!(
            template_reference_in_config,
            textwrap::dedent(
                r#"
                ```%config
                [[vars]]
                name = "foo"

                [[vars]]
                name = "bar"

                [envs.dev]
                foo = "test!"
                bar = "{{:foo}}"
                ```

                ```%request
                GET http://example.com?value={{:bar}} HTTP/1.1
                ```
                "#
            ),
            Ok(ParsedRequestFile {
                config: Some((
                    ParsedConfig {
                        vars: Some(vec![
                            ParsedConfigVariable {
                                name: "foo".to_string(),
                                default: None,
                            },
                            ParsedConfigVariable {
                                name: "bar".to_string(),
                                default: None,
                            }
                        ]),
                        envs: Some(HashMap::from([(
                            "dev".to_string(),
                            HashMap::from([
                                ("foo".to_string(), "test!".to_string()),
                                ("bar".to_string(), "{{:foo}}".to_string())
                            ])
                        ),])),
                        prompts: None,
                        secrets: None,
                        auth: None
                    },
                    12..99
                )),
                request: (
                    HttpRequest {
                        verb: HttpVerb::get(),
                        target: "http://example.com?value={{:bar}}".to_string(),
                        http_version: "1.1".into(),
                        headers: vec![],
                        body: Some("".to_string())
                    },
                    117..163
                ),
                response: None,
                refs: vec![
                    (ReferenceType::Variable("bar".to_string()), 117..163),
                    (ReferenceType::Variable("foo".to_string()), 12..99),
                ],
                exprs: vec![],
            })
        );

        parser_test!(
            full_request_file,
            textwrap::dedent(
                r#"
                ```%config
                secrets = ["api_key"]

                [[vars]]
                name = "query_value"
                
                [envs.dev]
                query_value = "dev_value"
                [envs.prod]
                query_value = "prod_value"

                [[prompts]]
                name = "test_value"
                
                [[prompts]]
                name = "expected_response_body"

                ```

                ```%request
                POST /?query={{:query_value}} HTTP/1.1
                x-test: {{?test_value}}
                x-api-key: {{!api_key}}
                x-provider: {{@provider}}

                [1, 2, 3]
                ```

                ```%response
                HTTP/1.1 200 OK

                {{?expected_response_body}}

                ```
                "#
            ),
            Ok(ParsedRequestFile {
                request: (
                    HttpRequest {
                        verb: HttpVerb::post(),
                        target: "/?query={{:query_value}}".to_string(),
                        http_version: "1.1".into(),
                        headers: vec![
                            ("x-test".to_string(), "{{?test_value}}".to_string()),
                            ("x-api-key".to_string(), "{{!api_key}}".to_string()),
                            ("x-provider".to_string(), "{{@provider}}".to_string()),
                        ],
                        body: Some("[1, 2, 3]\n\n".to_string())
                    },
                    238..361
                ),
                response: Some((
                    HttpResponse {
                        http_version: "1.1".into(),
                        status_code: HttpStatusCode::new(200),
                        status_text: "OK".to_string(),
                        headers: vec![],
                        body: Some("{{?expected_response_body}}\n\n\n".to_string())
                    },
                    380..425
                )),
                config: Some((
                    ParsedConfig {
                        vars: Some(vec![ParsedConfigVariable {
                            name: "query_value".to_string(),
                            default: None,
                        }]),
                        envs: Some(HashMap::from([
                            (
                                "dev".to_string(),
                                HashMap::from([(
                                    "query_value".to_string(),
                                    "dev_value".to_string()
                                )])
                            ),
                            (
                                "prod".to_string(),
                                HashMap::from([(
                                    "query_value".to_string(),
                                    "prod_value".to_string()
                                )])
                            ),
                        ])),
                        prompts: Some(vec![
                            ParsedConfigPrompt {
                                name: "test_value".to_string(),
                                description: None,
                                default: None,
                            },
                            ParsedConfigPrompt {
                                name: "expected_response_body".to_string(),
                                description: None,
                                default: None,
                            }
                        ]),
                        secrets: Some(vec!["api_key".to_string()]),
                        auth: None
                    },
                    12..220
                )),
                refs: vec![
                    (ReferenceType::Variable("query_value".to_string()), 238..361),
                    (ReferenceType::Prompt("test_value".to_string()), 238..361),
                    (ReferenceType::Secret("api_key".to_string()), 238..361),
                    (ReferenceType::Provider("provider".to_string()), 238..361),
                    (
                        ReferenceType::Prompt("expected_response_body".to_string()),
                        380..425
                    )
                ],
                exprs: vec![],
            })
        );

        parser_test!(
            markdown_request_file,
            textwrap::dedent(
                "
                # Request File As Markdown

                - Request files are also markdown files.
                - [Configuration](#config), [Request](#request), and [Response](#response) are defined using code blocks.
                - Everything else is considered a comment.

                ## Config

                Use a `%config` code block to define the configuration.

                ```%config
                [[prompts]]
                name = \"status_code\"
                description = \"Status code the response will return\"
                ```

                ## Request

                Use a `%request` code block to define the request.

                ```%request
                GET https://httpbin.org/status/{{?status_code}} HTTP/1.1
                ```

                ## Response

                Use a `%response` code block to define the response.

                ```%response
                HTTP/1.1 200 OK
                content-type: application/json

                ```
                "
            ),
            Ok(ParsedRequestFile {
                config: Some((
                    ParsedConfig {
                        vars: None,
                        envs: None,
                        prompts: Some(vec![
                            ParsedConfigPrompt {
                                name: "status_code".to_string(),
                                description: Some("Status code the response will return".to_string()),
                                default: None,
                            }
                        ]),
                        secrets: None,
                        auth: None
                    },
                    299..384
                )),
                request: (
                    HttpRequest {
                        verb: HttpVerb::get(),
                        target: String::from("https://httpbin.org/status/{{?status_code}}"),
                        http_version: HttpVersion::one_point_one(),
                        headers: vec![],
                        body: Some(String::default())
                    },
                    466..522
                ),
                response: Some((
                    HttpResponse {
                        http_version: HttpVersion::one_point_one(),
                        status_code: HttpStatusCode::new(200),
                        status_text: "OK".to_owned(),
                        headers: vec![("content-type".to_string(), "application/json".to_string())],
                        body: Some("\n".to_owned())
                    },
                    608..655
                )),
                refs: vec![
                    (ReferenceType::Prompt(String::from("status_code")), 466..522)
                ],
                exprs: vec![],
            })
        );
    }
}

#[cfg(test)]
mod resolve_tests {
    use std::collections::HashMap;

    macro_rules! resolve_test {
        ($test_name:ident, $reqfile:expr, $env:expr, $result:expr) => {
            #[test]
            fn $test_name() {
                let ast = $crate::ast::Ast::from($reqfile);
                let resolved_reqfile = $crate::parser::parse(&ast).unwrap();

                pretty_assertions::assert_eq!($result, resolved_reqfile.env($env));
            }
        };
    }

    resolve_test!(
        get_default_env_when_no_config_declared,
        textwrap::dedent(
            "
            ```%request
            GET https://example.com HTTP/1.1
            ```
            "
        ),
        "default",
        None
    );

    resolve_test!(
        get_default_env_when_user_env_defined,
        textwrap::dedent(
            r#"
            ```%config
            [[vars]]
            name = "value"

            [envs.test]
            value = "foo"
            ```

            ```%request
            GET https://example.com?{{:value}} HTTP/1.1
            ```
            "#
        ),
        "default",
        None
    );

    resolve_test!(
        get_user_env_when_no_config_declared,
        textwrap::dedent(
            "
            ```%request
            GET https://example.com HTTP/1.1
            ```
            "
        ),
        "test",
        None
    );

    resolve_test!(
        get_user_env_when_user_env_defined,
        textwrap::dedent(
            r#"
            ```%config
            [[vars]]
            name = "value"

            [envs.test]
            value = "foo"
            ```

            ```%request
            GET https://example.com?{{:value}} HTTP/1.1
            ```
            "#
        ),
        "test",
        Some(HashMap::from([("value".to_string(), "foo".to_string())]))
    );

    resolve_test!(
        get_non_existent_env_when_user_env_defined,
        textwrap::dedent(
            r#"
            ```%config
            [[vars]]
            name = "value"

            [envs.test]
            value = "foo"
            ```

            ```%request
            GET https://example.com?{{:value}} HTTP/1.1
            ```
            "#
        ),
        "doesnt_exist",
        None
    );

    resolve_test!(
        get_var_default_value_if_defined_but_not_in_env,
        textwrap::dedent(
            r#"
            ```%config
            [[vars]]
            name = "value"
            default = "foo"

            [envs.test]
            ```

            ```%request
            GET https://example.com?{{:value}} HTTP/1.1
            ```
            "#
        ),
        "test",
        Some(HashMap::from([("value".to_string(), "foo".to_string())]))
    );
}
