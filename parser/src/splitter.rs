use errors::{ParseError, ReqlangError};
use extract_codeblocks::extract_codeblocks;
use span::{Spanned, NO_SPAN};

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

    let requests = extract_codeblocks(input, "%request");
    let responses = extract_codeblocks(input, "%response");
    let configs = extract_codeblocks(input, "%config");

    if requests.is_empty() {
        return Err(vec![(ParseError::MissingRequest.into(), 0..input.len())]);
    }

    let (request, span) = requests.first().unwrap();

    let request: Spanned<String> = (format!("{request}\n\n"), span.clone());

    let response = responses.first().cloned();

    let response: Option<Spanned<String>> =
        response.map(|(response, span)| (format!("{response}\n\n"), span));

    let config: Option<Spanned<String>> = configs.first().cloned();

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
