use errors::{ParseError, ReqlangError};
use span::{Spanned, NO_SPAN};

use crate::parser::RequestFileSplitUp;

/// Delimiter used to split request files
const DELIMITER: &str = "---\n";

/// Split string in to a [HttpRequest], and optional [HttpResponse], [ParsedConfig]
pub fn split(input: &str) -> Result<RequestFileSplitUp, Vec<Spanned<ReqlangError>>> {
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

    let config_start = 0;

    let config = documents.first();

    let config_end = match config {
        Some(config) => config_start + config.len(),
        None => config_start,
    };

    let config = config
        .map(|x| x.to_string())
        .map(|x| (x, config_start..config_end));

    Ok(RequestFileSplitUp {
        request,
        response,
        config,
    })
}
