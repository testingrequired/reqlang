use serde::{Deserialize, Serialize};
use types::http::{HttpResponse, HttpStatusCode, HttpVersion};

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

/// Asserts that the `actual` response matches the `expected` response. Returns an error if there are any differences.
pub fn assert_response(
    expected: &HttpResponse,
    actual: &HttpResponse,
) -> Result<(), Vec<ResponseDiff>> {
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

    for key in expected.headers.keys() {
        if !actual.headers.contains_key(key) {
            differences.push(ResponseDiff::MissingHeader(key.to_string()));
        } else {
            let expected_header_value = expected.headers.get(key).unwrap();
            let actual_header_value = actual.headers.get(key).unwrap();

            if actual_header_value != expected_header_value {
                differences.push(ResponseDiff::MismatchHeaderValue {
                    header: key.to_string(),
                    expected: expected_header_value.clone(),
                    actual: actual_header_value.clone(),
                });
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
        return Err(differences);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use types::http::{HttpStatusCode, HttpVersion};

    use super::*;

    use pretty_assertions::assert_eq;

    #[test]
    fn test_assert_exact_matching_responses() {
        let expected = HttpResponse {
            http_version: HttpVersion::one_point_one(),
            status_code: HttpStatusCode::new(200),
            status_text: "OK".to_string(),
            headers: HashMap::from([("Content-Type".to_string(), "application/json".to_string())]),
            body: Some(r#"{"key": "value"}"#.to_string()),
        };

        let actual = HttpResponse {
            http_version: HttpVersion::one_point_one(),
            status_code: HttpStatusCode::new(200),
            status_text: "OK".to_string(),
            headers: HashMap::from([("Content-Type".to_string(), "application/json".to_string())]),
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
            headers: HashMap::from([("Content-Type".to_string(), "application/json".to_string())]),
            body: Some(r#"{"key": "value"}"#.to_string()),
        };

        let actual = HttpResponse {
            http_version: HttpVersion::one_point_one(),
            status_code: HttpStatusCode::new(201),
            status_text: "CREATED".to_string(),
            headers: HashMap::from([("Content-Type".to_string(), "application/json".to_string())]),
            body: Some(r#"{"key": "value"}"#.to_string()),
        };

        assert_eq!(
            Err(vec![
                ResponseDiff::StatusCode {
                    expected: HttpStatusCode::new(200),
                    actual: HttpStatusCode::new(201)
                },
                ResponseDiff::StatusText {
                    expected: "OK".to_string(),
                    actual: "CREATED".to_string()
                }
            ]),
            assert_response(&expected, &actual)
        )
    }

    #[test]
    fn test_missing_header() {
        let expected = HttpResponse {
            http_version: HttpVersion::one_point_one(),
            status_code: HttpStatusCode::new(200),
            status_text: "OK".to_string(),
            headers: HashMap::from([
                ("Content-Type".to_string(), "application/json".to_string()),
                ("X-Custom-Header".to_string(), "custom-value".to_string()),
            ]),
            body: Some(r#"{"key": "value"}"#.to_string()),
        };

        let actual = HttpResponse {
            http_version: HttpVersion::one_point_one(),
            status_code: HttpStatusCode::new(200),
            status_text: "OK".to_string(),
            headers: HashMap::from([("Content-Type".to_string(), "application/json".to_string())]),
            body: Some(r#"{"key": "value"}"#.to_string()),
        };

        assert_eq!(
            Err(vec![ResponseDiff::MissingHeader(
                "X-Custom-Header".to_string()
            )]),
            assert_response(&expected, &actual)
        )
    }

    #[test]
    fn test_extra_header() {
        let expected = HttpResponse {
            http_version: HttpVersion::one_point_one(),
            status_code: HttpStatusCode::new(200),
            status_text: "OK".to_string(),
            headers: HashMap::from([("Content-Type".to_string(), "application/json".to_string())]),
            body: Some(r#"{"key": "value"}"#.to_string()),
        };

        let actual = HttpResponse {
            http_version: HttpVersion::one_point_one(),
            status_code: HttpStatusCode::new(200),
            status_text: "OK".to_string(),
            headers: HashMap::from([
                ("Content-Type".to_string(), "application/json".to_string()),
                ("X-Custom-Header".to_string(), "custom-value".to_string()),
            ]),
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
            headers: HashMap::from([("Content-Type".to_string(), "application/json".to_string())]),
            body: Some(r#"{"key": "value"}"#.to_string()),
        };

        let actual = HttpResponse {
            http_version: HttpVersion::one_point_one(),
            status_code: HttpStatusCode::new(200),
            status_text: "OK".to_string(),
            headers: HashMap::from([("Content-Type".to_string(), "text/plain".to_string())]),
            body: Some(r#"{"key": "value"}"#.to_string()),
        };

        assert_eq!(
            Err(vec![ResponseDiff::MismatchHeaderValue {
                header: "Content-Type".to_string(),
                expected: "application/json".to_string(),
                actual: "text/plain".to_string(),
            }]),
            assert_response(&expected, &actual)
        )
    }

    #[test]
    fn test_mismatch_body_text() {
        let expected = HttpResponse {
            http_version: HttpVersion::one_point_one(),
            status_code: HttpStatusCode::new(200),
            status_text: "OK".to_string(),
            headers: HashMap::from([("Content-Type".to_string(), "text/plain".to_string())]),
            body: Some("Hello World!".to_string()),
        };

        let actual = HttpResponse {
            http_version: HttpVersion::one_point_one(),
            status_code: HttpStatusCode::new(200),
            status_text: "OK".to_string(),
            headers: HashMap::from([("Content-Type".to_string(), "text/plain".to_string())]),
            body: Some("Greetings World!".to_string()),
        };

        assert_eq!(
            Err(vec![ResponseDiff::Body {
                expected: Some(String::from("Hello World!")),
                actual: Some(String::from("Greetings World!")),
            }]),
            assert_response(&expected, &actual)
        )
    }

    #[test]
    fn test_mismatch_body_none() {
        let expected = HttpResponse {
            http_version: HttpVersion::one_point_one(),
            status_code: HttpStatusCode::new(200),
            status_text: "OK".to_string(),
            headers: HashMap::from([("Content-Type".to_string(), "text/plain".to_string())]),
            body: Some("Hello World!".to_string()),
        };

        let actual = HttpResponse {
            http_version: HttpVersion::one_point_one(),
            status_code: HttpStatusCode::new(200),
            status_text: "OK".to_string(),
            headers: HashMap::from([("Content-Type".to_string(), "text/plain".to_string())]),
            body: None,
        };

        assert_eq!(
            Err(vec![ResponseDiff::Body {
                expected: Some(String::from("Hello World!")),
                actual: None,
            }]),
            assert_response(&expected, &actual)
        )
    }
}
