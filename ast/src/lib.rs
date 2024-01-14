#[derive(Clone, Debug, PartialEq, Default)]
pub struct Document {
    pub request: Request,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Request {
    pub verb: String,
    pub target: String,
    pub http_version: HttpVersion,
}

#[derive(Clone, Debug, PartialEq)]
pub enum HttpVersion {
    OneOne,
}

impl Default for HttpVersion {
    fn default() -> Self {
        Self::OneOne
    }
}
