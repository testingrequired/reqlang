use errors::ReqlangError;
use extract_codeblocks::extract_codeblocks;
use span::{Span, Spanned};

#[allow(clippy::upper_case_acronyms)]
#[derive(Debug, PartialEq, Default)]
pub struct AST(Vec<Spanned<Node>>);

impl AST {
    pub fn push(&mut self, node: Spanned<Node>) {
        self.0.push(node);
    }

    pub fn nodes(&self) -> impl Iterator<Item = &Spanned<Node>> {
        self.0.iter()
    }

    pub fn config(&self) -> Option<&Spanned<String>> {
        self.nodes().find_map(|(node, _)| match &node {
            Node::ConfigBlock(config) => Some(config),
            _ => None,
        })
    }

    pub fn request(&self) -> Option<&Spanned<String>> {
        self.nodes().find_map(|(node, _)| match &node {
            Node::RequestBlock(request) => Some(request),
            _ => None,
        })
    }

    pub fn response(&self) -> Option<&Spanned<String>> {
        self.nodes().find_map(|(node, _)| match &node {
            Node::ResponseBlock(response) => Some(response),
            _ => None,
        })
    }

    pub fn _comments(&self) -> Vec<Spanned<String>> {
        let mut comments = vec![];

        for node in self.nodes() {
            if let Node::Comment(comment) = &node.0 {
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
#[derive(Debug, PartialEq)]
pub enum Node {
    Comment(String),
    ConfigBlock(Spanned<String>),
    RequestBlock(Spanned<String>),
    ResponseBlock(Spanned<String>),
}

impl Node {
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

/// Parse request file string in to AST.
pub fn ast(input: impl AsRef<str>) -> Result<AST, Vec<Spanned<ReqlangError>>> {
    let mut ast = AST::default();

    let mut code_block_spans: Vec<Span> = vec![];

    for (text, span) in extract_codeblocks(&input, "%config").iter() {
        code_block_spans.push(span.clone());
        ast.push((Node::config(text, span.clone()), span.clone()));
    }

    for (text, span) in extract_codeblocks(&input, "%request").iter() {
        code_block_spans.push(span.clone());
        ast.push((Node::request(text, span.clone()), span.clone()));
    }

    for (text, span) in extract_codeblocks(&input, "%response").iter() {
        code_block_spans.push(span.clone());
        ast.push((Node::response(text, span.clone()), span.clone()));
    }

    // Sort code_block_spans be start & end of the spans
    code_block_spans.sort_by(|a, b| a.start.cmp(&b.start));

    let mut index = 0usize;

    for code_block_span in code_block_spans.iter() {
        let start = code_block_span.start;

        if index < start {
            let new_span = index..start;
            let comment = input.as_ref()[new_span.clone()].to_string();
            ast.push((Node::comment(comment), new_span.clone()));
            index = code_block_span.end;
        }
    }

    ast.0.sort_by(|a, b| a.1.start.cmp(&b.1.start));

    Ok(ast)
}

#[cfg(test)]
mod ast_tests {
    use super::*;
    use pretty_assertions::assert_eq;

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

        let ast_result = ast(source);
        assert_eq!(
            Ok(AST(vec![
                (Node::comment("\nA\n\n"), 0..4),
                (
                    Node::ConfigBlock((
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
                (Node::comment("\n\nB\n\n"), 53..58),
                (
                    Node::RequestBlock(("GET https://example.com HTTP/1.1".to_string(), 70..103)),
                    58..106
                ),
                (Node::comment("\n\nC\n\n"), 106..111),
                (
                    Node::ResponseBlock((
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
