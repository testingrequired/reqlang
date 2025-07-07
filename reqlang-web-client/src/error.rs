use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use tracing::error;

#[derive(Debug)]
pub enum Error {
    Io(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let Error::Io(printable) = self;

        write!(f, "{printable}")
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::Io(err.to_string())
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let Error::Io(body) = self;
        error!(body);
        (StatusCode::INTERNAL_SERVER_ERROR, body).into_response()
    }
}
