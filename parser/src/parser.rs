use errors::{ParseError, ReqlangError};
use regex::Regex;
use span::{Span, Spanned, NO_SPAN};
use std::{collections::HashMap, vec};
use types::{ReferenceType, Request, Response, UnresolvedRequestFile, UnresolvedRequestFileConfig};

/// Parse a string in to a request file
pub struct RequestFileParser {}

impl RequestFileParser {
    const TEMPLATE_REFERENCE_PATTERN: &'static str = r"\{\{([:?!]{1})([a-zA-Z][_a-zA-Z]+)\}\}";

    pub fn new() -> Self {
        Self {}
    }

    /// Parse a string in to an request file with unresolved template values.
    pub fn parse_string(input: &str) -> Result<UnresolvedRequestFile, Vec<Spanned<ReqlangError>>> {
        RequestFileParser::new().parse(input)
    }

    /// Parse a string in to an request file with unresolved template values.
    pub fn parse(&self, input: &str) -> Result<UnresolvedRequestFile, Vec<Spanned<ReqlangError>>> {
        let mut parse_errors: Vec<Spanned<ReqlangError>> = vec![];

        self.split(input).and_then(|reqfile| {
            let request_refs = self.extract_references(&reqfile.request);
            let response_refs =
                self.extract_references(&reqfile.response.clone().unwrap_or_default());
            let mut refs: Vec<(ReferenceType, std::ops::Range<usize>)> = vec![];

            refs.extend(request_refs);
            refs.extend(response_refs);

            let request = match self.parse_request(&reqfile.request) {
                Ok(request) => Some(request),
                Err(err) => {
                    parse_errors.extend(err);
                    None
                }
            };

            let response = match self.parse_response(&reqfile.response) {
                Some(Ok(response)) => Some(response),
                Some(Err(err)) => {
                    parse_errors.extend(err);
                    None
                }
                None => None,
            };

            let config = match self.parse_config(&reqfile.config) {
                Some(Ok(config)) => Some(config),
                Some(Err(err)) => {
                    parse_errors.extend(err);
                    None
                }
                None => None,
            };

            for (ref_type, span) in refs.iter() {
                match ref_type {
                    ReferenceType::Variable(name) => {
                        if let Some((config, _)) = &config {
                            if let Some(vars) = &config.vars {
                                if !vars.contains(name) {
                                    parse_errors.push((
                                        ReqlangError::ParseError(
                                            ParseError::UndefinedReferenceError(
                                                ReferenceType::Variable(name.to_string()),
                                            ),
                                        ),
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
                            if let Some(prompts) = &config.prompts {
                                if !prompts.contains_key(name) {
                                    parse_errors.push((
                                        ReqlangError::ParseError(
                                            ParseError::UndefinedReferenceError(
                                                ReferenceType::Prompt(name.to_string()),
                                            ),
                                        ),
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
                            if let Some(secrets) = &config.secrets {
                                if !secrets.contains(name) {
                                    parse_errors.push((
                                        ReqlangError::ParseError(
                                            ParseError::UndefinedReferenceError(
                                                ReferenceType::Secret(name.to_string()),
                                            ),
                                        ),
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
                        } else {
                            parse_errors.push((
                                ReqlangError::ParseError(ParseError::UndefinedReferenceError(
                                    ReferenceType::Secret(name.to_string()),
                                )),
                                span.clone(),
                            ));
                        }
                    }
                    ReferenceType::Unknown(_name) => {}
                }
            }

            if let Some((ref config, ref span)) = config {
                let ref_names: Vec<String> = refs
                    .clone()
                    .into_iter()
                    .map(|(x, _)| {
                        let ref_name = match x {
                            ReferenceType::Variable(name) => name,
                            ReferenceType::Prompt(name) => name,
                            ReferenceType::Secret(name) => name,
                            ReferenceType::Unknown(name) => name,
                        };

                        ref_name
                    })
                    .collect();

                if let Some(vars) = &config.vars {
                    for var in vars {
                        if !ref_names.contains(&var) {
                            parse_errors.push((
                                ReqlangError::ParseError(ParseError::UnusedValue(
                                    ReferenceType::Variable(var.clone()),
                                )),
                                span.clone(),
                            ))
                        }
                    }
                }

                if let Some(prompts) = &config.prompts {
                    let keys = prompts.keys();

                    for key in keys {
                        if !ref_names.contains(&key) {
                            parse_errors.push((
                                ReqlangError::ParseError(ParseError::UnusedValue(
                                    ReferenceType::Prompt(key.clone()),
                                )),
                                span.clone(),
                            ))
                        }
                    }
                }

                if let Some(secrets) = &config.secrets {
                    for secret in secrets {
                        if !ref_names.contains(&secret) {
                            parse_errors.push((
                                ReqlangError::ParseError(ParseError::UnusedValue(
                                    ReferenceType::Secret(secret.clone()),
                                )),
                                span.clone(),
                            ))
                        }
                    }
                }
            }

            if parse_errors.len() > 0 {
                return Err(parse_errors);
            }

            Ok(UnresolvedRequestFile {
                request: request.unwrap(),
                response,
                config,
                refs,
            })
        })
    }

    /// Map an `Into<ReqlangError>` in to a `Result<T, ReqlangError>`
    fn err<T>(
        &self,
        err: impl Into<ReqlangError>,
        span: Option<Span>,
    ) -> Result<T, Vec<Spanned<ReqlangError>>> {
        Err(vec![(err.into(), span.unwrap_or(NO_SPAN))])
    }

    /// Split string in to a request, and optional response, config
    fn split(&self, input: &str) -> Result<RequestFileSplitUp, Vec<Spanned<ReqlangError>>> {
        let mut parse_errors: Vec<Spanned<ReqlangError>> = vec![];

        if input.is_empty() {
            parse_errors.push((
                ReqlangError::ParseError(ParseError::EmptyFileError),
                NO_SPAN,
            ));

            return Err(parse_errors);
        }

        let documents: Vec<&str> = input.split(DELIMITER).collect();

        if documents.len() < 2 {
            parse_errors.push((
                ReqlangError::ParseError(ParseError::NoDividersError),
                0..input.len(),
            ));
        }

        if documents.len() > 5 {
            parse_errors.push((
                ReqlangError::ParseError(ParseError::TooManyDividersError),
                0..input.len(),
            ));
        }

        if !parse_errors.is_empty() {
            return Err(parse_errors);
        }

        let first_divider = input.find(DELIMITER).unwrap_or_default();

        let mut request = documents.get(1).map(|x| x.to_string()).unwrap();

        let request_start = first_divider + 4;
        let request_end = request_start + request.len();

        // Fixes requests that doesn't end in correct number of new lines
        if !request.ends_with("\n") {
            request = format!("{request}\n\n");
        }

        if request.ends_with("\n") && !request.ends_with("\n\n") {
            request = format!("{request}\n");
        }

        let request = (request, request_start..request_end);

        let response_start = request_end + 4;

        let response = documents.get(2);

        let response_end = match response {
            Some(response) => response_start + response.len(),
            None => response_start,
        };

        let response = response
            .map(|x| x.trim_start().to_string())
            .filter(|x| !x.is_empty())
            .map(|x| (x, response_start..response_end));

        let config_start = response_end + 4;

        let config = documents.get(3);

        let config_end = match config {
            Some(config) => config_start + config.len(),
            None => config_start,
        };

        let config = config
            .map(|x| x.trim().to_string())
            .filter(|x| !x.is_empty())
            .map(|x| (x, config_start..config_end));

        Ok(RequestFileSplitUp {
            request,
            response,
            config,
        })
    }

    fn parse_config(
        &self,
        config: &Option<Spanned<String>>,
    ) -> Option<Result<Spanned<UnresolvedRequestFileConfig>, Vec<Spanned<ReqlangError>>>> {
        config.as_ref().map(|(config, span)| {
            let config: Result<UnresolvedRequestFileConfig, _> = toml::from_str(&config);

            config.map(|x| (x, span.clone())).map_err(|x| {
                let toml_span = x.span().unwrap_or(NO_SPAN);
                vec![(
                    ReqlangError::ParseError(ParseError::InvalidConfigError {
                        message: x.message().to_string(),
                    }),
                    span.start + toml_span.start..span.start + toml_span.start + toml_span.end,
                )]
            })
        })
    }

    /// Extract template references from a string
    fn extract_references(&self, (input, span): &Spanned<String>) -> Vec<Spanned<ReferenceType>> {
        let re = Regex::new(RequestFileParser::TEMPLATE_REFERENCE_PATTERN).unwrap();

        let mut captured_refs: Vec<Spanned<ReferenceType>> = vec![];

        for (_, [prefix, name]) in re.captures_iter(&input).map(|cap| cap.extract()) {
            captured_refs.push(match prefix {
                ":" => (ReferenceType::Variable(name.to_string()), span.to_owned()),
                "?" => (ReferenceType::Prompt(name.to_string()), span.to_owned()),
                "!" => (ReferenceType::Secret(name.to_string()), span.to_owned()),
                _ => (ReferenceType::Unknown(name.to_string()), span.to_owned()),
            });
        }

        return captured_refs;
    }

    fn parse_request(
        &self,
        (request, span): &Spanned<String>,
    ) -> Result<Spanned<Request>, Vec<Spanned<ReqlangError>>> {
        let mut headers = [httparse::EMPTY_HEADER; 64];
        let mut req = httparse::Request::new(&mut headers);

        let parse_result = req.parse(request.as_bytes());

        if let Err(error) = parse_result {
            return self.err(
                ParseError::InvalidRequestError {
                    message: format!("{error}"),
                },
                Some(span.clone()),
            );
        }

        let size_minus_body = match parse_result.unwrap() {
            httparse::Status::Complete(x) => x,
            httparse::Status::Partial => {
                return self.err(
                    ParseError::InvalidRequestError {
                        message: "Unable to parse a partial request".to_string(),
                    },
                    Some(span.clone()),
                )
            }
        };

        let body = &request[size_minus_body..];

        let mut mapped_headers = HashMap::new();

        req.headers
            .into_iter()
            .filter(|x| !x.name.is_empty())
            .for_each(|x| {
                mapped_headers.insert(
                    x.name.to_string(),
                    std::str::from_utf8(x.value).unwrap().to_string(),
                );
            });

        Ok((
            Request {
                verb: req.method.unwrap().to_string(),
                target: req.path.unwrap().to_string(),
                http_version: format!("1.{}", req.version.unwrap().to_string()),
                headers: mapped_headers,
                body: Some(body.to_string()),
            },
            span.clone(),
        ))
    }

    fn parse_response(
        &self,
        response: &Option<Spanned<String>>,
    ) -> Option<Result<Spanned<Response>, Vec<Spanned<ReqlangError>>>> {
        let mut headers = [httparse::EMPTY_HEADER; 64];
        let mut res = httparse::Response::new(&mut headers);

        let (response, span) = match response {
            Some(x) => x,
            None => return None,
        };

        let parse_result = res.parse(response.as_bytes());

        if let Err(error) = parse_result {
            return Some(self.err(
                ParseError::InvalidRequestError {
                    message: format!("{error}"),
                },
                Some(span.clone()),
            ));
        }

        let size_minus_body = match parse_result.unwrap() {
            httparse::Status::Complete(x) => x,
            httparse::Status::Partial => {
                return Some(self.err(
                    ParseError::InvalidRequestError {
                        message: "Unable to parse a partial response".to_string(),
                    },
                    Some(span.clone()),
                ))
            }
        };

        let body = &response[size_minus_body..];

        let mut mapped_headers = HashMap::new();

        res.headers
            .into_iter()
            .filter(|x| !x.name.is_empty())
            .for_each(|x| {
                mapped_headers.insert(
                    x.name.to_string(),
                    std::str::from_utf8(x.value).unwrap().to_string(),
                );
            });

        Some(Ok((
            Response {
                http_version: format!("1.{}", res.version.unwrap().to_string()),
                status_code: res.code.unwrap().to_string(),
                status_text: res.reason.unwrap().to_string(),
                headers: mapped_headers,
                body: Some(body.to_string()),
            },
            span.clone(),
        )))
    }
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use errors::ParseError;
    use span::NO_SPAN;
    use types::{
        ReferenceType, Request, Response, UnresolvedRequestFile, UnresolvedRequestFileConfig,
    };

    use crate::parser::RequestFileParser;

    macro_rules! parser_test {
        ($test_name:ident, $reqfile:expr, $result:expr) => {
            #[test]
            fn $test_name() {
                assert_eq!($result, RequestFileParser::parse_string(&$reqfile));
            }
        };
    }

    parser_test!(
        empty,
        "",
        Err(vec![(ParseError::EmptyFileError.into(), NO_SPAN)])
    );

    parser_test!(
        no_doc_dividers,
        "GET http://example.com HTTP/1.1\n",
        Err(vec![(ParseError::NoDividersError.into(), 0..32)])
    );

    parser_test!(
        too_many_doc_dividers,
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
        just_request_ends_with_no_newline,
        concat!("---\n", "GET http://example.com HTTP/1.1", "---\n"),
        Ok(UnresolvedRequestFile {
            config: None,
            request: (
                Request {
                    verb: "GET".to_string(),
                    target: "http://example.com".to_string(),
                    http_version: "1.1".to_string(),
                    headers: HashMap::new(),
                    body: Some("".to_string())
                },
                4..35
            ),
            response: None,
            refs: vec![],
        })
    );

    parser_test!(
        undefined_variable_reference_in_request,
        concat!("---\n", "GET / HTTP/1.1\n", "test: {{:value}}\n", "---\n"),
        Err(vec![(
            errors::ReqlangError::ParseError(ParseError::UndefinedReferenceError(
                ReferenceType::Variable("value".to_string())
            )),
            4..36
        )])
    );

    parser_test!(
        undefined_prompt_reference_in_request,
        concat!("---\n", "GET / HTTP/1.1\n", "test: {{?value}}\n", "---\n"),
        Err(vec![(
            errors::ReqlangError::ParseError(ParseError::UndefinedReferenceError(
                ReferenceType::Prompt("value".to_string())
            )),
            4..36
        )])
    );

    parser_test!(
        undefined_secret_reference_in_request,
        concat!("---\n", "GET / HTTP/1.1\n", "test: {{!value}}\n", "---\n"),
        Err(vec![(
            errors::ReqlangError::ParseError(ParseError::UndefinedReferenceError(
                ReferenceType::Secret("value".to_string())
            )),
            4..36
        )])
    );

    parser_test!(
        undefined_variable_reference_in_response,
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
        undefined_prompt_reference_in_response,
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
        undefined_secret_reference_in_response,
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

    parser_test!(
        just_request_ends_with_single_newline,
        concat!("---\n", "GET http://example.com HTTP/1.1\n", "---\n"),
        Ok(UnresolvedRequestFile {
            config: None,
            request: (
                Request {
                    verb: "GET".to_string(),
                    target: "http://example.com".to_string(),
                    http_version: "1.1".to_string(),
                    headers: HashMap::new(),
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
        Ok(UnresolvedRequestFile {
            config: None,
            request: (
                Request {
                    verb: "GET".to_string(),
                    target: "http://example.com".to_string(),
                    http_version: "1.1".to_string(),
                    headers: HashMap::new(),
                    body: Some("".to_string())
                },
                4..37
            ),
            response: None,
            refs: vec![],
        })
    );

    parser_test!(
        unused_variable,
        concat!(
            "---\n",
            "GET http://example.com HTTP/1.1\n\n",
            "---\n",
            "---\n",
            "vars = [\"base_url\"]\n",
            "---\n"
        ),
        Err(vec![(
            errors::ReqlangError::ParseError(ParseError::UnusedValue(ReferenceType::Variable(
                "base_url".to_string()
            ))),
            45..65
        )])
    );

    parser_test!(
        unused_prompt,
        concat!(
            "---\n",
            "GET http://example.com HTTP/1.1\n\n",
            "---\n",
            "---\n",
            "[prompts]\n",
            "base_url = \"\"\n",
            "---\n"
        ),
        Err(vec![(
            errors::ReqlangError::ParseError(ParseError::UnusedValue(ReferenceType::Prompt(
                "base_url".to_string()
            ))),
            45..69
        )])
    );

    parser_test!(
        unused_secret,
        concat!(
            "---\n",
            "GET http://example.com HTTP/1.1\n\n",
            "---\n",
            "---\n",
            "secrets = [\"base_url\"]\n",
            "---\n"
        ),
        Err(vec![(
            errors::ReqlangError::ParseError(ParseError::UnusedValue(ReferenceType::Secret(
                "base_url".to_string()
            ))),
            45..68
        )])
    );

    parser_test!(
        full_request_file,
        concat!(
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
            "\n",
            "{{?expected_response_body}}\n",
            "\n",
            "---\n",
            "vars = [\"base_url\"]\n",
            "secrets = [\"api_key\"]",
            "\n",
            "[envs]\n",
            "[envs.dev]\n",
            "base_url = \"https://dev.example.com\"\n",
            "\n",
            "[envs.prod]\n",
            "base_url = \"https://example.com\"\n",
            "\n",
            "[prompts]\n",
            "test_value = \"\"\n",
            "expected_response_body = \"\"\n",
            "\n",
            "---\n"
        ),
        Ok(UnresolvedRequestFile {
            request: (
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
                4..100
            ),
            response: Some((
                Response {
                    http_version: "1.1".to_string(),
                    status_code: "200".to_string(),
                    status_text: "OK".to_string(),
                    headers: HashMap::new(),
                    body: Some("{{?expected_response_body}}\n\n".to_string())
                },
                104..150
            )),
            config: Some((
                UnresolvedRequestFileConfig {
                    vars: Some(vec!["base_url".to_string()]),
                    envs: Some(HashMap::from([
                        (
                            "dev".to_string(),
                            HashMap::from([(
                                "base_url".to_string(),
                                "https://dev.example.com".to_string()
                            )])
                        ),
                        (
                            "prod".to_string(),
                            HashMap::from([(
                                "base_url".to_string(),
                                "https://example.com".to_string()
                            )])
                        ),
                    ])),
                    prompts: Some(HashMap::from([
                        ("test_value".to_string(), Some("".to_string())),
                        ("expected_response_body".to_string(), Some("".to_string()))
                    ])),
                    secrets: Some(vec!["api_key".to_string()])
                },
                154..353
            )),
            refs: vec![
                (ReferenceType::Variable("base_url".to_string()), 4..100),
                (ReferenceType::Prompt("test_value".to_string()), 4..100),
                (ReferenceType::Secret("api_key".to_string()), 4..100),
                (
                    ReferenceType::Prompt("expected_response_body".to_string()),
                    104..150
                )
            ],
        })
    );
}

/// Delimiter used to split request files
const DELIMITER: &str = "---\n";

struct RequestFileSplitUp {
    request: Spanned<String>,
    response: Option<Spanned<String>>,
    config: Option<Spanned<String>>,
}
