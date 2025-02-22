use errors::ReqlangError;
use extract_codeblocks::extract_codeblocks;
use span::{Span, Spanned};

#[derive(Debug, PartialEq)]
pub struct Ast(Vec<Spanned<AstNode>>);

impl Ast {
    /// Parse a string in to an abstract syntax tree
    pub fn parse(input: impl AsRef<str>) -> Result<Self, Vec<Spanned<ReqlangError>>> {
        let mut ast = Self(vec![]);

        for (text, span) in extract_codeblocks(&input, "%request").iter() {
            ast.push((AstNode::request(text, span.clone()), span.clone()));
        }

        if ast.0.is_empty() {
            return Err(vec![(
                ReqlangError::ParseError(errors::ParseError::MissingRequest),
                0..0,
            )]);
        }

        for (text, span) in extract_codeblocks(&input, "%config").iter() {
            ast.push((AstNode::config(text, span.clone()), span.clone()));
        }

        for (text, span) in extract_codeblocks(&input, "%response").iter() {
            ast.push((AstNode::response(text, span.clone()), span.clone()));
        }

        // Sort AST nodes by their positions
        ast.0.sort_by(|a, b| a.1.start.cmp(&b.1.start));

        let mut index = 0usize;

        // Find comment nodes
        for (_, node_span) in ast.0.clone().iter() {
            let start = node_span.start;

            if index < start {
                let new_span = index..start;
                let comment = input.as_ref()[new_span.clone()].to_string();
                ast.push((AstNode::comment(comment), new_span.clone()));
                index = node_span.end;
            }
        }

        // Sort AST nodes by their positions
        ast.0.sort_by(|a, b| a.1.start.cmp(&b.1.start));

        Ok(ast)
    }

    fn push(&mut self, node: Spanned<AstNode>) {
        self.0.push(node);
    }

    pub fn nodes(&self) -> impl Iterator<Item = &Spanned<AstNode>> {
        self.0.iter()
    }

    pub fn config(&self) -> Option<&Spanned<String>> {
        self.nodes().find_map(|(node, _)| match &node {
            AstNode::ConfigBlock(config) => Some(config),
            _ => None,
        })
    }

    pub fn request(&self) -> Option<&Spanned<String>> {
        self.nodes().find_map(|(node, _)| match &node {
            AstNode::RequestBlock(request) => Some(request),
            _ => None,
        })
    }

    pub fn response(&self) -> Option<&Spanned<String>> {
        self.nodes().find_map(|(node, _)| match &node {
            AstNode::ResponseBlock(response) => Some(response),
            _ => None,
        })
    }

    pub fn _comments(&self) -> Vec<Spanned<String>> {
        let mut comments = vec![];

        for node in self.nodes() {
            if let AstNode::Comment(comment) = &node.0 {
                comments.push((comment.clone(), node.1.clone()));
            }
        }

        // self.nodes().find_map(|(node, _)| match &node {
        //     Node::Comment(comment) => Some(response),
        //     _ => None,
        // })

        comments
    }
}

#[allow(clippy::enum_variant_names)]
#[derive(Debug, PartialEq, Clone)]
pub enum AstNode {
    Comment(String),
    ConfigBlock(Spanned<String>),
    RequestBlock(Spanned<String>),
    ResponseBlock(Spanned<String>),
}

impl AstNode {
    pub fn comment(text: impl AsRef<str>) -> Self {
        Self::Comment(text.as_ref().to_string())
    }

    pub fn config(text: impl AsRef<str>, span: Span) -> Self {
        let prefix = "```%config";
        let suffix = "```";

        let start = span.start + prefix.len() + 1;
        let end = span.end - suffix.len();

        Self::ConfigBlock((text.as_ref().to_string(), start..end))
    }

    pub fn request(text: impl AsRef<str>, span: Span) -> Self {
        let prefix = "```%request";
        let suffix = "```";

        let start = span.start + prefix.len() + 1;
        let end = span.end - suffix.len();

        Self::RequestBlock((text.as_ref().to_string(), start..end))
    }

    pub fn response(text: impl AsRef<str>, span: Span) -> Self {
        let prefix = "```%response";
        let suffix = "```";

        let start = span.start + prefix.len() + 1;
        let end = span.end - suffix.len();

        Self::ResponseBlock((text.as_ref().to_string(), start..end))
    }
}

#[derive(Debug, PartialEq)]
pub struct Comment(String);

#[cfg(test)]
mod ast_tests {
    use crate::ast;

    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_empty_string() {
        let output = ast::Ast::parse("");

        assert_eq!(
            Err(vec![(
                ReqlangError::ParseError(errors::ParseError::MissingRequest),
                0..0
            )]),
            output
        );
    }

    #[test]
    fn test_whitespace_string() {
        let output = ast::Ast::parse(" \n ");

        assert_eq!(
            Err(vec![(
                ReqlangError::ParseError(errors::ParseError::MissingRequest),
                0..0
            )]),
            output
        );
    }

    #[test]
    fn test_request_without_response_or_config() {
        let input = textwrap::dedent(
            "
        ```%request
        REQUEST
        ```
        ",
        );

        let ast_result = ast::Ast::parse(input);
        assert_eq!(
            Ok(Ast(vec![
                (AstNode::comment("\n"), 0..1),
                (AstNode::request("REQUEST", 1..24), 1..24),
            ])),
            ast_result
        );
    }

    #[test]
    fn test_request_with_response_and_empty_config() {
        let input = textwrap::dedent(
            "
        ```%request
        REQUEST
        ```
        ```%response
        RESPONSE
        ```
        ",
        );

        let ast_result = ast::Ast::parse(input);
        assert_eq!(
            Ok(Ast(vec![
                (AstNode::comment("\n"), 0..1),
                (AstNode::request("REQUEST", 1..24), 1..24),
                (AstNode::comment("\n"), 24..25),
                (AstNode::response("RESPONSE", 25..50), 25..50),
            ])),
            ast_result
        );
    }

    #[test]
    fn test_request_with_response_and_config() {
        let input = textwrap::dedent(
            "
            ```%config
            CONFIG
            ```
            ```%request
            REQUEST
            ```
            ```%response
            RESPONSE
            ```
            ",
        );

        let ast_result = ast::Ast::parse(input);
        assert_eq!(
            Ok(Ast(vec![
                (AstNode::comment("\n"), 0..1),
                (AstNode::config("CONFIG", 1..22), 1..22),
                (AstNode::comment("\n"), 22..23),
                (AstNode::request("REQUEST", 23..46), 23..46),
                (AstNode::comment("\n"), 46..47),
                (AstNode::response("RESPONSE", 47..72), 47..72),
            ])),
            ast_result
        );
    }

    #[test]
    fn parse_request_file() {
        let source = textwrap::dedent(
            r#"
            A

            ```%config
            vars = ["foo"]

            [envs]
            foo = "bar"
            ```

            B

            ```%request
            GET https://example.com HTTP/1.1
            ```

            C

            ```%response
            HTTP/1.1 200 OK
            content-type: application/html

            <html></html>
            ```

            D
            "#,
        );

        let ast_result = ast::Ast::parse(source);
        assert_eq!(
            Ok(Ast(vec![
                (AstNode::comment("\nA\n\n"), 0..4),
                (
                    AstNode::ConfigBlock((
                        textwrap::dedent(
                            r#"
                            vars = ["foo"]

                            [envs]
                            foo = "bar"
                            "#
                        )
                        .trim()
                        .to_string(),
                        15..50
                    )),
                    4..53
                ),
                (AstNode::comment("\n\nB\n\n"), 53..58),
                (
                    AstNode::RequestBlock((
                        "GET https://example.com HTTP/1.1".to_string(),
                        70..103
                    )),
                    58..106
                ),
                (AstNode::comment("\n\nC\n\n"), 106..111),
                (
                    AstNode::ResponseBlock((
                        textwrap::dedent(
                            r#"
                            HTTP/1.1 200 OK
                            content-type: application/html

                            <html></html>
                            "#
                        )
                        .trim()
                        .to_string(),
                        124..186
                    )),
                    111..189
                ),
            ])),
            ast_result
        );
    }
}
