#[derive(Clone, Debug, PartialEq)]
pub struct Request {
    pub verb: String,
    pub target: String,
    pub http_version: HttpVersion,
}

#[derive(Clone, Debug, PartialEq)]
pub enum HttpVersion {
    OneOne,
}
