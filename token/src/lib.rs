use logos::Logos;
use std::fmt;

#[derive(Logos, Clone, Debug, PartialEq)]
#[logos(skip r"[\t\f]+")]
pub enum Token {
    #[token(" ")]
    SP,
    #[token("\n")]
    NL,
    #[token("HTTP/1.1")]
    HttpVersion,
    #[regex(r#"[A-Z]+"#, lex_string, priority = 2)]
    Verb(String),
    #[regex(r#"[-_a-zA-Z0-9/:?%&.]+"#, lex_string)]
    Url(String),
}

fn lex_string(lexer: &mut logos::Lexer<Token>) -> String {
    let slice = lexer.slice();
    slice.to_string()
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
