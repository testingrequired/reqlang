use std::fmt::{self, Display};

use crate::types::http::{HttpResponse, HttpStatusCode, HttpVersion};
use console::Style;
use serde::{Deserialize, Serialize};
use similar::{ChangeTag, TextDiff};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ResponseDiff {
    StatusCode {
        expected: HttpStatusCode,
        actual: HttpStatusCode,
    },
    StatusText {
        expected: String,
        actual: String,
    },
    HttpVersion {
        expected: HttpVersion,
        actual: HttpVersion,
    },
    MissingHeader(String),
    MismatchHeaderValue {
        header: String,
        expected: String,
        actual: String,
    },
    Body {
        expected: Option<String>,
        actual: Option<String>,
    },
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct ResponseDiffs(Vec<ResponseDiff>, HttpResponse);

impl Display for ResponseDiffs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_diff_string())
    }
}

impl ResponseDiffs {
    pub fn diffs(&self) -> Vec<ResponseDiff> {
        self.0.clone()
    }

    pub fn to_diff_string(&self) -> String {
        let mut http_version_diff: Option<(String, String)> = None;
        let mut status_code_diff: Option<(String, String)> = None;
        let mut status_text_diff: Option<(String, String)> = None;

        let mut header_diffs: Vec<(String, String)> = vec![];
        let mut body_diff: Option<(String, String)> = None;

        for response_diff in self.0.iter() {
            match response_diff {
                ResponseDiff::StatusCode { expected, actual } => {
                    status_code_diff = Some((expected.to_string(), actual.to_string()))
                }
                ResponseDiff::StatusText { expected, actual } => {
                    status_text_diff = Some((expected.to_string(), actual.to_string()))
                }
                ResponseDiff::HttpVersion { expected, actual } => {
                    http_version_diff = Some((expected.to_string(), actual.to_string()))
                }
                ResponseDiff::MissingHeader(header) => {
                    header_diffs.push((format!("{header}: ..."), String::new()));
                }
                ResponseDiff::MismatchHeaderValue {
                    header,
                    expected,
                    actual,
                } => {
                    header_diffs.push((
                        format!("{header}: {expected}"),
                        format!("{header}: {actual}"),
                    ));
                }
                ResponseDiff::Body { expected, actual } => {
                    body_diff = Some((
                        expected.as_ref().cloned().unwrap_or_default(),
                        actual.as_ref().cloned().unwrap_or_default(),
                    ))
                }
            };
        }

        let first_line_diff: Option<(String, String)> =
            match (http_version_diff, status_code_diff, status_text_diff) {
                (None, None, None) => None,
                (None, None, Some((expected_status_text, actual_status_text))) => Some((
                    format!(
                        "HTTP/{} {} {}",
                        self.1.http_version, self.1.status_code, expected_status_text
                    ),
                    format!(
                        "HTTP/{} {} {}",
                        self.1.http_version, self.1.status_code, actual_status_text
                    ),
                )),
                (None, Some((expected_status_code, actual_status_code)), None) => Some((
                    format!(
                        "HTTP/{} {} {}",
                        self.1.http_version, expected_status_code, self.1.status_text
                    ),
                    format!(
                        "HTTP/{} {} {}",
                        self.1.http_version, actual_status_code, self.1.status_text
                    ),
                )),
                (
                    None,
                    Some((expected_status_code, actual_status_code)),
                    Some((expected_status_text, actual_status_text)),
                ) => Some((
                    format!(
                        "HTTP/{} {} {}",
                        self.1.http_version, expected_status_code, expected_status_text
                    ),
                    format!(
                        "HTTP/{} {} {}",
                        self.1.http_version, actual_status_code, actual_status_text
                    ),
                )),
                (Some((expected_http_version, actual_http_version)), None, None) => Some((
                    format!(
                        "HTTP/{} {} {}",
                        expected_http_version, self.1.status_code, self.1.status_text
                    ),
                    format!(
                        "HTTP/{} {} {}",
                        actual_http_version, self.1.status_code, self.1.status_text
                    ),
                )),
                (
                    Some((expected_http_version, actual_http_version)),
                    None,
                    Some((expected_status_text, actual_status_text)),
                ) => Some((
                    format!(
                        "HTTP/{} {} {}",
                        expected_http_version, self.1.status_code, expected_status_text
                    ),
                    format!(
                        "HTTP/{} {} {}",
                        actual_http_version, self.1.status_code, actual_status_text
                    ),
                )),
                (
                    Some((expected_http_version, actual_http_version)),
                    Some((expected_status_code, actual_status_code)),
                    None,
                ) => Some((
                    format!(
                        "HTTP/{} {} {}",
                        expected_http_version, expected_status_code, self.1.status_text
                    ),
                    format!(
                        "HTTP/{} {} {}",
                        actual_http_version, actual_status_code, self.1.status_text
                    ),
                )),
                (
                    Some((expected_http_version, actual_http_version)),
                    Some((expected_status_code, actual_status_code)),
                    Some((expected_status_text, actual_status_text)),
                ) => Some((
                    format!(
                        "HTTP/{} {} {}",
                        expected_http_version, expected_status_code, expected_status_text
                    ),
                    format!(
                        "HTTP/{} {} {}",
                        actual_http_version, actual_status_code, actual_status_text
                    ),
                )),
            };

        let mut output = String::new();

        output.push('\n');

        if let Some((expected, actual)) = &first_line_diff {
            let diff = TextDiff::from_lines(expected, actual);

            for op in diff.ops() {
                for change in diff.iter_changes(op) {
                    let (sign, style) = match change.tag() {
                        ChangeTag::Delete => ("-", Style::new().red()),
                        ChangeTag::Insert => ("+", Style::new().green()),
                        ChangeTag::Equal => (" ", Style::new()),
                    };

                    output.push_str(&format!(
                        "{}{}",
                        style.apply_to(sign).bold(),
                        style.apply_to(change)
                    ));
                }
            }
        }

        for (expected, actual) in header_diffs.iter() {
            let diff = TextDiff::from_lines(expected, actual);

            for op in diff.ops() {
                for change in diff.iter_changes(op) {
                    let (sign, style) = match change.tag() {
                        ChangeTag::Delete => ("-", Style::new().red()),
                        ChangeTag::Insert => ("+", Style::new().green()),
                        ChangeTag::Equal => (" ", Style::new()),
                    };

                    output.push_str(&format!(
                        "{}{}",
                        style.apply_to(sign).bold(),
                        style.apply_to(change)
                    ));
                }
            }
        }

        if let Some((expected, actual)) = &body_diff {
            output.push('\n');

            let diff = TextDiff::from_lines(expected, actual);

            for op in diff.ops() {
                for change in diff.iter_changes(op) {
                    let (sign, style) = match change.tag() {
                        ChangeTag::Delete => ("-", Style::new().red()),
                        ChangeTag::Insert => ("+", Style::new().green()),
                        ChangeTag::Equal => (" ", Style::new()),
                    };

                    output.push_str(&format!(
                        "{}{}",
                        style.apply_to(sign).bold(),
                        style.apply_to(change)
                    ));
                }
            }
        }

        output
    }
}

/// Asserts that the `actual` response matches the `expected` response. Returns an error if there are any differences.
pub fn assert_response(
    expected: &HttpResponse,
    actual: &HttpResponse,
) -> Result<(), Box<ResponseDiffs>> {
    let mut differences: Vec<ResponseDiff> = vec![];

    if expected.status_code != actual.status_code {
        differences.push(ResponseDiff::StatusCode {
            expected: expected.status_code.clone(),
            actual: actual.status_code.clone(),
        });
    }

    if expected.status_text != actual.status_text {
        differences.push(ResponseDiff::StatusText {
            expected: expected.status_text.clone(),
            actual: actual.status_text.clone(),
        });
    }

    for (expected_key, expected_value) in expected.headers.iter() {
        let maybe_actual_header = actual.headers.iter().find(|x| x.0 == *expected_key);

        match maybe_actual_header {
            Some((_, actual_value)) => {
                if actual_value != expected_value {
                    differences.push(ResponseDiff::MismatchHeaderValue {
                        header: expected_key.clone(),
                        expected: expected_value.clone(),
                        actual: actual_value.clone(),
                    })
                }
            }
            None => {
                differences.push(ResponseDiff::MissingHeader(expected_key.clone()));
            }
        }
    }

    if expected.body.is_some() && expected.body != actual.body {
        differences.push(ResponseDiff::Body {
            expected: expected.body.clone(),
            actual: actual.body.clone(),
        });
    }

    if !differences.is_empty() {
        return Err(ResponseDiffs(differences, expected.clone()).into());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::types::http::{HttpStatusCode, HttpVersion};

    use super::*;

    use pretty_assertions::assert_eq;

    #[test]
    fn test_assert_exact_matching_responses() {
        let expected = HttpResponse {
            http_version: HttpVersion::one_point_one(),
            status_code: HttpStatusCode::new(200),
            status_text: "OK".to_string(),
            headers: vec![("Content-Type".to_string(), "application/json".to_string())],
            body: Some(r#"{"key": "value"}"#.to_string()),
        };

        let actual = HttpResponse {
            http_version: HttpVersion::one_point_one(),
            status_code: HttpStatusCode::new(200),
            status_text: "OK".to_string(),
            headers: vec![("Content-Type".to_string(), "application/json".to_string())],
            body: Some(r#"{"key": "value"}"#.to_string()),
        };

        assert_eq!(Ok(()), assert_response(&expected, &actual))
    }

    #[test]
    fn test_mismatched_status() {
        let expected = HttpResponse {
            http_version: HttpVersion::one_point_one(),
            status_code: HttpStatusCode::new(200),
            status_text: "OK".to_string(),
            headers: vec![("Content-Type".to_string(), "application/json".to_string())],
            body: Some(r#"{"key": "value"}"#.to_string()),
        };

        let actual = HttpResponse {
            http_version: HttpVersion::one_point_one(),
            status_code: HttpStatusCode::new(201),
            status_text: "CREATED".to_string(),
            headers: vec![("Content-Type".to_string(), "application/json".to_string())],
            body: Some(r#"{"key": "value"}"#.to_string()),
        };

        assert_eq!(
            Err(ResponseDiffs(
                vec![
                    ResponseDiff::StatusCode {
                        expected: HttpStatusCode::new(200),
                        actual: HttpStatusCode::new(201)
                    },
                    ResponseDiff::StatusText {
                        expected: "OK".to_string(),
                        actual: "CREATED".to_string()
                    }
                ],
                expected.clone()
            )
            .into()),
            assert_response(&expected, &actual)
        )
    }

    #[test]
    fn test_missing_header() {
        let expected = HttpResponse {
            http_version: HttpVersion::one_point_one(),
            status_code: HttpStatusCode::new(200),
            status_text: "OK".to_string(),
            headers: vec![
                ("Content-Type".to_string(), "application/json".to_string()),
                ("X-Custom-Header".to_string(), "custom-value".to_string()),
            ],
            body: Some(r#"{"key": "value"}"#.to_string()),
        };

        let actual = HttpResponse {
            http_version: HttpVersion::one_point_one(),
            status_code: HttpStatusCode::new(200),
            status_text: "OK".to_string(),
            headers: vec![("Content-Type".to_string(), "application/json".to_string())],
            body: Some(r#"{"key": "value"}"#.to_string()),
        };

        assert_eq!(
            Err(ResponseDiffs(
                vec![ResponseDiff::MissingHeader("X-Custom-Header".to_string())],
                expected.clone()
            )
            .into()),
            assert_response(&expected, &actual)
        )
    }

    #[test]
    fn test_extra_header() {
        let expected = HttpResponse {
            http_version: HttpVersion::one_point_one(),
            status_code: HttpStatusCode::new(200),
            status_text: "OK".to_string(),
            headers: vec![("Content-Type".to_string(), "application/json".to_string())],
            body: Some(r#"{"key": "value"}"#.to_string()),
        };

        let actual = HttpResponse {
            http_version: HttpVersion::one_point_one(),
            status_code: HttpStatusCode::new(200),
            status_text: "OK".to_string(),
            headers: vec![
                ("Content-Type".to_string(), "application/json".to_string()),
                ("X-Custom-Header".to_string(), "custom-value".to_string()),
            ],
            body: Some(r#"{"key": "value"}"#.to_string()),
        };

        assert_eq!(Ok(()), assert_response(&expected, &actual))
    }

    #[test]
    fn test_mismatch_header_value() {
        let expected = HttpResponse {
            http_version: HttpVersion::one_point_one(),
            status_code: HttpStatusCode::new(200),
            status_text: "OK".to_string(),
            headers: vec![("Content-Type".to_string(), "application/json".to_string())],
            body: Some(r#"{"key": "value"}"#.to_string()),
        };

        let actual = HttpResponse {
            http_version: HttpVersion::one_point_one(),
            status_code: HttpStatusCode::new(200),
            status_text: "OK".to_string(),
            headers: vec![("Content-Type".to_string(), "text/plain".to_string())],
            body: Some(r#"{"key": "value"}"#.to_string()),
        };

        assert_eq!(
            Err(ResponseDiffs(
                vec![ResponseDiff::MismatchHeaderValue {
                    header: "Content-Type".to_string(),
                    expected: "application/json".to_string(),
                    actual: "text/plain".to_string(),
                }],
                expected.clone()
            )
            .into()),
            assert_response(&expected, &actual)
        )
    }

    #[test]
    fn test_mismatch_body_text() {
        let expected = HttpResponse {
            http_version: HttpVersion::one_point_one(),
            status_code: HttpStatusCode::new(200),
            status_text: "OK".to_string(),
            headers: vec![("Content-Type".to_string(), "text/plain".to_string())],
            body: Some("Hello World!".to_string()),
        };

        let actual = HttpResponse {
            http_version: HttpVersion::one_point_one(),
            status_code: HttpStatusCode::new(200),
            status_text: "OK".to_string(),
            headers: vec![("Content-Type".to_string(), "text/plain".to_string())],
            body: Some("Greetings World!".to_string()),
        };

        assert_eq!(
            Err(ResponseDiffs(
                vec![ResponseDiff::Body {
                    expected: Some(String::from("Hello World!")),
                    actual: Some(String::from("Greetings World!")),
                }],
                expected.clone()
            )
            .into()),
            assert_response(&expected, &actual)
        )
    }

    #[test]
    fn test_mismatch_body_none() {
        let expected = HttpResponse {
            http_version: HttpVersion::one_point_one(),
            status_code: HttpStatusCode::new(200),
            status_text: "OK".to_string(),
            headers: vec![("Content-Type".to_string(), "text/plain".to_string())],
            body: Some("Hello World!".to_string()),
        };

        let actual = HttpResponse {
            http_version: HttpVersion::one_point_one(),
            status_code: HttpStatusCode::new(200),
            status_text: "OK".to_string(),
            headers: vec![("Content-Type".to_string(), "text/plain".to_string())],
            body: None,
        };

        assert_eq!(
            Err(ResponseDiffs(
                vec![ResponseDiff::Body {
                    expected: Some(String::from("Hello World!")),
                    actual: None,
                }],
                expected.clone()
            )
            .into()),
            assert_response(&expected, &actual)
        )
    }
}
