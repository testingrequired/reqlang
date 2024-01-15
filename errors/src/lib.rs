use span::Spanned;

#[derive(Clone, Debug, PartialEq)]
pub enum LexicalError {
    InvalidToken(String),
}

impl Default for LexicalError {
    fn default() -> Self {
        Self::InvalidToken("".to_string())
    }
}

pub type ErrorS = Spanned<LexicalError>;
