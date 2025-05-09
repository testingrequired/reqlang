use serde::{Deserialize, Serialize};

use crate::{extract_codeblocks::extract_codeblocks, span::Spanned};

/// Abstract syntax tree for a request file
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Ast(Vec<Spanned<AstNode>>);

impl Ast {
    /// Create an [AST](Ast) from a collection of [nodes](AstNode)
    pub fn new(nodes: Vec<Spanned<AstNode>>) -> Self {
        Self(nodes)
    }

    /// Create an [AST](Ast) by parsing a string in to a collection of [nodes](AstNode)
    pub fn from(input: impl AsRef<str>) -> Self {
        let mut nodes: Vec<Spanned<AstNode>> = vec![];

        for (text, span) in extract_codeblocks(&input, "%request").iter() {
            nodes.push((AstNode::RequestBlock(text.clone()), span.clone()));
        }

        for (text, span) in extract_codeblocks(&input, "%config").iter() {
            nodes.push((AstNode::ConfigBlock(text.clone()), span.clone()));
        }

        for (text, span) in extract_codeblocks(&input, "%response").iter() {
            nodes.push((AstNode::ResponseBlock(text.clone()), span.clone()));
        }

        // Sort AST nodes by their positions
        nodes.sort_by(|a, b| a.1.start.cmp(&b.1.start));

        let mut index = 0usize;

        // Find comment nodes
        for (_, node_span) in nodes.clone().iter() {
            let start = node_span.start;

            if index < start {
                let new_span = index..start;
                let comment = input.as_ref()[new_span.clone()].to_string();
                nodes.push((AstNode::Comment(comment), new_span.clone()));
                index = node_span.end;
            }
        }

        // Sort AST nodes by their positions
        nodes.sort_by(|a, b| a.1.start.cmp(&b.1.start));

        Self::new(nodes)
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

#[cfg(test)]
mod ast_tests {

    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_empty_string() {
        let output = Ast::from("");

        assert_eq!(Ast(vec![]), output);
    }

    #[test]
    fn test_whitespace_string() {
        let output = Ast::from(" \n ");

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

        let ast_result = Ast::from(input);
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

        let ast_result = Ast::from(input);
        assert_eq!(
            Ast(vec![
                (AstNode::Comment("\n".to_string()), 0..1),
                (
                    AstNode::RequestBlock(("REQUEST".to_string(), 13..20)),
                    1..24
                ),
                (AstNode::Comment("\n".to_string()), 24..25),
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

        let ast_result = Ast::from(input);
        assert_eq!(
            Ast(vec![
                (AstNode::Comment("\n".to_string()), 0..1),
                (AstNode::ConfigBlock(("CONFIG".to_string(), 12..18)), 1..22),
                (AstNode::Comment("\n".to_string()), 22..23),
                (
                    AstNode::RequestBlock(("REQUEST".to_string(), 35..42)),
                    23..46
                ),
                (AstNode::Comment("\n".to_string()), 46..47),
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
[[vars]]
name = "foo"

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

        let ast_result = Ast::from(source);
        assert_eq!(
            Ast(vec![
                (AstNode::Comment("\nA\n\n".to_string()), 0..4),
                (
                    AstNode::ConfigBlock((
                        concat!(
                            "[[vars]]\n",
                            "name = \"foo\"\n",
                            "\n",
                            "[envs]\n",
                            "foo = \"bar\"",
                        )
                        .to_string(),
                        15..56
                    )),
                    4..60
                ),
                (AstNode::Comment("\n\nB\n\n".to_string()), 60..65),
                (
                    AstNode::RequestBlock((
                        "GET https://example.com HTTP/1.1".to_string(),
                        77..109
                    )),
                    65..113
                ),
                (AstNode::Comment("\n\nC\n\n".to_string()), 113..118),
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
                        131..192
                    )),
                    118..196
                ),
            ]),
            ast_result
        );
    }
}
