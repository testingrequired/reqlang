use logos::Logos;
use std::fmt;

#[derive(Logos, Clone, Debug, PartialEq)]
#[logos(skip r"[\t\f]+")]
pub enum Token {
    #[regex(r"\s*")]
    SP,
    #[token("\n")]
    NL,
    #[regex(r"#!(.*)")]
    Shebang,
    #[regex(r#"HTTP/([0-9.]+)"#, lex_string)]
    HttpVersion(String),
    #[regex(
        r#"(GET|POST|PUT|PATCH|DELETE|HEAD|OPTIONS|CONNECT|TRACE)"#,
        lex_string
    )]
    Verb(String),
    #[regex(r#"(?:(?:http|https)://|/)[-_a-zA-Z0-9/:?%&.=]*"#, lex_string)]
    Url(String),
    #[regex(r#"[a-zA-Z][-a-zA-Z]+:\s+.*"#, lex_header_key_value)]
    HeaderKeyValue((String, String)),
    #[token("---")]
    TripleDash,
}

fn lex_string(lexer: &mut logos::Lexer<Token>) -> String {
    let slice = lexer.slice();
    slice.to_string()
}

fn lex_header_key_value(lexer: &mut logos::Lexer<Token>) -> (String, String) {
    let slice = lexer.slice();
    let parts: Vec<&str> = slice.split(":").map(|x| x.trim()).collect();

    let key = parts.get(0).expect("...").to_owned().to_string();
    let value = parts[1..].join(":");

    (key, value)
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
