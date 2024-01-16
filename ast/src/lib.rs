#[derive(Clone, Debug, PartialEq, Default)]
pub struct Document {
    pub request: Request,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Request {
    pub verb: String,
    pub target: String,
    pub http_version: String,
    pub headers: Vec<(String, String)>,
    pub body: Option<String>,
}
