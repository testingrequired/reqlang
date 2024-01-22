use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
pub enum ReqlangError {
    #[error("ParseError: {0}")]
    ParseError(ParseError),
}

#[derive(Debug, Error, PartialEq)]
pub enum ParseError {
    #[error("Request file is an empty file")]
    EmptyFileError,
    #[error("Request file has no document dividers")]
    NoDividersError,
    #[error("Request file has too many document dividers")]
    TooManyDividersError,
    #[error("Request is invalid: {message}")]
    InvalidRequestError { message: String },
    #[error("Config is invalid: {message}")]
    InvalidConfigError { message: String },
    #[error("Undefined template reference: {name}")]
    UndefinedReferenceError { name: String },
}

macro_rules! impl_from_error {
    ($($error:tt),+) => {$(
        impl From<$error> for ReqlangError {
            fn from(e: $error) -> Self {
                ReqlangError::$error(e)
            }
        }
    )+};
}

impl_from_error!(ParseError);
