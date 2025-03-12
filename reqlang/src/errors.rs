use crate::types::ReferenceType;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Common error for parsing and templating request files
#[derive(Debug, Clone, Error, PartialEq, Serialize, Deserialize)]
pub enum ReqlangError {
    #[error("ParseError: {0}")]
    ParseError(ParseError),
    #[error("ResolverError: {0}")]
    ResolverError(ResolverError),
}

#[derive(Debug, Clone, Error, PartialEq, Serialize, Deserialize)]
pub enum ParseError {
    #[error("Request file requires a request be defined")]
    MissingRequest,
    #[error("Request is invalid: {message}")]
    InvalidRequestError { message: String },
    #[error("Config is invalid: {message}")]
    InvalidConfigError { message: String },
    #[error("Undefined template reference: {0}")]
    UndefinedReferenceError(ReferenceType),
    #[error(
        "Value was declared but not used. Try adding the template reference {0} to the request or response."
    )]
    UnusedValueError(ReferenceType),
    #[error(
        "This request header is calculated at request time and can not be specified by user: {0}"
    )]
    ForbiddenRequestHeaderNameError(String),
    #[error("Variable '{0}' is undefined in the environment '{1}'")]
    VariableUndefinedInEnvironment(String, String),
    #[error("Variable '{0}' is not defined in any environment or no environments are defined")]
    VariableNotDefinedInAnyEnvironment(String),
}

#[derive(Debug, Clone, Error, PartialEq, Serialize, Deserialize)]
pub enum ResolverError {
    #[error("'{0}' is not a defined environment in the request file")]
    InvalidEnvError(String),
    #[error(
        "Trying to resolve the environment '{0}' but no environments are defined in the request file"
    )]
    NoEnvironmentsDefined(String),
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
