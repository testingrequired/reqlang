use span::Spanned;

#[derive(Clone, Debug, PartialEq)]
pub enum LexicalError {
    InvalidToken,
}

pub type ErrorS = Spanned<LexicalError>;
