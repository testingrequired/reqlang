use errors::{ParseError, ReqlangError};
use regex::Regex;
use span::{Spanned, NO_SPAN};
use std::{collections::HashMap, vec};
use types::{
    http::{HttpRequest, HttpResponse},
    ParsedConfig, ParsedRequestFile, ReferenceType,
};

use crate::{splitter::split, TEMPLATE_REFERENCE_PATTERN};

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

/// Parse string into a [ParsedRequestFile]
pub fn parse(input: &str) -> Result<ParsedRequestFile, Vec<Spanned<ReqlangError>>> {
    let mut parse_errors: Vec<Spanned<ReqlangError>> = vec![];

    split(input).and_then(|reqfile| {
        let request_refs = parse_references(&reqfile.request);
        let response_refs = parse_references(&reqfile.response.clone().unwrap_or_default());
        let config_refs = parse_references(&reqfile.config.clone().unwrap_or_default());
        let mut refs: Vec<(ReferenceType, std::ops::Range<usize>)> = vec![];
        refs.extend(request_refs);
        refs.extend(response_refs);
        refs.extend(config_refs);

        let request = match parse_request(&reqfile.request) {
            Ok((request, span)) => {
                for key in request.headers.iter().map(|x| &x.0) {
                    if FORBIDDEN_REQUEST_HEADER_NAMES.contains(&key.to_lowercase().as_str()) {
                        parse_errors.push((
                            ParseError::ForbiddenRequestHeaderNameError(key.to_lowercase()).into(),
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

        let response = match parse_response(&reqfile.response) {
            Some(Ok(response)) => Some(response),
            Some(Err(err)) => {
                parse_errors.extend(err);
                None
            }
            None => None,
        };

        let config = match parse_config(&reqfile.config) {
            Some(Ok((mut config, config_span))) => {
                if let Some(envs) = &mut config.envs {
                    if envs.keys().len() == 0 {
                        envs.insert("default".to_string(), HashMap::new());
                    }
                } else {
                    let mut envs: HashMap<String, HashMap<String, String>> = HashMap::new();

                    envs.insert("default".to_string(), HashMap::new());

                    config.envs = Some(envs);
                }

                Some((config, config_span))
            }
            Some(Err(err)) => {
                parse_errors.extend(err);
                None
            }
            None => None,
        };

        // Validate template references are declared/defined vars, secrets, prompts, etc.
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

            for var in &config.vars() {
                if !ref_names.contains(var) {
                    parse_errors.push((
                        ReqlangError::ParseError(ParseError::UnusedValueError(
                            ReferenceType::Variable(var.clone()),
                        )),
                        span.clone(),
                    ))
                }
            }

            for key in &config.prompts() {
                if !ref_names.contains(key) {
                    parse_errors.push((
                        ReqlangError::ParseError(ParseError::UnusedValueError(
                            ReferenceType::Prompt(key.clone()),
                        )),
                        span.clone(),
                    ))
                }
            }

            for secret in &config.secrets() {
                if !ref_names.contains(secret) {
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
        })
    })
}

pub fn parse_config(
    config: &Option<Spanned<String>>,
) -> Option<Result<Spanned<ParsedConfig>, Vec<Spanned<ReqlangError>>>> {
    config.as_ref().map(|(config, span)| {
        let config: Result<ParsedConfig, _> = toml::from_str(config);

        config.map(|x| (x, span.clone())).map_err(|x| {
            let toml_span = x.span().unwrap_or(NO_SPAN);
            let err = ReqlangError::ParseError(ParseError::InvalidConfigError {
                message: x.message().to_string(),
            });
            let err_span = span.start + toml_span.start..span.start + toml_span.end;

            vec![(err, err_span)]
        })
    })
}

/// Extract template references from a string
pub fn parse_references((input, span): &Spanned<String>) -> Vec<Spanned<ReferenceType>> {
    let re = Regex::new(TEMPLATE_REFERENCE_PATTERN).unwrap();

    let mut captured_refs: Vec<Spanned<ReferenceType>> = vec![];

    for (_, [prefix, name]) in re.captures_iter(input).map(|cap| cap.extract()) {
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

pub fn parse_request(
    (request, span): &Spanned<String>,
) -> Result<Spanned<HttpRequest>, Vec<Spanned<ReqlangError>>> {
    let mut parse_errors: Vec<Spanned<ReqlangError>> = vec![];

    let mut headers = [httparse::EMPTY_HEADER; 64];
    let mut req = httparse::Request::new(&mut headers);

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

                let mut mapped_headers = HashMap::new();

                res.headers
                    .iter_mut()
                    .filter(|x| !x.name.is_empty())
                    .for_each(|x| {
                        mapped_headers.insert(
                            x.name.to_string(),
                            std::str::from_utf8(x.value).unwrap().to_string(),
                        );
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

#[derive(Debug, PartialEq)]
pub struct RequestFileSplitUp {
    pub request: Spanned<String>,
    pub response: Option<Spanned<String>>,
    pub config: Option<Spanned<String>>,
}

#[cfg(test)]
mod test {
    macro_rules! parser_test {
        ($test_name:ident, $reqfile:expr, $result:expr) => {
            #[test]
            fn $test_name() {
                pretty_assertions::assert_eq!($result, $crate::parse(&$reqfile));
            }
        };
    }

    mod invaid {
        use errors::ParseError;
        use span::NO_SPAN;
        use types::ReferenceType;

        // Structure

        parser_test!(
            empty_file,
            "",
            Err(vec![(ParseError::MissingRequest.into(), NO_SPAN)])
        );

        parser_test!(
            request_outside_of_code_fences,
            "GET http://example.com HTTP/1.1\n",
            Err(vec![(ParseError::MissingRequest.into(), 0..32)])
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
                errors::ReqlangError::ParseError(ParseError::UndefinedReferenceError(
                    ReferenceType::Variable("value".to_string())
                )),
                1..48
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
                errors::ReqlangError::ParseError(ParseError::UndefinedReferenceError(
                    ReferenceType::Prompt("value".to_string())
                )),
                1..48
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
                errors::ReqlangError::ParseError(ParseError::UndefinedReferenceError(
                    ReferenceType::Secret("value".to_string())
                )),
                1..48
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
                errors::ReqlangError::ParseError(ParseError::UndefinedReferenceError(
                    ReferenceType::Variable("value".to_string())
                )),
                33..82
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
                errors::ReqlangError::ParseError(ParseError::UndefinedReferenceError(
                    ReferenceType::Prompt("value".to_string())
                )),
                33..82
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
                errors::ReqlangError::ParseError(ParseError::UndefinedReferenceError(
                    ReferenceType::Secret("value".to_string())
                )),
                33..82
            )])
        );

        // Unused Config Data

        parser_test!(
            unused_variable,
            textwrap::dedent(
                "
                ```%config
                vars = [\"base_url\"]
                ```

                ```%request
                GET http://example.com HTTP/1.1
                ```
                "
            ),
            Err(vec![(
                errors::ReqlangError::ParseError(ParseError::UnusedValueError(
                    ReferenceType::Variable("base_url".to_string())
                )),
                1..35
            )])
        );

        parser_test!(
            unused_prompt,
            textwrap::dedent(
                "
                ```%config
                [prompts]
                base_url = \"\"
                ```

                ```%request
                GET http://example.com HTTP/1.1
                ```
                "
            ),
            Err(vec![(
                errors::ReqlangError::ParseError(ParseError::UnusedValueError(
                    ReferenceType::Prompt("base_url".to_string())
                )),
                1..39
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
                errors::ReqlangError::ParseError(ParseError::UnusedValueError(
                    ReferenceType::Secret("base_url".to_string())
                )),
                1..38
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
                errors::ReqlangError::ParseError(ParseError::ForbiddenRequestHeaderNameError(
                    "host".to_string()
                )),
                1..49
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
                errors::ReqlangError::ParseError(ParseError::ForbiddenRequestHeaderNameError(
                    "host".to_string()
                )),
                1..49
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
                errors::ReqlangError::ParseError(ParseError::ForbiddenRequestHeaderNameError(
                    "host".to_string()
                )),
                1..49
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
                errors::ReqlangError::ParseError(ParseError::ForbiddenRequestHeaderNameError(
                    "accept-charset".to_string()
                )),
                1..72
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
                errors::ReqlangError::ParseError(ParseError::ForbiddenRequestHeaderNameError(
                    "accept-encoding".to_string()
                )),
                1..73
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
                errors::ReqlangError::ParseError(ParseError::ForbiddenRequestHeaderNameError(
                    "access-control-request-headers".to_string()
                )),
                1..88
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
                errors::ReqlangError::ParseError(ParseError::ForbiddenRequestHeaderNameError(
                    "access-control-request-method".to_string()
                )),
                1..87
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
                errors::ReqlangError::ParseError(ParseError::ForbiddenRequestHeaderNameError(
                    "connection".to_string()
                )),
                1..68
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
                errors::ReqlangError::ParseError(ParseError::ForbiddenRequestHeaderNameError(
                    "content-length".to_string()
                )),
                1..72
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
                errors::ReqlangError::ParseError(ParseError::ForbiddenRequestHeaderNameError(
                    "cookie".to_string()
                )),
                1..64
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
                errors::ReqlangError::ParseError(ParseError::ForbiddenRequestHeaderNameError(
                    "date".to_string()
                )),
                1..62
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
                errors::ReqlangError::ParseError(ParseError::ForbiddenRequestHeaderNameError(
                    "dnt".to_string()
                )),
                1..61
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
                errors::ReqlangError::ParseError(ParseError::ForbiddenRequestHeaderNameError(
                    "expect".to_string()
                )),
                1..64
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
                errors::ReqlangError::ParseError(ParseError::ForbiddenRequestHeaderNameError(
                    "keep-alive".to_string()
                )),
                1..68
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
                errors::ReqlangError::ParseError(ParseError::ForbiddenRequestHeaderNameError(
                    "origin".to_string()
                )),
                1..64
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
                errors::ReqlangError::ParseError(ParseError::ForbiddenRequestHeaderNameError(
                    "permission-policy".to_string()
                )),
                1..75
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
                errors::ReqlangError::ParseError(ParseError::ForbiddenRequestHeaderNameError(
                    "te".to_string()
                )),
                1..60
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
                errors::ReqlangError::ParseError(ParseError::ForbiddenRequestHeaderNameError(
                    "trailer".to_string()
                )),
                1..65
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
                errors::ReqlangError::ParseError(ParseError::ForbiddenRequestHeaderNameError(
                    "transfer-encoding".to_string()
                )),
                1..75
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
                errors::ReqlangError::ParseError(ParseError::ForbiddenRequestHeaderNameError(
                    "upgrade".to_string()
                )),
                1..65
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
                errors::ReqlangError::ParseError(ParseError::ForbiddenRequestHeaderNameError(
                    "via".to_string()
                )),
                1..61
            )])
        );

        //

        parser_test!(
            invalid_config_syntax_error_incomplete_table,
            textwrap::dedent(
                "
                ```%config
                vars = [\"body\"]
                [envs.dev.body = 123
                ```

                ```%request
                GET https://example.com/ HTTP/1.1
                ```
                "
            ),
            Err(vec![(
                errors::ReqlangError::ParseError(ParseError::InvalidConfigError {
                    message: "invalid table header\nexpected `.`, `]`".to_string()
                }),
                32..33
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
                errors::ReqlangError::ParseError(ParseError::InvalidConfigError {
                    message: "invalid key".to_string()
                }),
                1..2
            )])
        );
    }

    mod valid {
        use std::collections::HashMap;

        use types::{
            http::{HttpRequest, HttpResponse, HttpStatusCode, HttpVerb, HttpVersion},
            ParsedConfig, ParsedRequestFile, ReferenceType,
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
                    1..50
                ),
                response: None,
                refs: vec![]
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
                    1..48
                ),
                response: Some((
                    HttpResponse {
                        http_version: HttpVersion::one_point_one(),
                        status_code: HttpStatusCode::new(200),
                        status_text: "OK".to_owned(),
                        headers: HashMap::default(),
                        body: Some("".to_string())
                    },
                    50..82
                )),
                refs: vec![],
            })
        );

        parser_test!(
            template_reference_in_config,
            textwrap::dedent(
                "
                ```%config
                vars = [\"foo\", \"bar\"]
                envs.dev.foo = \"test!\"
                envs.dev.bar = \"{{:foo}}\"
                ```

                ```%request
                GET http://example.com?value={{:bar}} HTTP/1.1
                ```
                "
            ),
            Ok(ParsedRequestFile {
                config: Some((
                    ParsedConfig {
                        vars: Some(vec!["foo".to_string(), "bar".to_string()]),
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
                    1..86
                )),
                request: (
                    HttpRequest {
                        verb: HttpVerb::get(),
                        target: "http://example.com?value={{:bar}}".to_string(),
                        http_version: "1.1".into(),
                        headers: vec![],
                        body: Some("".to_string())
                    },
                    88..150
                ),
                response: None,
                refs: vec![
                    (ReferenceType::Variable("bar".to_string()), 88..150),
                    (ReferenceType::Variable("foo".to_string()), 1..86),
                ],
            })
        );

        parser_test!(
            full_request_file,
            textwrap::dedent(
                "
                ```%config
                vars = [\"query_value\"]
                secrets = [\"api_key\"]

                [envs.dev]
                query_value = \"dev_value\"
                [envs.prod]
                query_value = \"prod_value\"

                [prompts]
                test_value = \"\"
                expected_response_body = \"\"

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
                "
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
                    195..334
                ),
                response: Some((
                    HttpResponse {
                        http_version: "1.1".into(),
                        status_code: HttpStatusCode::new(200),
                        status_text: "OK".to_string(),
                        headers: HashMap::new(),
                        body: Some("{{?expected_response_body}}\n\n\n".to_string())
                    },
                    336..398
                )),
                config: Some((
                    ParsedConfig {
                        vars: Some(vec!["query_value".to_string()]),
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
                        prompts: Some(HashMap::from([
                            ("test_value".to_string(), Some("".to_string())),
                            ("expected_response_body".to_string(), Some("".to_string()))
                        ])),
                        secrets: Some(vec!["api_key".to_string()]),
                        auth: None
                    },
                    1..193
                )),
                refs: vec![
                    (ReferenceType::Variable("query_value".to_string()), 195..334),
                    (ReferenceType::Prompt("test_value".to_string()), 195..334),
                    (ReferenceType::Secret("api_key".to_string()), 195..334),
                    (ReferenceType::Provider("provider".to_string()), 195..334),
                    (
                        ReferenceType::Prompt("expected_response_body".to_string()),
                        336..398
                    )
                ],
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
                [prompts]
                # Status code the response will return
                status_code = \"\"
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

                ```
                "
            ),
            Ok(ParsedRequestFile {
                config: Some((
                    ParsedConfig {
                        vars: None,
                        envs: Some(HashMap::from([("default".to_owned(), HashMap::default())])),
                        prompts: Some(HashMap::from([("status_code".to_owned(), Some("".to_owned()))])),
                        secrets: None,
                        auth: None
                    },
                    288..368
                )),
                request: (
                    HttpRequest {
                        verb: HttpVerb::get(),
                        target: String::from("https://httpbin.org/status/{{?status_code}}"),
                        http_version: HttpVersion::one_point_one(),
                        headers: vec![],
                        body: Some(String::default())
                    },
                    434..506
                ),
                response: Some((
                    HttpResponse {
                        http_version: HttpVersion::one_point_one(),
                        status_code: HttpStatusCode::new(200),
                        status_text: "OK".to_owned(),
                        headers: HashMap::default(),
                        body: Some("\n".to_owned())
                    },
                    575..608
                )),
                refs: vec![
                    (ReferenceType::Prompt(String::from("status_code")), 434..506)
                ]
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
                let resolved_reqfile = $crate::parse(&$reqfile).unwrap();

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
        get_default_env_when_default_env_defined,
        textwrap::dedent(
            "
            ```%config
            vars = [\"value\"]

            envs.default.value = \"foo\"
            ```

            ```%request
            GET https://example.com?{{:value}} HTTP/1.1
            ```
            "
        ),
        "default",
        Some(HashMap::from([("value".to_string(), "foo".to_string())]))
    );

    resolve_test!(
        get_default_env_when_user_env_defined,
        textwrap::dedent(
            "
            ```%config
            vars = [\"value\"]

            envs.test.value = \"foo\"
            ```

            ```%request
            GET https://example.com?{{:value}} HTTP/1.1
            ```
            "
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
        get_user_env_when_default_env_defined,
        textwrap::dedent(
            "
            ```%config
            vars = [\"value\"]

            envs.default.value = \"foo\"
            ```

            ```%request
            GET https://example.com?{{:value}} HTTP/1.1
            ```
            "
        ),
        "test",
        None
    );

    resolve_test!(
        get_user_env_when_user_env_defined,
        textwrap::dedent(
            "
            ```%config
            vars = [\"value\"]

            envs.test.value = \"foo\"
            ```

            ```%request
            GET https://example.com?{{:value}} HTTP/1.1
            ```
            "
        ),
        "test",
        Some(HashMap::from([("value".to_string(), "foo".to_string())]))
    );

    resolve_test!(
        get_non_existent_env_when_user_env_defined,
        textwrap::dedent(
            "
            ```%config
            vars = [\"value\"]

            envs.test.value = \"foo\"
            ```

            ```%request
            GET https://example.com?{{:value}} HTTP/1.1
            ```
            "
        ),
        "doesnt_exist",
        None
    );
}
