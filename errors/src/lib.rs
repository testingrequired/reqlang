use thiserror::Error;
use types::ReferenceType;

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
    #[error("Undefined template reference: {0}")]
    UndefinedReferenceError(ReferenceType),
    #[error("Value was declared but not used. Try adding the template reference {0} to the request or response.")]
    UnusedValue(ReferenceType),
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
