use errors::{ParseError, ReqlangError};
use span::{Spanned, NO_SPAN};

use crate::parser::RequestFileSplitUp;

/// Delimiter used to split request files
///
/// Request files must have at least 1-3 document dividers
const DELIMITER: &str = "---\n";

/// Split string in to a [types::HttpRequest], and optional [types::HttpResponse], [types::ParsedConfig]
pub fn split(input: &str) -> Result<RequestFileSplitUp, Vec<Spanned<ReqlangError>>> {
    let mut parse_errors: Vec<Spanned<ReqlangError>> = vec![];

    if input.is_empty() {
        parse_errors.push((
            ReqlangError::ParseError(ParseError::NoDividersError),
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

    if documents.len() > 4 {
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
    if !request.ends_with('\n') {
        request = format!("{request}\n\n");
    }

    if request.ends_with('\n') && !request.ends_with("\n\n") {
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

    let config = documents
        .first()
        .filter(|c| !c.is_empty())
        .map(|v| (v.to_string(), 0..v.len()));

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
            Err(vec![(ParseError::NoDividersError.into(), NO_SPAN)]),
            output
        );
    }

    #[test]
    fn test_whitespace_string() {
        let input = " \n ";
        let output = split(input);

        assert_eq!(
            Err(vec![(ParseError::NoDividersError.into(), 0..3)]),
            output
        );
    }

    #[test]
    fn test_single_delimiter_with_newline() {
        let input = "---\n";
        let output = split(input);

        assert_eq!(
            Ok(RequestFileSplitUp {
                request: (String::from("\n\n"), 4..4),
                response: None,
                config: None
            }),
            output
        );
    }

    #[test]
    fn test_single_delimiter_without_newline() {
        let input = "---";
        let output = split(input);

        assert_eq!(
            Err(vec![(ParseError::NoDividersError.into(), 0..3)]),
            output
        );
    }

    #[test]
    fn test_request_without_response_or_config() {
        let input = "---\nREQUEST";
        let output = split(input);

        assert_eq!(
            Ok(RequestFileSplitUp {
                request: (String::from("REQUEST\n\n"), 4..11),
                response: None,
                config: None
            }),
            output
        );
    }

    #[test]
    fn test_empty_request_with_empty_response_and_empty_config() {
        let input = "---\n---\n---\n";
        let output = split(input);

        assert_eq!(
            Ok(RequestFileSplitUp {
                request: (String::from("\n\n"), 4..4),
                response: None,
                config: None
            }),
            output
        );
    }

    #[test]
    fn test_too_many_delimiters() {
        let input = "---\n---\n---\n---\n";
        let output = split(input);

        assert_eq!(
            Err(vec![(ParseError::TooManyDividersError.into(), 0..16)]),
            output
        );
    }

    #[test]
    fn test_request_with_empty_response_and_empty_config() {
        let input = "---\nREQUEST\n---\n";
        let output = split(input);

        assert_eq!(
            Ok(RequestFileSplitUp {
                request: (String::from("REQUEST\n\n"), 4..12),
                response: None,
                config: None
            }),
            output
        );
    }

    #[test]
    fn test_request_with_response_and_empty_config() {
        let input = "---\nREQUEST\n---\nRESPONSE\n";
        let output = split(input);

        assert_eq!(
            Ok(RequestFileSplitUp {
                request: (String::from("REQUEST\n\n"), 4..12),
                response: Some((String::from("RESPONSE\n"), 16..25)),
                config: None
            }),
            output
        );
    }

    #[test]
    fn test_request_with_response_and_config() {
        let input = "CONFIG\n---\nREQUEST\n---\nRESPONSE\n";
        let output = split(input);

        assert_eq!(
            Ok(RequestFileSplitUp {
                request: (String::from("REQUEST\n\n"), 11..19),
                response: Some((String::from("RESPONSE\n"), 23..32)),
                config: Some((String::from("CONFIG\n"), 0..7))
            }),
            output
        );
    }
}
