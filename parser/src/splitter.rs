use errors::{ParseError, ReqlangError};
use markdown::mdast::Node;
use span::{Span, Spanned, NO_SPAN};

use crate::parser::RequestFileSplitUp;

/// Split string in to a [types::HttpRequest], and optional [types::HttpResponse], [types::ParsedConfig]
pub fn split(input: &str) -> Result<RequestFileSplitUp, Vec<Spanned<ReqlangError>>> {
    let mut parse_errors: Vec<Spanned<ReqlangError>> = vec![];

    if input.is_empty() {
        parse_errors.push((
            ReqlangError::ParseError(ParseError::MissingRequest),
            NO_SPAN,
        ));

        return Err(parse_errors);
    }

    let root = markdown::to_mdast(input, &markdown::ParseOptions::default()).unwrap();

    let children: Vec<Node> = root.children().cloned().unwrap_or_default();

    let mut request: Option<String> = None;
    let mut request_span: Option<Span> = None;
    let mut response: Option<String> = None;
    let mut response_span: Option<Span> = None;
    let mut config: Option<String> = None;
    let mut config_span: Option<Span> = None;

    for child in &children {
        if let Node::Code(code) = child {
            let value = &code.value;
            let position = code.position.as_ref().unwrap();
            let start = position.start.offset;
            let end = position.end.offset;

            if let Some(lang) = &code.lang {
                if lang == "%request" {
                    request = Some(value.clone());
                    request_span = Some(start..end)
                } else if lang == "%response" {
                    response = Some(value.clone());
                    response_span = Some(start..end);
                } else if lang == "%config" {
                    config = Some(value.clone());
                    config_span = Some(start..end);
                }
            }
        }
    }

    if request.is_none() {
        return Err(vec![(ParseError::MissingRequest.into(), 0..input.len())]);
    }

    let request: Spanned<String> = (
        request.map(|r| format!("{r}\n\n")).unwrap(),
        request_span.expect("should have a request span from the markdown parsing"),
    );

    let response: Option<Spanned<String>> = response.map(|response| {
        (
            format!("{response}\n\n"),
            response_span.expect("should have a response span from the markdown parsing"),
        )
    });

    let config: Option<Spanned<String>> = config.map(|config| {
        (
            config,
            config_span.expect("should have a config span from the markdown parsing"),
        )
    });

    Ok(RequestFileSplitUp {
        request,
        response,
        config,
    })
}

/// Tests
#[cfg(test)]
mod tests {
    use super::*;

    use pretty_assertions::assert_eq;

    #[test]
    fn test_empty_string() {
        let input = "";
        let output = split(input);

        assert_eq!(
            Err(vec![(ParseError::MissingRequest.into(), NO_SPAN)]),
            output
        );
    }

    #[test]
    fn test_whitespace_string() {
        let input = " \n ";
        let output = split(input);

        assert_eq!(Err(vec![(ParseError::MissingRequest.into(), 0..3)]), output);
    }

    #[test]
    fn test_request_without_response_or_config() {
        let input = textwrap::dedent(
            "
        ```%request
        REQUEST
        ```
        ",
        );

        let output = split(&input);

        assert_eq!(
            Ok(RequestFileSplitUp {
                request: (String::from("REQUEST\n\n"), 1..24),
                response: None,
                config: None
            }),
            output
        );
    }

    #[test]
    fn test_request_with_response_and_empty_config() {
        let input = textwrap::dedent(
            "
        ```%request
        REQUEST
        ```

        ```%response
        RESPONSE
        ```
        ",
        );

        let output = split(&input);

        assert_eq!(
            Ok(RequestFileSplitUp {
                request: (String::from("REQUEST\n\n"), 1..24),
                response: Some((String::from("RESPONSE\n\n"), 26..51)),
                config: None
            }),
            output
        );
    }

    #[test]
    fn test_request_with_response_and_config() {
        let input = textwrap::dedent(
            "
        ```%config
        CONFIG
        ```

        ```%request
        REQUEST
        ```

        ```%response
        RESPONSE
        ```
        ",
        );

        let output = split(&input);

        assert_eq!(
            Ok(RequestFileSplitUp {
                request: (String::from("REQUEST\n\n"), 24..47),
                response: Some((String::from("RESPONSE\n\n"), 49..74)),
                config: Some((String::from("CONFIG"), 1..22))
            }),
            output
        );
    }
}
