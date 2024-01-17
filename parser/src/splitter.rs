/// Delimiter used to split request files
const DELIMITER: &str = "---";

/// A request file split up for parsing
#[derive(Clone, Debug, PartialEq, Default)]
pub struct RequestFileSplitUp {
    pub request: String,
    pub response: Option<String>,
    pub config: Option<String>,
}

pub struct RequestFileSplitter<'input> {
    input: &'input str,
}

impl<'input> RequestFileSplitter<'input> {
    pub fn new(input: &'input str) -> Self {
        Self { input }
    }

    pub fn parse(&self) -> Result<RequestFileSplitUp, &'static str> {
        if self.input.is_empty() {
            return Err("Request file is an empty file");
        }

        let documents: Vec<&str> = self.input.split(DELIMITER).collect();

        if documents.len() < 2 {
            return Err("Request file has no document dividers");
        }

        if documents.len() > 5 {
            return Err("Request file has too many document dividers");
        }

        let request = documents.get(1).map(|x| x.trim().to_string()).unwrap();
        let response = documents
            .get(2)
            .map(|x| x.trim().to_string())
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
}

#[cfg(test)]
mod request_file_splitter_tests {
    use super::*;

    macro_rules! splitter_test {
        ($test_name:ident, $reqfile:expr, $result:expr) => {
            #[test]
            fn $test_name() {
                let splitter = RequestFileSplitter::new($reqfile);

                assert_eq!($result, splitter.parse());
            }
        };
    }

    splitter_test!(empty, "", Err("Request file is an empty file"));

    splitter_test!(
        no_doc_dividers,
        "GET http://example.com HTTP/1.1\n",
        Err("Request file has no document dividers")
    );

    splitter_test!(
        too_many_doc_dividers,
        concat!(
            "---\n",
            "GET http://example.com HTTP/1.1\n",
            "---\n",
            "---\n",
            "---\n",
            "---\n"
        ),
        Err("Request file has too many document dividers")
    );
    splitter_test!(
        just_request,
        concat!("---\n", "GET http://example.com HTTP/1.1\n", "---\n",),
        Ok(RequestFileSplitUp {
            request: "GET http://example.com HTTP/1.1".to_string(),
            response: None,
            config: None,
        })
    );

    splitter_test!(
        just_request_with_headers,
        concat!(
            "---\n",
            "GET / HTTP/1.1\n",
            "host: http://example.com\n",
            "\n",
            "---\n",
        ),
        Ok(RequestFileSplitUp {
            request: "GET / HTTP/1.1\nhost: http://example.com".to_string(),
            response: None,
            config: None,
        })
    );

    splitter_test!(
        just_request_with_headers_and_body,
        concat!(
            "---\n",
            "POST / HTTP/1.1\n",
            "host: http://example.com\n",
            "content-type: application/json\n",
            "\n",
            "[1, 2, 3]",
            "---\n",
        ),
        Ok(RequestFileSplitUp {
            request: concat!(
                "POST / HTTP/1.1\n",
                "host: http://example.com\n",
                "content-type: application/json\n",
                "\n",
                "[1, 2, 3]"
            )
            .to_string(),
            response: None,
            config: None,
        })
    );

    splitter_test!(
        request_and_response,
        concat!(
            "---\n",
            "GET http://example.com HTTP/1.1\n",
            "---\n",
            "HTTP/1.1 200 OK\n",
            "---\n"
        ),
        Ok(RequestFileSplitUp {
            request: "GET http://example.com HTTP/1.1".to_string(),
            response: Some("HTTP/1.1 200 OK".to_string()),
            config: None,
        })
    );

    splitter_test!(
        request_and_response_and_config,
        concat!(
            "---\n",
            "GET http://example.com HTTP/1.1\n",
            "---\n",
            "HTTP/1.1 200 OK\n",
            "---\n",
            "value = 123\n",
            "foo = \"bar\"\n",
            "---\n"
        ),
        Ok(RequestFileSplitUp {
            request: "GET http://example.com HTTP/1.1".to_string(),
            response: Some("HTTP/1.1 200 OK".to_string()),
            config: Some("value = 123\nfoo = \"bar\"".to_string()),
        })
    );
}
