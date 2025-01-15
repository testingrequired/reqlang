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
            Err(vec![(ParseError::EmptyFileError.into(), NO_SPAN)])
        );

        parser_test!(
            bare_request,
            "GET http://example.com HTTP/1.1\n",
            Err(vec![(ParseError::NoDividersError.into(), 0..32)])
        );

        parser_test!(
            too_many_doc_dividers_with_5,
            concat!(
                "---\n",
                "GET http://example.com HTTP/1.1\n",
                "---\n",
                "---\n",
                "---\n",
                "---\n"
            ),
            Err(vec![(ParseError::TooManyDividersError.into(), 0..52)])
        );

        parser_test!(
            too_many_doc_dividers_with_4,
            concat!(
                "---\n",
                "GET http://example.com HTTP/1.1\n",
                "---\n",
                "---\n",
                "---\n"
            ),
            Err(vec![(ParseError::TooManyDividersError.into(), 0..48)])
        );

        // Undefined References

        parser_test!(
            reference_undefined_variable_in_request,
            concat!("---\n", "GET / HTTP/1.1\n", "test: {{:value}}\n", "---\n"),
            Err(vec![(
                errors::ReqlangError::ParseError(ParseError::UndefinedReferenceError(
                    ReferenceType::Variable("value".to_string())
                )),
                4..36
            )])
        );

        parser_test!(
            reference_undefined_prompt_in_request,
            concat!("---\n", "GET / HTTP/1.1\n", "test: {{?value}}\n", "---\n"),
            Err(vec![(
                errors::ReqlangError::ParseError(ParseError::UndefinedReferenceError(
                    ReferenceType::Prompt("value".to_string())
                )),
                4..36
            )])
        );

        parser_test!(
            reference_undefined_secret_in_request,
            concat!("---\n", "GET / HTTP/1.1\n", "test: {{!value}}\n", "---\n"),
            Err(vec![(
                errors::ReqlangError::ParseError(ParseError::UndefinedReferenceError(
                    ReferenceType::Secret("value".to_string())
                )),
                4..36
            )])
        );

        parser_test!(
            reference_undefined_variable_in_response,
            concat!(
                "---\n",
                "GET / HTTP/1.1\n",
                "---\n",
                "HTTP/1.1 200 OK\n",
                "test: {{:value}}\n\n",
                "---\n"
            ),
            Err(vec![(
                errors::ReqlangError::ParseError(ParseError::UndefinedReferenceError(
                    ReferenceType::Variable("value".to_string())
                )),
                23..57
            )])
        );

        parser_test!(
            reference_undefined_prompt_in_reponse,
            concat!(
                "---\n",
                "GET / HTTP/1.1\n",
                "---\n",
                "HTTP/1.1 200 OK\n",
                "test: {{?value}}\n\n",
                "---\n"
            ),
            Err(vec![(
                errors::ReqlangError::ParseError(ParseError::UndefinedReferenceError(
                    ReferenceType::Prompt("value".to_string())
                )),
                23..57
            )])
        );

        parser_test!(
            reference_undefined_secret_in_response,
            concat!(
                "---\n",
                "GET / HTTP/1.1\n",
                "---\n",
                "HTTP/1.1 200 OK\n",
                "test: {{!value}}\n\n",
                "---\n"
            ),
            Err(vec![(
                errors::ReqlangError::ParseError(ParseError::UndefinedReferenceError(
                    ReferenceType::Secret("value".to_string())
                )),
                23..57
            )])
        );

        // Unused Config Data

        parser_test!(
            unused_variable,
            concat!(
                "vars = [\"base_url\"]\n",
                "---\n",
                "GET http://example.com HTTP/1.1\n\n",
                "---\n",
                "---\n"
            ),
            Err(vec![(
                errors::ReqlangError::ParseError(ParseError::UnusedValueError(
                    ReferenceType::Variable("base_url".to_string())
                )),
                0..20
            )])
        );

        parser_test!(
            unused_prompt,
            concat!(
                "[prompts]\n",
                "base_url = \"\"\n",
                "---\n",
                "GET http://example.com HTTP/1.1\n\n",
                "---\n",
                "---\n"
            ),
            Err(vec![(
                errors::ReqlangError::ParseError(ParseError::UnusedValueError(
                    ReferenceType::Prompt("base_url".to_string())
                )),
                0..24
            )])
        );

        parser_test!(
            unused_secret,
            concat!(
                "secrets = [\"base_url\"]\n",
                "---\n",
                "GET http://example.com HTTP/1.1\n\n",
                "---\n",
                "---\n"
            ),
            Err(vec![(
                errors::ReqlangError::ParseError(ParseError::UnusedValueError(
                    ReferenceType::Secret("base_url".to_string())
                )),
                0..23
            )])
        );

        // Forbidden Request Headers

        parser_test!(
            forbidden_header_host,
            concat!(
                "---\n",
                "GET / HTTP/1.1\n",
                "host: example.com\n",
                "---\n",
                "---\n"
            ),
            Err(vec![(
                errors::ReqlangError::ParseError(ParseError::ForbiddenRequestHeaderNameError(
                    "host".to_string()
                )),
                4..37
            )])
        );

        parser_test!(
            forbidden_header_host_capitalized,
            concat!(
                "---\n",
                "GET / HTTP/1.1\n",
                "Host: example.com\n",
                "---\n",
                "---\n"
            ),
            Err(vec![(
                errors::ReqlangError::ParseError(ParseError::ForbiddenRequestHeaderNameError(
                    "host".to_string()
                )),
                4..37
            )])
        );

        parser_test!(
            forbidden_header_host_mixed_capitialization,
            concat!(
                "---\n",
                "GET / HTTP/1.1\n",
                "hOsT: example.com\n",
                "---\n",
                "---\n"
            ),
            Err(vec![(
                errors::ReqlangError::ParseError(ParseError::ForbiddenRequestHeaderNameError(
                    "host".to_string()
                )),
                4..37
            )])
        );

        parser_test!(
            forbidden_header_accept_charset,
            concat!(
                "---\n",
                "GET / HTTP/1.1\n",
                "accept-charset: value\n",
                "---\n",
                "---\n"
            ),
            Err(vec![(
                errors::ReqlangError::ParseError(ParseError::ForbiddenRequestHeaderNameError(
                    "accept-charset".to_string()
                )),
                4..41
            )])
        );

        parser_test!(
            forbidden_header_accept_encoding,
            concat!(
                "---\n",
                "GET / HTTP/1.1\n",
                "accept-encoding: value\n",
                "---\n",
                "---\n"
            ),
            Err(vec![(
                errors::ReqlangError::ParseError(ParseError::ForbiddenRequestHeaderNameError(
                    "accept-encoding".to_string()
                )),
                4..42
            )])
        );

        parser_test!(
            forbidden_header_acr_headers,
            concat!(
                "---\n",
                "GET / HTTP/1.1\naccess-control-request-headers: value\n",
                "---\n",
                "---\n"
            ),
            Err(vec![(
                errors::ReqlangError::ParseError(ParseError::ForbiddenRequestHeaderNameError(
                    "access-control-request-headers".to_string()
                )),
                4..57
            )])
        );

        parser_test!(
            forbidden_header_acr_method,
            concat!(
                "---\n",
                "GET / HTTP/1.1\n",
                "access-control-request-method: value\n",
                "---\n",
                "---\n"
            ),
            Err(vec![(
                errors::ReqlangError::ParseError(ParseError::ForbiddenRequestHeaderNameError(
                    "access-control-request-method".to_string()
                )),
                4..56
            )])
        );

        parser_test!(
            forbidden_header_connection,
            concat!(
                "---\n",
                "GET / HTTP/1.1\n",
                "connection: value\n",
                "---\n",
                "---\n"
            ),
            Err(vec![(
                errors::ReqlangError::ParseError(ParseError::ForbiddenRequestHeaderNameError(
                    "connection".to_string()
                )),
                4..37
            )])
        );

        parser_test!(
            forbidden_header_content_length,
            concat!(
                "---\n",
                "GET / HTTP/1.1\n",
                "content-length: value\n",
                "---\n",
                "---\n"
            ),
            Err(vec![(
                errors::ReqlangError::ParseError(ParseError::ForbiddenRequestHeaderNameError(
                    "content-length".to_string()
                )),
                4..41
            )])
        );

        parser_test!(
            forbidden_header_cookie,
            concat!(
                "---\n",
                "GET / HTTP/1.1\n",
                "cookie: value\n",
                "---\n",
                "---\n"
            ),
            Err(vec![(
                errors::ReqlangError::ParseError(ParseError::ForbiddenRequestHeaderNameError(
                    "cookie".to_string()
                )),
                4..33
            )])
        );

        parser_test!(
            forbidden_header_date,
            concat!(
                "---\n",
                "GET / HTTP/1.1\n",
                "date: value\n",
                "---\n",
                "---\n"
            ),
            Err(vec![(
                errors::ReqlangError::ParseError(ParseError::ForbiddenRequestHeaderNameError(
                    "date".to_string()
                )),
                4..31
            )])
        );

        parser_test!(
            forbidden_header_dnt,
            concat!(
                "---\n",
                "GET / HTTP/1.1\n",
                "dnt: value\n",
                "---\n",
                "---\n"
            ),
            Err(vec![(
                errors::ReqlangError::ParseError(ParseError::ForbiddenRequestHeaderNameError(
                    "dnt".to_string()
                )),
                4..30
            )])
        );

        parser_test!(
            forbidden_header_expect,
            concat!(
                "---\n",
                "GET / HTTP/1.1\n",
                "expect: value\n",
                "---\n",
                "---\n"
            ),
            Err(vec![(
                errors::ReqlangError::ParseError(ParseError::ForbiddenRequestHeaderNameError(
                    "expect".to_string()
                )),
                4..33
            )])
        );

        parser_test!(
            forbidden_header_keep_alive,
            concat!(
                "---\n",
                "GET / HTTP/1.1\n",
                "keep-alive: value\n",
                "---\n",
                "---\n"
            ),
            Err(vec![(
                errors::ReqlangError::ParseError(ParseError::ForbiddenRequestHeaderNameError(
                    "keep-alive".to_string()
                )),
                4..37
            )])
        );

        parser_test!(
            forbidden_header_origin,
            concat!(
                "---\n",
                "GET / HTTP/1.1\n",
                "origin: value\n",
                "---\n",
                "---\n"
            ),
            Err(vec![(
                errors::ReqlangError::ParseError(ParseError::ForbiddenRequestHeaderNameError(
                    "origin".to_string()
                )),
                4..33
            )])
        );

        parser_test!(
            forbidden_header_permission_policy,
            concat!(
                "---\n",
                "GET / HTTP/1.1\n",
                "permission-policy: value\n",
                "---\n",
                "---\n"
            ),
            Err(vec![(
                errors::ReqlangError::ParseError(ParseError::ForbiddenRequestHeaderNameError(
                    "permission-policy".to_string()
                )),
                4..44
            )])
        );

        parser_test!(
            forbidden_header_te,
            concat!("---\n", "GET / HTTP/1.1\n", "te: value\n", "---\n", "---\n"),
            Err(vec![(
                errors::ReqlangError::ParseError(ParseError::ForbiddenRequestHeaderNameError(
                    "te".to_string()
                )),
                4..29
            )])
        );

        parser_test!(
            forbidden_header_trailer,
            concat!(
                "---\n",
                "GET / HTTP/1.1\n",
                "trailer: value\n",
                "---\n",
                "---\n"
            ),
            Err(vec![(
                errors::ReqlangError::ParseError(ParseError::ForbiddenRequestHeaderNameError(
                    "trailer".to_string()
                )),
                4..34
            )])
        );

        parser_test!(
            forbidden_header_transfer_encoding,
            concat!(
                "---\n",
                "GET / HTTP/1.1\n",
                "transfer-encoding: value\n",
                "---\n",
                "---\n"
            ),
            Err(vec![(
                errors::ReqlangError::ParseError(ParseError::ForbiddenRequestHeaderNameError(
                    "transfer-encoding".to_string()
                )),
                4..44
            )])
        );

        parser_test!(
            forbidden_header_upgrade,
            concat!(
                "---\n",
                "GET / HTTP/1.1\n",
                "upgrade: value\n",
                "---\n",
                "---\n"
            ),
            Err(vec![(
                errors::ReqlangError::ParseError(ParseError::ForbiddenRequestHeaderNameError(
                    "upgrade".to_string()
                )),
                4..34
            )])
        );

        parser_test!(
            forbidden_header_via,
            concat!(
                "---\n",
                "GET / HTTP/1.1\n",
                "via: value\n",
                "---\n",
                "---\n"
            ),
            Err(vec![(
                errors::ReqlangError::ParseError(ParseError::ForbiddenRequestHeaderNameError(
                    "via".to_string()
                )),
                4..30
            )])
        );

        //

        parser_test!(
            invalid_config_syntax_error_incomplete_table,
            concat!(
                "vars = [\"body\"]\n",
                "[envs.dev.body = 123\n",
                "---\n",
                "GET / HTTP/1.1\n",
                "\n",
                "---\n",
                "---\n"
            ),
            Err(vec![(
                errors::ReqlangError::ParseError(ParseError::InvalidConfigError {
                    message: "invalid table header\nexpected `.`, `]`".to_string()
                }),
                31..32
            )])
        );

        parser_test!(
            invalid_config_syntax_error_invalid_key_name,
            concat!(
                "/123=123\n",
                "---\n",
                "GET http://example.com HTTP/1.1\n",
                "---\n",
                "---\n"
            ),
            Err(vec![(
                errors::ReqlangError::ParseError(ParseError::InvalidConfigError {
                    message: "invalid key".to_string()
                }),
                0..1
            )])
        );
    }

    mod valid {
        use std::collections::HashMap;

        use span::NO_SPAN;
        use types::{
            http::{HttpRequest, HttpResponse, HttpStatusCode, HttpVerb},
            ParsedConfig, ParsedRequestFile, ReferenceType,
        };

        parser_test!(
            just_request_ends_with_no_newline,
            concat!("---\n", "GET http://example.com HTTP/1.1", "---\n"),
            Ok(ParsedRequestFile {
                config: Some((
                    ParsedConfig {
                        vars: None,
                        envs: Some(HashMap::from([("default".to_string(), HashMap::new())])),
                        prompts: None,
                        secrets: None,
                        auth: None
                    },
                    0..0
                )),
                request: (HttpRequest::get("http://example.com", "1.1", vec![]), 4..35),
                response: None,
                refs: vec![],
            })
        );

        parser_test!(
            just_request_ends_with_no_newline_with_envs,
            concat!("[envs]\n---\n", "GET http://example.com HTTP/1.1", "---\n"),
            Ok(ParsedRequestFile {
                config: Some((
                    ParsedConfig {
                        vars: None,
                        envs: Some(HashMap::from([("default".to_string(), HashMap::new())])),
                        prompts: None,
                        secrets: None,
                        auth: None
                    },
                    0..7
                )),
                request: (
                    HttpRequest::get("http://example.com", "1.1", vec![]),
                    11..42
                ),
                response: None,
                refs: vec![],
            })
        );

        parser_test!(
            just_request_ends_with_no_newline_or_split,
            concat!("---\nGET http://example.com HTTP/1.1"),
            Ok(ParsedRequestFile {
                config: Some((
                    ParsedConfig {
                        vars: None,
                        envs: Some(HashMap::from([("default".to_string(), HashMap::new())])),
                        prompts: None,
                        secrets: None,
                        auth: None
                    },
                    NO_SPAN
                )),
                request: (HttpRequest::get("http://example.com", "1.1", vec![]), 4..35),
                response: None,
                refs: vec![],
            })
        );

        parser_test!(
            just_request_ends_with_single_newline,
            concat!("---\n", "GET http://example.com HTTP/1.1\n", "---\n"),
            Ok(ParsedRequestFile {
                config: Some((
                    ParsedConfig {
                        vars: None,
                        envs: Some(HashMap::from([("default".to_string(), HashMap::new())])),
                        prompts: None,
                        secrets: None,
                        auth: None
                    },
                    0..0
                )),
                request: (
                    HttpRequest {
                        verb: HttpVerb::get(),
                        target: "http://example.com".to_string(),
                        http_version: "1.1".into(),
                        headers: vec![],
                        body: Some("".to_string())
                    },
                    4..36
                ),
                response: None,
                refs: vec![]
            })
        );

        parser_test!(
            just_request_ends_with_multiple_newlines,
            concat!("---\n", "GET http://example.com HTTP/1.1\n\n", "---\n"),
            Ok(ParsedRequestFile {
                config: Some((
                    ParsedConfig {
                        vars: None,
                        envs: Some(HashMap::from([("default".to_string(), HashMap::new())])),
                        prompts: None,
                        secrets: None,
                        auth: None
                    },
                    0..0
                )),
                request: (
                    HttpRequest {
                        verb: HttpVerb::get(),
                        target: "http://example.com".to_string(),
                        http_version: "1.1".into(),
                        headers: vec![],
                        body: Some("".to_string())
                    },
                    4..37
                ),
                response: None,
                refs: vec![],
            })
        );

        parser_test!(
            template_reference_in_config,
            concat!(
                "vars = [\"foo\", \"bar\"]\n",
                "envs.dev.foo = \"test!\"\n",
                "envs.dev.bar = \"{{:foo}}\"\n",
                "---\n",
                "GET http://example.com?value={{:bar}} HTTP/1.1\n\n",
                "---\n",
                "---\n"
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
                    0..71
                )),
                request: (
                    HttpRequest {
                        verb: HttpVerb::get(),
                        target: "http://example.com?value={{:bar}}".to_string(),
                        http_version: "1.1".into(),
                        headers: vec![],
                        body: Some("".to_string())
                    },
                    75..123
                ),
                response: None,
                refs: vec![
                    (ReferenceType::Variable("bar".to_string()), 75..123),
                    (ReferenceType::Variable("foo".to_string()), 0..71),
                ],
            })
        );

        parser_test!(
            full_request_file,
            concat!(
                "vars = [\"query_value\"]\n",
                "secrets = [\"api_key\"]",
                "\n",
                "[envs]\n",
                "[envs.dev]\n",
                "query_value = \"dev_value\"\n",
                "\n",
                "[envs.prod]\n",
                "query_value = \"prod_value\"\n",
                "\n",
                "[prompts]\n",
                "test_value = \"\"\n",
                "expected_response_body = \"\"\n",
                "\n",
                "---\n",
                "POST /?query={{:query_value}} HTTP/1.1\n",
                "x-test: {{?test_value}}\n",
                "x-api-key: {{!api_key}}\n",
                "x-provider: {{@provider}}\n",
                "\n",
                "[1, 2, 3]\n",
                "\n",
                "---\n",
                "HTTP/1.1 200 OK\n",
                "\n",
                "{{?expected_response_body}}\n",
                "\n",
                "---\n"
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
                    189..314
                ),
                response: Some((
                    HttpResponse {
                        http_version: "1.1".into(),
                        status_code: HttpStatusCode::new(200),
                        status_text: "OK".to_string(),
                        headers: HashMap::new(),
                        body: Some("{{?expected_response_body}}\n\n".to_string())
                    },
                    318..364
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
                    0..185
                )),
                refs: vec![
                    (ReferenceType::Variable("query_value".to_string()), 189..314),
                    (ReferenceType::Prompt("test_value".to_string()), 189..314),
                    (ReferenceType::Secret("api_key".to_string()), 189..314),
                    (ReferenceType::Provider("provider".to_string()), 189..314),
                    (
                        ReferenceType::Prompt("expected_response_body".to_string()),
                        318..364
                    )
                ],
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
        concat!("---\n", "GET https://example.com HTTP/1.1\n"),
        "default",
        Some(HashMap::new())
    );

    resolve_test!(
        get_default_env_when_default_env_defined,
        concat!(
            "vars = [\"value\"]\n",
            "envs.default.value = \"foo\"\n",
            "---\n",
            "GET https://example.com?{{:value}} HTTP/1.1\n"
        ),
        "default",
        Some(HashMap::from([("value".to_string(), "foo".to_string())]))
    );

    resolve_test!(
        get_default_env_when_user_env_defined,
        concat!(
            "vars = [\"value\"]\n",
            "envs.test.value = \"foo\"\n",
            "---\n",
            "GET https://example.com?{{:value}} HTTP/1.1\n"
        ),
        "default",
        None
    );

    resolve_test!(
        get_user_env_when_no_config_declared,
        concat!("---\n", "GET https://example.com HTTP/1.1\n"),
        "test",
        None
    );

    resolve_test!(
        get_user_env_when_default_env_defined,
        concat!(
            "vars = [\"value\"]\n",
            "envs.default.value = \"foo\"\n",
            "---\n",
            "GET https://example.com?{{:value}} HTTP/1.1\n"
        ),
        "test",
        None
    );

    resolve_test!(
        get_user_env_when_user_env_defined,
        concat!(
            "vars = [\"value\"]\n",
            "envs.test.value = \"foo\"\n",
            "---\n",
            "GET https://example.com?{{:value}} HTTP/1.1\n"
        ),
        "test",
        Some(HashMap::from([("value".to_string(), "foo".to_string())]))
    );

    resolve_test!(
        get_non_existent_env_when_user_env_defined,
        concat!(
            "vars = [\"value\"]\n",
            "envs.test.value = \"foo\"\n",
            "---\n",
            "GET https://example.com?{{:value}} HTTP/1.1\n"
        ),
        "doesnt_exist",
        None
    );
}
