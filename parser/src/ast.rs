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
}

#[allow(clippy::enum_variant_names)]
#[derive(Debug, PartialEq)]
pub enum Node {
    ConfigBlock(Spanned<String>),
    RequestBlock(Spanned<String>),
    ResponseBlock(Spanned<String>),
}

impl Node {
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

    for (text, span) in extract_codeblocks(&input, "%config").iter() {
        ast.push((Node::config(text, span.clone()), span.clone()));
    }

    for (text, span) in extract_codeblocks(&input, "%request").iter() {
        ast.push((Node::request(text, span.clone()), span.clone()));
    }

    for (text, span) in extract_codeblocks(&input, "%response").iter() {
        ast.push((Node::response(text, span.clone()), span.clone()));
    }

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
            ```%config
            vars = ["foo"]

            [envs]
            foo = "bar"
            ```

            ```%request
            GET https://example.com HTTP/1.1
            ```

            ```%response
            HTTP/1.1 200 OK
            content-type: application/html

            <html></html>
            ```
            "#,
        );

        let ast_result = ast(source);
        assert_eq!(
            Ok(AST(vec![
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
                        11..47
                    )),
                    1..50
                ),
                (
                    Node::RequestBlock(("GET https://example.com HTTP/1.1".to_string(), 63..97)),
                    52..100
                ),
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
                        114..177
                    )),
                    102..180
                ),
            ])),
            ast_result
        );
    }
}
