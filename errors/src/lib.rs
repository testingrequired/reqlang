use serde::{Deserialize, Serialize};
use thiserror::Error;
use types::ReferenceType;

#[derive(Debug, Error, PartialEq, Serialize, Deserialize)]
pub enum ReqlangError {
    #[error("ParseError: {0}")]
    ParseError(ParseError),
    #[error("ResolverError: {0}")]
    ResolverError(ResolverError),
}

#[derive(Debug, Error, PartialEq, Serialize, Deserialize)]
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
    UnusedValueError(ReferenceType),
}

#[derive(Debug, Error, PartialEq, Serialize, Deserialize)]
pub enum ResolverError {
    #[error("Invalid env: {0}")]
    InvalidEnvError(String),
    #[error("Prompt required but not passed: {0}")]
    PromptValueNotPassed(String),
    #[error("Secret required but not passed: {0}")]
    SecretValueNotPassed(String),
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

impl_from_error!(ParseError, ResolverError);
