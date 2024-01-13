#[derive(Clone, Debug, PartialEq)]
pub struct Request {
    pub verb: Verb,
    pub target: String,
    pub http_version: HttpVersion,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Verb {
    GET,
    POST,
    PUT,
    DELETE,
    HEAD,
    OPTIONS,
    PATCH,
    CONNECT,
    TRACE,
}

#[derive(Clone, Debug, PartialEq)]
pub enum HttpVersion {
    OneOne,
}
