use lalrpop_util::lalrpop_mod;

lalrpop_mod!(pub reqlang);

#[cfg(test)]
mod tests {
    use super::*;

    use ast::{Document, Request};
    use lexer::Lexer;

    #[test]
    fn test() {
        let input = concat!(
            "#!/usr/bin/env reqlang\n",
            "---\n",
            "GET http://example.com HTTP/1.1\n",
            "---\n"
        );

        let lexer = Lexer::new(input);

        let parser = reqlang::DocumentParser::new();

        let mut parser_errors = Vec::new();

        let document = match parser.parse(lexer) {
            Ok(program) => program,
            Err(err) => {
                parser_errors.push(err);
                Document::default()
            }
        };

        let expected: Vec<
            lalrpop_util::ParseError<
                usize,
                token::Token,
                (errors::LexicalError, std::ops::Range<usize>),
            >,
        > = vec![];

        assert_eq!(expected, parser_errors);

        assert_eq!(
            document,
            Document {
                request: Request {
                    verb: "GET".to_string(),
                    target: "http://example.com".to_string(),
                    http_version: "HTTP/1.1".to_string(),
                    headers: vec![]
                }
            }
        )
    }
}
