use reqlang::{ast::AstNode, Ast, NO_SPAN};
use tower_lsp::lsp_types::{SemanticToken, SemanticTokenType};

pub const LEGEND_TYPE: &[SemanticTokenType] = &[SemanticTokenType::COMMENT];

pub fn generate_semantic_tokens(ast: &Ast, source: &str) -> Vec<SemanticToken> {
    let mut tokens: Vec<SemanticToken> = vec![];

    let mut prev_span = NO_SPAN;

    for (node, span) in ast.iter() {
        let positon = str_idxpos::index_to_position(source, span.start);
        let prev_position = str_idxpos::index_to_position(source, prev_span.start);
        let delta_line: u32 = (positon.0 as u32 - prev_position.0 as u32) + 1;

        if let AstNode::Comment(_) = node {
            let comment_text = &source[span.start..span.end];

            let lines = comment_text.lines();

            for comment_line in lines {
                tokens.push(SemanticToken {
                    delta_line,
                    delta_start: 0,
                    length: comment_line.len() as u32,
                    token_type: LEGEND_TYPE
                        .iter()
                        .position(|token| token == &SemanticTokenType::COMMENT)
                        .unwrap()
                        .try_into()
                        .unwrap(),
                    token_modifiers_bitset: 0,
                });
            }
        }

        prev_span = span.clone();
    }

    tokens
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn single_comment_node() {
        let source = "\nHello, World!\nFoo\n\nBar\n";

        let ast = Ast::from(vec![(AstNode::Comment(source.to_string()), 0..24)]);

        assert_eq!(
            vec![
                SemanticToken {
                    delta_line: 1,
                    delta_start: 0,
                    length: 0,
                    token_type: 0,
                    token_modifiers_bitset: 0
                },
                SemanticToken {
                    delta_line: 1,
                    delta_start: 0,
                    length: 13,
                    token_type: 0,
                    token_modifiers_bitset: 0
                },
                SemanticToken {
                    delta_line: 1,
                    delta_start: 0,
                    length: 3,
                    token_type: 0,
                    token_modifiers_bitset: 0
                },
                SemanticToken {
                    delta_line: 1,
                    delta_start: 0,
                    length: 0,
                    token_type: 0,
                    token_modifiers_bitset: 0
                },
                SemanticToken {
                    delta_line: 1,
                    delta_start: 0,
                    length: 3,
                    token_type: 0,
                    token_modifiers_bitset: 0
                },
            ],
            generate_semantic_tokens(&ast, source)
        );
    }

    #[test]
    fn different_nodes() {
        let source = "Foo\n```%request\nREQUEST\n```\nBar";

        let ast = Ast::new(source);

        assert_eq!(
            vec![
                SemanticToken {
                    delta_line: 1,
                    delta_start: 0,
                    length: 3,
                    token_type: 0,
                    token_modifiers_bitset: 0
                },
                SemanticToken {
                    delta_line: 4,
                    delta_start: 0,
                    length: 3,
                    token_type: 0,
                    token_modifiers_bitset: 0
                },
            ],
            generate_semantic_tokens(&ast, source)
        );
    }
}
