use errors::{ParseError, ReqlangError};
use regex::Regex;
use span::{Spanned, NO_SPAN};
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

        self.split(input).and_then(|x| {
            let request_refs = self.extract_references(x.request.as_str());
            let response_refs =
                self.extract_references(x.response.clone().unwrap_or_default().as_str());

            let request = match self.parse_request(x.request) {
                Ok(request) => Some(request),
                Err(err) => {
                    parse_errors.extend(err);
                    None
                }
            };

            let response = match self.parse_response(x.response) {
                Some(Ok(response)) => Some(response),
                Some(Err(err)) => {
                    parse_errors.extend(err);
                    None
                }
                None => None,
            };

            let config = match self.parse_config(x.config) {
                Some(Ok(config)) => Some(config),
                Some(Err(err)) => {
                    parse_errors.extend(err);
                    None
                }
                None => None,
            };

            for (refs, _) in request_refs.iter() {
                match refs {
                    ReferenceType::Variable(name) => {
                        if let Some(ref config) = config {
                            if let Some(vars) = &config.vars {
                                if !vars.contains(name) {
                                    parse_errors.push((
                                        ReqlangError::ParseError(
                                            ParseError::UndefinedReferenceError(
                                                ReferenceType::Variable(name.to_string()),
                                            ),
                                        ),
                                        NO_SPAN,
                                    ));
                                }
                            } else {
                                parse_errors.push((
                                    ReqlangError::ParseError(ParseError::UndefinedReferenceError(
                                        ReferenceType::Variable(name.to_string()),
                                    )),
                                    NO_SPAN,
                                ));
                            }
                        } else {
                            parse_errors.push((
                                ReqlangError::ParseError(ParseError::UndefinedReferenceError(
                                    ReferenceType::Variable(name.to_string()),
                                )),
                                NO_SPAN,
                            ));
                        }
                    }
                    ReferenceType::Prompt(name) => {
                        if let Some(ref config) = config {
                            if let Some(prompts) = &config.prompts {
                                if !prompts.contains_key(name) {
                                    parse_errors.push((
                                        ReqlangError::ParseError(
                                            ParseError::UndefinedReferenceError(
                                                ReferenceType::Prompt(name.to_string()),
                                            ),
                                        ),
                                        NO_SPAN,
                                    ));
                                }
                            } else {
                                parse_errors.push((
                                    ReqlangError::ParseError(ParseError::UndefinedReferenceError(
                                        ReferenceType::Prompt(name.to_string()),
                                    )),
                                    NO_SPAN,
                                ));
                            }
                        } else {
                            parse_errors.push((
                                ReqlangError::ParseError(ParseError::UndefinedReferenceError(
                                    ReferenceType::Prompt(name.to_string()),
                                )),
                                NO_SPAN,
                            ));
                        }
                    }
                    ReferenceType::Secret(name) => {
                        if let Some(ref config) = config {
                            if let Some(secrets) = &config.secrets {
                                if !secrets.contains(name) {
                                    parse_errors.push((
                                        ReqlangError::ParseError(
                                            ParseError::UndefinedReferenceError(
                                                ReferenceType::Secret(name.to_string()),
                                            ),
                                        ),
                                        NO_SPAN,
                                    ));
                                }
                            } else {
                                parse_errors.push((
                                    ReqlangError::ParseError(ParseError::UndefinedReferenceError(
                                        ReferenceType::Secret(name.to_string()),
                                    )),
                                    NO_SPAN,
                                ));
                            }
                        } else {
                            parse_errors.push((
                                ReqlangError::ParseError(ParseError::UndefinedReferenceError(
                                    ReferenceType::Secret(name.to_string()),
                                )),
                                NO_SPAN,
                            ));
                        }
                    }
                    ReferenceType::Unknown(_name) => {}
                }
            }

            for (refs, _) in response_refs.iter() {
                match refs {
                    ReferenceType::Variable(name) => {
                        if let Some(ref config) = config {
                            if let Some(vars) = &config.vars {
                                if !vars.contains(name) {
                                    parse_errors.push((
                                        ReqlangError::ParseError(
                                            ParseError::UndefinedReferenceError(
                                                ReferenceType::Variable(name.to_string()),
                                            ),
                                        ),
                                        NO_SPAN,
                                    ));
                                }
                            } else {
                                parse_errors.push((
                                    ReqlangError::ParseError(ParseError::UndefinedReferenceError(
                                        ReferenceType::Variable(name.to_string()),
                                    )),
                                    NO_SPAN,
                                ));
                            }
                        } else {
                            parse_errors.push((
                                ReqlangError::ParseError(ParseError::UndefinedReferenceError(
                                    ReferenceType::Variable(name.to_string()),
                                )),
                                NO_SPAN,
                            ));
                        }
                    }
                    ReferenceType::Prompt(name) => {
                        if let Some(ref config) = config {
                            if let Some(prompts) = &config.prompts {
                                if !prompts.contains_key(name) {
                                    parse_errors.push((
                                        ReqlangError::ParseError(
                                            ParseError::UndefinedReferenceError(
                                                ReferenceType::Prompt(name.to_string()),
                                            ),
                                        ),
                                        NO_SPAN,
                                    ));
                                }
                            } else {
                                parse_errors.push((
                                    ReqlangError::ParseError(ParseError::UndefinedReferenceError(
                                        ReferenceType::Prompt(name.to_string()),
                                    )),
                                    NO_SPAN,
                                ));
                            }
                        } else {
                            parse_errors.push((
                                ReqlangError::ParseError(ParseError::UndefinedReferenceError(
                                    ReferenceType::Prompt(name.to_string()),
                                )),
                                NO_SPAN,
                            ));
                        }
                    }
                    ReferenceType::Secret(name) => {
                        if let Some(ref config) = config {
                            if let Some(secrets) = &config.secrets {
                                if !secrets.contains(name) {
                                    parse_errors.push((
                                        ReqlangError::ParseError(
                                            ParseError::UndefinedReferenceError(
                                                ReferenceType::Secret(name.to_string()),
                                            ),
                                        ),
                                        NO_SPAN,
                                    ));
                                }
                            } else {
                                parse_errors.push((
                                    ReqlangError::ParseError(ParseError::UndefinedReferenceError(
                                        ReferenceType::Secret(name.to_string()),
                                    )),
                                    NO_SPAN,
                                ));
                            }
                        } else {
                            parse_errors.push((
                                ReqlangError::ParseError(ParseError::UndefinedReferenceError(
                                    ReferenceType::Secret(name.to_string()),
                                )),
                                NO_SPAN,
                            ));
                        }
                    }
                    ReferenceType::Unknown(_name) => {}
                }
            }

            if parse_errors.len() > 0 {
                return Err(parse_errors);
            }

            Ok(UnresolvedRequestFile {
                request: (request.unwrap(), NO_SPAN),
                response: response.map(|x| (x, NO_SPAN)),
                config: config.map(|x| (x, NO_SPAN)),
                request_refs,
                response_refs,
                config_refs: vec![],
            })
        })
    }

    /// Map an `Into<ReqlangError>` in to a `Result<T, ReqlangError>`
    fn err<T>(&self, err: impl Into<ReqlangError>) -> Result<T, Vec<Spanned<ReqlangError>>> {
        Err(vec![(err.into(), NO_SPAN)])
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
                NO_SPAN,
            ));
        }

        if documents.len() > 5 {
            parse_errors.push((
                ReqlangError::ParseError(ParseError::TooManyDividersError),
                NO_SPAN,
            ));
        }

        if !parse_errors.is_empty() {
            return Err(parse_errors);
        }

        let mut request = documents.get(1).map(|x| x.to_string()).unwrap();

        // Fixes requests that doesn't end in correct number of new lines
        if !request.ends_with("\n") {
            request = format!("{request}\n\n");
        }

        if request.ends_with("\n") && !request.ends_with("\n\n") {
            request = format!("{request}\n");
        }

        let response = documents
            .get(2)
            .map(|x| x.trim_start().to_string())
            .filter(|x| !x.is_empty());
        let config = documents
            .get(3)
            .map(|x| x.trim().to_string())
            .filter(|x| !x.is_empty());

        Ok(RequestFileSplitUp {
            request,
            response,
            config,
        })
    }

    fn parse_config(
        &self,
        config: Option<String>,
    ) -> Option<Result<UnresolvedRequestFileConfig, Vec<Spanned<ReqlangError>>>> {
        config.map(|c| {
            toml::from_str(&c).map_err(|x| {
                vec![(
                    ReqlangError::ParseError(ParseError::InvalidConfigError {
                        message: x.message().to_string(),
                    }),
                    x.span().unwrap_or(NO_SPAN),
                )]
            })
        })
    }

    /// Extract template references from a string
    fn extract_references(&self, input: &str) -> Vec<Spanned<ReferenceType>> {
        let re = Regex::new(RequestFileParser::TEMPLATE_REFERENCE_PATTERN).unwrap();

        let mut captured_refs: Vec<Spanned<ReferenceType>> = vec![];

        for (_, [prefix, name]) in re.captures_iter(&input).map(|cap| cap.extract()) {
            captured_refs.push(match prefix {
                ":" => (ReferenceType::Variable(name.to_string()), NO_SPAN),
                "?" => (ReferenceType::Prompt(name.to_string()), NO_SPAN),
                "!" => (ReferenceType::Secret(name.to_string()), NO_SPAN),
                _ => (ReferenceType::Unknown(name.to_string()), NO_SPAN),
            });
        }

        return captured_refs;
    }

    fn parse_request(&self, request: String) -> Result<Request, Vec<Spanned<ReqlangError>>> {
        let mut headers = [httparse::EMPTY_HEADER; 64];
        let mut req = httparse::Request::new(&mut headers);

        let parse_result = req.parse(request.as_bytes());

        if let Err(error) = parse_result {
            return self.err(ParseError::InvalidRequestError {
                message: format!("{error}"),
            });
        }

        let size_minus_body = match parse_result.unwrap() {
            httparse::Status::Complete(x) => x,
            httparse::Status::Partial => {
                return self.err(ParseError::InvalidRequestError {
                    message: "Unable to parse a partial request".to_string(),
                })
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

        Ok(Request {
            verb: req.method.unwrap().to_string(),
            target: req.path.unwrap().to_string(),
            http_version: format!("1.{}", req.version.unwrap().to_string()),
            headers: mapped_headers,
            body: Some(body.to_string()),
        })
    }

    fn parse_response(
        &self,
        response: Option<String>,
    ) -> Option<Result<Response, Vec<Spanned<ReqlangError>>>> {
        let mut headers = [httparse::EMPTY_HEADER; 64];
        let mut res = httparse::Response::new(&mut headers);

        let response = match response {
            Some(x) => x,
            None => return None,
        };

        let parse_result = res.parse(response.as_bytes());

        if let Err(error) = parse_result {
            return Some(self.err(ParseError::InvalidRequestError {
                message: format!("{error}"),
            }));
        }

        let size_minus_body = match parse_result.unwrap() {
            httparse::Status::Complete(x) => x,
            httparse::Status::Partial => {
                return Some(self.err(ParseError::InvalidRequestError {
                    message: "Unable to parse a partial response".to_string(),
                }))
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

        Some(Ok(Response {
            http_version: format!("1.{}", res.version.unwrap().to_string()),
            status_code: res.code.unwrap().to_string(),
            status_text: res.reason.unwrap().to_string(),
            headers: mapped_headers,
            body: Some(body.to_string()),
        }))
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
        Err(vec![(ParseError::NoDividersError.into(), NO_SPAN)])
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
        Err(vec![(ParseError::TooManyDividersError.into(), NO_SPAN)])
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
                NO_SPAN
            ),
            response: None,
            request_refs: vec![],
            response_refs: vec![],
            config_refs: vec![],
        })
    );

    parser_test!(
        undefined_variable_reference_in_request,
        concat!("---\n", "GET / HTTP/1.1\n", "test: {{:value}}\n", "---\n"),
        Err(vec![(
            errors::ReqlangError::ParseError(ParseError::UndefinedReferenceError(
                ReferenceType::Variable("value".to_string())
            )),
            NO_SPAN
        )])
    );

    parser_test!(
        undefined_prompt_reference_in_request,
        concat!("---\n", "GET / HTTP/1.1\n", "test: {{?value}}\n", "---\n"),
        Err(vec![(
            errors::ReqlangError::ParseError(ParseError::UndefinedReferenceError(
                ReferenceType::Prompt("value".to_string())
            )),
            NO_SPAN
        )])
    );

    parser_test!(
        undefined_secret_reference_in_request,
        concat!("---\n", "GET / HTTP/1.1\n", "test: {{!value}}\n", "---\n"),
        Err(vec![(
            errors::ReqlangError::ParseError(ParseError::UndefinedReferenceError(
                ReferenceType::Secret("value".to_string())
            )),
            NO_SPAN
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
            NO_SPAN
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
            NO_SPAN
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
            NO_SPAN
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
                NO_SPAN
            ),
            response: None,
            request_refs: vec![],
            response_refs: vec![],
            config_refs: vec![],
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
                NO_SPAN
            ),
            response: None,
            request_refs: vec![],
            response_refs: vec![],
            config_refs: vec![],
        })
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
                NO_SPAN
            )),
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
                NO_SPAN
            ),
            response: Some((
                Response {
                    http_version: "1.1".to_string(),
                    status_code: "200".to_string(),
                    status_text: "OK".to_string(),
                    headers: HashMap::new(),
                    body: Some("{{?expected_response_body}}\n\n".to_string())
                },
                NO_SPAN
            )),
            request_refs: vec![
                (ReferenceType::Variable("base_url".to_string()), NO_SPAN),
                (ReferenceType::Prompt("test_value".to_string()), NO_SPAN),
                (ReferenceType::Secret("api_key".to_string()), NO_SPAN)
            ],
            response_refs: vec![(
                ReferenceType::Prompt("expected_response_body".to_string()),
                NO_SPAN
            )],
            config_refs: vec![],
        })
    );
}

/// Delimiter used to split request files
const DELIMITER: &str = "---\n";

struct RequestFileSplitUp {
    request: String,
    response: Option<String>,
    config: Option<String>,
}
