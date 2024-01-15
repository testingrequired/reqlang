use logos::{Logos, SpannedIter};

use errors::{ErrorS, LexicalError};
use token::Token;

pub struct Lexer<'input> {
    token_stream: SpannedIter<'input, Token>,
    input: &'input str,
}

impl<'input> Lexer<'input> {
    pub fn new(input: &'input str) -> Self {
        Self {
            input,
            token_stream: Token::lexer(input).spanned(),
        }
    }
}

impl<'input> Iterator for Lexer<'input> {
    type Item = Result<(usize, Token, usize), ErrorS>;

    fn next(&mut self) -> Option<Self::Item> {
        self.token_stream.next().map(|(token, span)| match token {
            Ok(token) => Ok((span.start, token, span.end)),
            Err(_) => Err((
                LexicalError::InvalidToken(self.input[span.start..span.end].to_string()),
                span.start..span.end,
            )),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use errors::LexicalError;

    macro_rules! lex_test {
        ($test_name:ident, $request:expr, $tokens:expr) => {
            #[test]
            fn $test_name() {
                let exp: Vec<
                    Result<(usize, Token, usize), (LexicalError, std::ops::Range<usize>)>,
                > = $tokens;
                let got: Vec<
                    Result<(usize, Token, usize), (LexicalError, std::ops::Range<usize>)>,
                > = Lexer::new($request).collect::<Vec<_>>();
                assert_eq!(exp, got);
            }
        };
    }

    lex_test!(
        lex_invalid_token,
        "@",
        vec![Err((LexicalError::InvalidToken("@".to_string()), 0..1))]
    );

    lex_test!(
        lex_get_request,
        concat!("---\n", "GET http://example.com HTTP/1.1\n", "---\n",),
        vec![
            Ok((0, Token::TripleDash, 3)),
            Ok((3, Token::NL, 4)),
            Ok((4, Token::Verb("GET".to_string()), 7)),
            Ok((7, Token::SP, 8)),
            Ok((8, Token::Url("http://example.com".to_string()), 26)),
            Ok((26, Token::SP, 27)),
            Ok((27, Token::HttpVersion("HTTP/1.1".to_string()), 35)),
            Ok((35, Token::NL, 36)),
            Ok((36, Token::TripleDash, 39)),
            Ok((39, Token::NL, 40)),
        ]
    );

    lex_test!(
        lex_get_request_with_single_header,
        concat!(
            "---\n",
            "GET / HTTP/1.1\n",
            "host: http://example.com\n",
            "accept: text/html\n",
            "---\n",
        ),
        vec![
            Ok((0, Token::TripleDash, 3)),
            Ok((3, Token::NL, 4)),
            Ok((4, Token::Verb("GET".to_string()), 7)),
            Ok((7, Token::SP, 8)),
            Ok((8, Token::Url("/".to_string()), 9)),
            Ok((9, Token::SP, 10)),
            Ok((10, Token::HttpVersion("HTTP/1.1".to_string()), 18)),
            Ok((18, Token::NL, 19)),
            Ok((
                19,
                Token::HeaderKeyValue(("host".to_string(), "http://example.com".to_string())),
                43
            )),
            Ok((43, Token::NL, 44)),
            Ok((
                44,
                Token::HeaderKeyValue(("accept".to_string(), "text/html".to_string())),
                61
            )),
            Ok((61, Token::NL, 62)),
            Ok((62, Token::TripleDash, 65)),
            Ok((65, Token::NL, 66)),
        ]
    );

    lex_test!(
        lex_post_request,
        concat!("---\n", "POST http://example.com HTTP/1.1\n", "---\n",),
        vec![
            Ok((0, Token::TripleDash, 3)),
            Ok((3, Token::NL, 4)),
            Ok((4, Token::Verb("POST".to_string()), 8)),
            Ok((8, Token::SP, 9)),
            Ok((9, Token::Url("http://example.com".to_string()), 27)),
            Ok((27, Token::SP, 28)),
            Ok((28, Token::HttpVersion("HTTP/1.1".to_string()), 36)),
            Ok((36, Token::NL, 37)),
            Ok((37, Token::TripleDash, 40)),
            Ok((40, Token::NL, 41)),
        ]
    );

    lex_test!(
        lex_put_request,
        concat!("---\n", "PUT http://example.com HTTP/1.1\n", "---\n",),
        vec![
            Ok((0, Token::TripleDash, 3)),
            Ok((3, Token::NL, 4)),
            Ok((4, Token::Verb("PUT".to_string()), 7)),
            Ok((7, Token::SP, 8)),
            Ok((8, Token::Url("http://example.com".to_string()), 26)),
            Ok((26, Token::SP, 27)),
            Ok((27, Token::HttpVersion("HTTP/1.1".to_string()), 35)),
            Ok((35, Token::NL, 36)),
            Ok((36, Token::TripleDash, 39)),
            Ok((39, Token::NL, 40)),
        ]
    );

    lex_test!(
        lex_delete_request,
        concat!("---\n", "DELETE http://example.com HTTP/1.1\n", "---\n",),
        vec![
            Ok((0, Token::TripleDash, 3)),
            Ok((3, Token::NL, 4)),
            Ok((4, Token::Verb("DELETE".to_string()), 10)),
            Ok((10, Token::SP, 11)),
            Ok((11, Token::Url("http://example.com".to_string()), 29)),
            Ok((29, Token::SP, 30)),
            Ok((30, Token::HttpVersion("HTTP/1.1".to_string()), 38)),
            Ok((38, Token::NL, 39)),
            Ok((39, Token::TripleDash, 42)),
            Ok((42, Token::NL, 43)),
        ]
    );

    lex_test!(
        lex_head_request,
        concat!("---\n", "HEAD http://example.com HTTP/1.1\n", "---\n",),
        vec![
            Ok((0, Token::TripleDash, 3)),
            Ok((3, Token::NL, 4)),
            Ok((4, Token::Verb("HEAD".to_string()), 8)),
            Ok((8, Token::SP, 9)),
            Ok((9, Token::Url("http://example.com".to_string()), 27)),
            Ok((27, Token::SP, 28)),
            Ok((28, Token::HttpVersion("HTTP/1.1".to_string()), 36)),
            Ok((36, Token::NL, 37)),
            Ok((37, Token::TripleDash, 40)),
            Ok((40, Token::NL, 41)),
        ]
    );

    lex_test!(
        lex_options_request,
        concat!("---\n", "OPTIONS http://example.com HTTP/1.1\n", "---\n",),
        vec![
            Ok((0, Token::TripleDash, 3)),
            Ok((3, Token::NL, 4)),
            Ok((4, Token::Verb("OPTIONS".to_string()), 11)),
            Ok((11, Token::SP, 12)),
            Ok((12, Token::Url("http://example.com".to_string()), 30)),
            Ok((30, Token::SP, 31)),
            Ok((31, Token::HttpVersion("HTTP/1.1".to_string()), 39)),
            Ok((39, Token::NL, 40)),
            Ok((40, Token::TripleDash, 43)),
            Ok((43, Token::NL, 44)),
        ]
    );

    lex_test!(
        lex_patch_request,
        concat!("---\n", "PATCH http://example.com HTTP/1.1\n", "---\n",),
        vec![
            Ok((0, Token::TripleDash, 3)),
            Ok((3, Token::NL, 4)),
            Ok((4, Token::Verb("PATCH".to_string()), 9)),
            Ok((9, Token::SP, 10)),
            Ok((10, Token::Url("http://example.com".to_string()), 28)),
            Ok((28, Token::SP, 29)),
            Ok((29, Token::HttpVersion("HTTP/1.1".to_string()), 37)),
            Ok((37, Token::NL, 38)),
            Ok((38, Token::TripleDash, 41)),
            Ok((41, Token::NL, 42)),
        ]
    );

    lex_test!(
        lex_connect_request,
        concat!("---\n", "CONNECT http://example.com HTTP/1.1\n", "---\n",),
        vec![
            Ok((0, Token::TripleDash, 3)),
            Ok((3, Token::NL, 4)),
            Ok((4, Token::Verb("CONNECT".to_string()), 11)),
            Ok((11, Token::SP, 12)),
            Ok((12, Token::Url("http://example.com".to_string()), 30)),
            Ok((30, Token::SP, 31)),
            Ok((31, Token::HttpVersion("HTTP/1.1".to_string()), 39)),
            Ok((39, Token::NL, 40)),
            Ok((40, Token::TripleDash, 43)),
            Ok((43, Token::NL, 44)),
        ]
    );

    lex_test!(
        lex_trace_request,
        concat!("---\n", "TRACE http://example.com HTTP/1.1\n", "---\n",),
        vec![
            Ok((0, Token::TripleDash, 3)),
            Ok((3, Token::NL, 4)),
            Ok((4, Token::Verb("TRACE".to_string()), 9)),
            Ok((9, Token::SP, 10)),
            Ok((10, Token::Url("http://example.com".to_string()), 28)),
            Ok((28, Token::SP, 29)),
            Ok((29, Token::HttpVersion("HTTP/1.1".to_string()), 37)),
            Ok((37, Token::NL, 38)),
            Ok((38, Token::TripleDash, 41)),
            Ok((41, Token::NL, 42)),
        ]
    );

    lex_test!(
        lex_invalid_verb_request,
        concat!("---\n", "FOO http://example.com HTTP/1.1\n", "---\n",),
        vec![
            Ok((0, Token::TripleDash, 3)),
            Ok((3, Token::NL, 4)),
            Err((LexicalError::InvalidToken("FOO".to_string()), 4..7)),
            Ok((7, Token::SP, 8)),
            Ok((8, Token::Url("http://example.com".to_string()), 26)),
            Ok((26, Token::SP, 27)),
            Ok((27, Token::HttpVersion("HTTP/1.1".to_string()), 35)),
            Ok((35, Token::NL, 36)),
            Ok((36, Token::TripleDash, 39)),
            Ok((39, Token::NL, 40)),
        ]
    );
}
