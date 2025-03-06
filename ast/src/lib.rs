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
            ast.push((AstNode::RequestBlock(text.clone()), span.clone()));
        }

        for (text, span) in extract_codeblocks(&input, "%config").iter() {
            ast.push((AstNode::ConfigBlock(text.clone()), span.clone()));
        }

        for (text, span) in extract_codeblocks(&input, "%response").iter() {
            ast.push((AstNode::ResponseBlock(text.clone()), span.clone()));
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
                ast.push(AstNode::comment(comment, new_span.clone()));
                index = node_span.end;
            }
        }

        // Sort AST nodes by their positions
        ast.0.sort_by(|a, b| a.1.start.cmp(&b.1.start));

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
    pub fn config(&self) -> Option<&Spanned<String>> {
        self.iter().find_map(|(node, _)| match &node {
            AstNode::ConfigBlock(config) => Some(config),
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
    ConfigBlock(Spanned<String>),
    /// A code block delimited request
    RequestBlock(Spanned<String>),
    /// A code block delimited response
    ResponseBlock(Spanned<String>),
}

impl AstNode {
    /// Utility function to create a [Spanned] [AstNode::Comment] from the given text and [Span].
    pub fn comment(text: impl AsRef<str>, span: Span) -> Spanned<Self> {
        (Self::Comment(text.as_ref().to_string()), span.clone())
    }

    /// Utility function to create a [Spanned] [AstNode::ConfigBlock] from the given text and [Span].
    ///
    /// This automatically handles calculating the [Span] for the code block text
    pub fn config(text: impl AsRef<str>, span: Span) -> Spanned<Self> {
        let prefix = "```%config";
        let suffix = "```";

        let start = span.start + prefix.len() + 1;
        let end = span.end - suffix.len() + 1;

        (
            Self::ConfigBlock((text.as_ref().to_string(), start..end)),
            span.clone(),
        )
    }

    /// Utility function to create a [Spanned] [AstNode::RequestBlock] from the given text and [Span].
    ///
    /// This automatically handles calculating the [Span] for the code block text
    pub fn request(text: impl AsRef<str>, span: Span) -> Spanned<Self> {
        let prefix = "```%request";
        let suffix = "```";

        let start = span.start + prefix.len() + 1;
        let end = span.end - suffix.len();

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
        let end = span.end - suffix.len();

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
                (AstNode::Comment("\n".to_string()), 0..1),
                (
                    AstNode::RequestBlock(("REQUEST".to_string(), 13..20)),
                    1..24
                )
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
                (
                    AstNode::RequestBlock(("REQUEST".to_string(), 13..20)),
                    1..24
                ),
                AstNode::comment("\n", 24..25),
                (
                    AstNode::ResponseBlock(("RESPONSE".to_string(), 38..46)),
                    25..50
                ),
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
                (AstNode::ConfigBlock(("CONFIG".to_string(), 12..18)), 1..22),
                AstNode::comment("\n", 22..23),
                (
                    AstNode::RequestBlock(("REQUEST".to_string(), 35..42)),
                    23..46
                ),
                AstNode::comment("\n", 46..47),
                (
                    AstNode::ResponseBlock(("RESPONSE".to_string(), 60..68)),
                    47..72
                ),
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
                    AstNode::ConfigBlock((
                        concat!("vars = [\"foo\"]\n", "\n", "[envs]\n", "foo = \"bar\"",)
                            .to_string(),
                        15..49
                    )),
                    4..53
                ),
                AstNode::comment("\n\nB\n\n", 53..58),
                (
                    AstNode::RequestBlock((
                        "GET https://example.com HTTP/1.1".to_string(),
                        70..102
                    )),
                    58..106
                ),
                AstNode::comment("\n\nC\n\n", 106..111),
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
                        124..185
                    )),
                    111..189
                ),
            ]),
            ast_result
        );
    }
}
