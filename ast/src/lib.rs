use extract_codeblocks::extract_codeblocks;
use serde::{Deserialize, Serialize};
use span::{Span, Spanned};

/// Abstract syntax tree for a request file
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Ast(Vec<Spanned<AstNode>>);

impl Ast {
    pub fn from(nodes: Vec<Spanned<AstNode>>) -> Self {
        Self(nodes)
    }

    /// Parse a string in to an abstract syntax tree
    pub fn new(input: impl AsRef<str>) -> Self {
        let mut ast = Self(vec![]);

        for (text, span) in extract_codeblocks(&input, "%request").iter() {
            let block_span = span.start..span.end;
            ast.push(AstNode::request(text, block_span));
        }

        for (text, span) in extract_codeblocks(&input, "%config").iter() {
            let block_span = span.start..span.end;
            ast.push((AstNode::config(text), block_span));
        }

        for (text, span) in extract_codeblocks(&input, "%response").iter() {
            let block_span = span.start..span.end;
            ast.push(AstNode::response(text, block_span));
        }

        // Sort AST nodes by their positions
        ast.0.sort_by(|a, b| a.1.start.cmp(&b.1.start));

        let mut index = 0usize;

        // Find comment nodes
        for (_, node_span) in ast.0.clone().iter() {
            if index < node_span.start {
                let new_span = index..node_span.start;
                let comment = input.as_ref()[new_span.clone()].to_string();
                ast.push(AstNode::comment(comment, new_span.clone()));
                index = node_span.end + 1;
            }
        }

        // Sort AST nodes by their positions
        ast.0.sort_by(|a, b| a.1.start.cmp(&b.1.start));

        if index < input.as_ref().len() {
            let last_comment = input.as_ref()[index..].to_string();
            if !last_comment.trim().is_empty() {
                ast.push(AstNode::comment(last_comment, index..input.as_ref().len()));
            }
        }

        ast
    }

    fn push(&mut self, node: Spanned<AstNode>) {
        self.0.push(node);
    }

    /// Iterate over the [nodes](AstNode)
    pub fn iter(&self) -> impl Iterator<Item = &Spanned<AstNode>> {
        self.0.iter()
    }

    /// Get the [AstNode::ConfigBlock], if present
    pub fn config(&self) -> Option<&Spanned<AstNode>> {
        self.iter()
            .find(|(node, _)| matches!(node, AstNode::ConfigBlock(_)))
    }

    /// Get the [AstNode::ConfigBlock], if present
    pub fn config_text(&self) -> Option<Spanned<String>> {
        self.config().iter().find_map(|(node, span)| match node {
            AstNode::ConfigBlock(text) => {
                let relative_span = node.relative_span();
                let absolute_span =
                    span.start + relative_span.start..span.start + relative_span.end;

                Some((text.clone(), absolute_span))
            }
            _ => None,
        })
    }

    /// Get the [AstNode::RequestBlock]
    pub fn request(&self) -> Option<&Spanned<String>> {
        self.iter().find_map(|(node, _)| match &node {
            AstNode::RequestBlock(request) => Some(request),
            _ => None,
        })
    }

    /// Get the [AstNode::ResponseBlock], if present
    pub fn response(&self) -> Option<&Spanned<String>> {
        self.iter().find_map(|(node, _)| match &node {
            AstNode::ResponseBlock(response) => Some(response),
            _ => None,
        })
    }

    /// Get all [AstNode::Comment]
    pub fn comments(&self) -> Vec<Spanned<String>> {
        let mut comments = vec![];

        for node in self.iter() {
            if let AstNode::Comment(comment) = &node.0 {
                comments.push((comment.clone(), node.1.clone()));
            }
        }

        comments
    }
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub enum AstNode {
    /// Any text that isn't a [AstNode::RequestBlock], [AstNode::ResponseBlock], or [AstNode::ConfigBlock].
    Comment(String),
    /// A code block delimited configuration
    ConfigBlock(String),
    /// A code block delimited request
    RequestBlock(Spanned<String>),
    /// A code block delimited response
    ResponseBlock(Spanned<String>),
}

impl AstNode {
    pub fn relative_span(&self) -> Span {
        match &self {
            AstNode::Comment(text) => 0..text.len(),
            AstNode::ConfigBlock(text) => {
                let prefix = "```%config\n";
                let suffix = "```\n";

                let start = prefix.len();
                let end = start + text.len() + suffix.len();

                start..end
            }
            AstNode::RequestBlock(_) => todo!(),
            AstNode::ResponseBlock(_) => todo!(),
        }
    }

    pub fn absolute_span(&self, span: &Span) -> Span {
        let relative_span = self.relative_span();
        let absolute_span = span.start + relative_span.start..span.start + relative_span.end;

        absolute_span
    }

    /// Utility function to create a [Spanned] [AstNode::Comment] from the given text and [Span].
    pub fn comment(text: impl AsRef<str>, span: Span) -> Spanned<Self> {
        (Self::Comment(text.as_ref().to_string()), span.clone())
    }

    /// Utility function to create a [AstNode::ConfigBlock] from the given text.
    pub fn config(text: impl AsRef<str>) -> Self {
        Self::ConfigBlock(text.as_ref().to_string())
    }

    /// Utility function to create a [Spanned] [AstNode::RequestBlock] from the given text and [Span].
    ///
    /// This automatically handles calculating the [Span] for the code block text
    pub fn request(text: impl AsRef<str>, span: Span) -> Spanned<Self> {
        let prefix = "```%request";
        let suffix = "```";

        let start = span.start + prefix.len() + 1;
        let end = span.end - suffix.len() + 1;

        (
            Self::RequestBlock((text.as_ref().to_string(), start..end)),
            span.clone(),
        )
    }

    /// Utility function to create a [Spanned] [AstNode::ResponseBlock] from the given text and [Span].
    ///
    /// This automatically handles calculating the [Span] for the code block text
    pub fn response(text: impl AsRef<str>, span: Span) -> Spanned<Self> {
        let prefix = "```%response";
        let suffix = "```";

        let start = span.start + prefix.len() + 1;
        let end = span.end - suffix.len() + 1;

        (
            Self::ResponseBlock((text.as_ref().to_string(), start..end)),
            span.clone(),
        )
    }
}

#[cfg(test)]
mod ast_tests {

    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_empty_string() {
        let output = Ast::new("");

        assert_eq!(Ast(vec![]), output);
    }

    #[test]
    fn test_whitespace_string() {
        let output = Ast::new(" \n ");

        assert_eq!(Ast(vec![]), output);
    }

    #[test]
    fn test_a() {
        let input = "Foo\n```%request\nREQUEST\n```\nBar".to_string();

        let ast_result = Ast::new(input);
        assert_eq!(
            Ast(vec![
                AstNode::comment("Foo\n", 0..4),
                AstNode::request("REQUEST", 4..28),
                AstNode::comment("Bar", 28..31),
            ]),
            ast_result
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

        let ast_result = Ast::new(input);
        assert_eq!(
            Ast(vec![
                AstNode::comment("\n", 0..1),
                AstNode::request("REQUEST", 1..25)
            ]),
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

        let ast_result = Ast::new(input);
        assert_eq!(
            Ast(vec![
                AstNode::comment("\n", 0..1),
                AstNode::request("REQUEST", 1..25),
                AstNode::comment("\n", 25..26),
                AstNode::response("RESPONSE", 26..52),
            ]),
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

        let ast_result = Ast::new(input);
        assert_eq!(
            Ast(vec![
                AstNode::comment("\n", 0..1),
                AstNode::config("CONFIG", 1..23),
                AstNode::comment("\n", 23..24),
                AstNode::request("REQUEST", 24..48),
                AstNode::comment("\n", 48..49),
                AstNode::response("RESPONSE", 49..75),
            ]),
            ast_result
        );
    }

    #[test]
    fn test_request_with_response_and_config_no_newlines_between() {
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
        )
        .trim()
        .to_string();

        let ast_result = Ast::new(input);
        assert_eq!(
            Ast(vec![
                AstNode::config("CONFIG", 0..22),
                AstNode::request("REQUEST", 23..47),
                AstNode::response("RESPONSE", 47..73)
            ]),
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

        let ast_result = Ast::new(source);
        assert_eq!(
            Ast(vec![
                AstNode::comment("\nA\n\n", 0..4),
                (
                    AstNode::ConfigBlock(
                        textwrap::dedent(
                            r#"
                            vars = ["foo"]

                            [envs]
                            foo = "bar"
                            "#
                        )
                        .trim()
                        .to_string()
                    ),
                    4..54
                ),
                AstNode::comment("\nB\n\n", 54..58),
                (
                    AstNode::RequestBlock((
                        "GET https://example.com HTTP/1.1".to_string(),
                        70..104
                    )),
                    58..107
                ),
                AstNode::comment("\nC\n\n", 107..111),
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
                        124..187
                    )),
                    111..190
                ),
                AstNode::comment("\nD\n", 190..193),
            ]),
            ast_result
        );
    }
}

#[cfg(test)]
mod ast_node_tests {
    use super::*;

    use pretty_assertions::assert_eq;

    #[test]
    fn config_block_no_span() {
        assert_eq!(
            AstNode::ConfigBlock("CONFIG TEXT".to_string()),
            AstNode::config_relative_span("CONFIG TEXT")
        );
    }
}
