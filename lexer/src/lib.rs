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

    #[test]
    fn lex_invalid_token() {
        let exp: Vec<Result<(usize, Token, usize), (LexicalError, std::ops::Range<usize>)>> =
            vec![Err((LexicalError::InvalidToken("@".to_string()), 0..1))];
        let got: Vec<Result<(usize, Token, usize), (LexicalError, std::ops::Range<usize>)>> =
            Lexer::new("@").collect::<Vec<_>>();
        assert_eq!(exp, got);
    }

    #[test]
    fn lex_get_request() {
        let exp: Vec<Result<(usize, Token, usize), (LexicalError, std::ops::Range<usize>)>> = vec![
            Ok((0, Token::String("GET".to_string()), 3)),
            Ok((3, Token::SP, 4)),
            Ok((4, Token::Url("http://example.com".to_string()), 22)),
            Ok((22, Token::SP, 23)),
            Ok((23, Token::HttpVersion, 31)),
        ];
        let got: Vec<Result<(usize, Token, usize), (LexicalError, std::ops::Range<usize>)>> =
            Lexer::new("GET http://example.com HTTP/1.1").collect::<Vec<_>>();
        assert_eq!(exp, got);
    }

    #[test]
    fn lex_post_request() {
        let exp: Vec<Result<(usize, Token, usize), (LexicalError, std::ops::Range<usize>)>> = vec![
            Ok((0, Token::String("POST".to_string()), 4)),
            Ok((4, Token::SP, 5)),
            Ok((5, Token::Url("http://example.com".to_string()), 23)),
            Ok((23, Token::SP, 24)),
            Ok((24, Token::HttpVersion, 32)),
        ];
        let got: Vec<Result<(usize, Token, usize), (LexicalError, std::ops::Range<usize>)>> =
            Lexer::new("POST http://example.com HTTP/1.1").collect::<Vec<_>>();
        assert_eq!(exp, got);
    }

    #[test]
    fn lex_put_request() {
        let exp: Vec<Result<(usize, Token, usize), (LexicalError, std::ops::Range<usize>)>> = vec![
            Ok((0, Token::String("PUT".to_string()), 3)),
            Ok((3, Token::SP, 4)),
            Ok((4, Token::Url("http://example.com".to_string()), 22)),
            Ok((22, Token::SP, 23)),
            Ok((23, Token::HttpVersion, 31)),
        ];
        let got: Vec<Result<(usize, Token, usize), (LexicalError, std::ops::Range<usize>)>> =
            Lexer::new("PUT http://example.com HTTP/1.1").collect::<Vec<_>>();
        assert_eq!(exp, got);
    }

    #[test]
    fn lex_delete_request() {
        let exp: Vec<Result<(usize, Token, usize), (LexicalError, std::ops::Range<usize>)>> = vec![
            Ok((0, Token::String("DELETE".to_string()), 6)),
            Ok((6, Token::SP, 7)),
            Ok((7, Token::Url("http://example.com".to_string()), 25)),
            Ok((25, Token::SP, 26)),
            Ok((26, Token::HttpVersion, 34)),
        ];
        let got: Vec<Result<(usize, Token, usize), (LexicalError, std::ops::Range<usize>)>> =
            Lexer::new("DELETE http://example.com HTTP/1.1").collect::<Vec<_>>();
        assert_eq!(exp, got);
    }

    #[test]
    fn lex_head_request() {
        let exp: Vec<Result<(usize, Token, usize), (LexicalError, std::ops::Range<usize>)>> = vec![
            Ok((0, Token::String("HEAD".to_string()), 4)),
            Ok((4, Token::SP, 5)),
            Ok((5, Token::Url("http://example.com".to_string()), 23)),
            Ok((23, Token::SP, 24)),
            Ok((24, Token::HttpVersion, 32)),
        ];
        let got: Vec<Result<(usize, Token, usize), (LexicalError, std::ops::Range<usize>)>> =
            Lexer::new("HEAD http://example.com HTTP/1.1").collect::<Vec<_>>();
        assert_eq!(exp, got);
    }

    #[test]
    fn lex_options_request() {
        let exp: Vec<Result<(usize, Token, usize), (LexicalError, std::ops::Range<usize>)>> = vec![
            Ok((0, Token::String("OPTIONS".to_string()), 7)),
            Ok((7, Token::SP, 8)),
            Ok((8, Token::Url("http://example.com".to_string()), 26)),
            Ok((26, Token::SP, 27)),
            Ok((27, Token::HttpVersion, 35)),
        ];
        let got: Vec<Result<(usize, Token, usize), (LexicalError, std::ops::Range<usize>)>> =
            Lexer::new("OPTIONS http://example.com HTTP/1.1").collect::<Vec<_>>();
        assert_eq!(exp, got);
    }

    #[test]
    fn lex_patch_request() {
        let exp: Vec<Result<(usize, Token, usize), (LexicalError, std::ops::Range<usize>)>> = vec![
            Ok((0, Token::String("PATCH".to_string()), 5)),
            Ok((5, Token::SP, 6)),
            Ok((6, Token::Url("http://example.com".to_string()), 24)),
            Ok((24, Token::SP, 25)),
            Ok((25, Token::HttpVersion, 33)),
        ];
        let got: Vec<Result<(usize, Token, usize), (LexicalError, std::ops::Range<usize>)>> =
            Lexer::new("PATCH http://example.com HTTP/1.1").collect::<Vec<_>>();
        assert_eq!(exp, got);
    }

    #[test]
    fn lex_connect_request() {
        let exp: Vec<Result<(usize, Token, usize), (LexicalError, std::ops::Range<usize>)>> = vec![
            Ok((0, Token::String("CONNECT".to_string()), 7)),
            Ok((7, Token::SP, 8)),
            Ok((8, Token::Url("http://example.com".to_string()), 26)),
            Ok((26, Token::SP, 27)),
            Ok((27, Token::HttpVersion, 35)),
        ];
        let got: Vec<Result<(usize, Token, usize), (LexicalError, std::ops::Range<usize>)>> =
            Lexer::new("CONNECT http://example.com HTTP/1.1").collect::<Vec<_>>();
        assert_eq!(exp, got);
    }

    #[test]
    fn lex_trace_request() {
        let exp: Vec<Result<(usize, Token, usize), (LexicalError, std::ops::Range<usize>)>> = vec![
            Ok((0, Token::String("TRACE".to_string()), 5)),
            Ok((5, Token::SP, 6)),
            Ok((6, Token::Url("http://example.com".to_string()), 24)),
            Ok((24, Token::SP, 25)),
            Ok((25, Token::HttpVersion, 33)),
        ];
        let got: Vec<Result<(usize, Token, usize), (LexicalError, std::ops::Range<usize>)>> =
            Lexer::new("TRACE http://example.com HTTP/1.1").collect::<Vec<_>>();
        assert_eq!(exp, got);
    }

    #[test]
    fn lex_nonstandard_verb_request() {
        let exp: Vec<Result<(usize, Token, usize), (LexicalError, std::ops::Range<usize>)>> = vec![
            Ok((0, Token::String("FOO".to_string()), 3)),
            Ok((3, Token::SP, 4)),
            Ok((4, Token::Url("http://example.com".to_string()), 22)),
            Ok((22, Token::SP, 23)),
            Ok((23, Token::HttpVersion, 31)),
        ];
        let got: Vec<Result<(usize, Token, usize), (LexicalError, std::ops::Range<usize>)>> =
            Lexer::new("FOO http://example.com HTTP/1.1").collect::<Vec<_>>();
        assert_eq!(exp, got);
    }
}
