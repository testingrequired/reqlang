use std::{collections::HashMap, future::Future};

use parser::template;
use reqwest::{Client, Method, Response, Version};
use types::{
    http::{HttpRequest, HttpResponse, HttpStatusCode, HttpVersion},
    RequestParamsFromClient,
};

/// Implement a fetch that returns an [HttpResponse]. See [HttpRequestFetcher].
pub trait Fetch {
    fn fetch(
        &self,
    ) -> impl Future<Output = Result<HttpResponse, Box<dyn std::error::Error + Send>>> + Send;
}

/// Fetch using an [HttpRequest] that returns an [HttpResponse]
///
/// ## Usage
///
/// ```ignore
/// let fetcher: HttpRequestFetcher = http_request.into();
/// let response: HttpResponse = fetcher.fetch().await?;
/// ```
pub struct HttpRequestFetcher(HttpRequest);

impl HttpRequestFetcher {
    fn request_method(&self) -> Method {
        match self.0.verb.0.as_str() {
            "GET" => Method::GET,
            "POST" => Method::POST,
            _ => todo!(),
        }
    }

    fn request_url(&self) -> &str {
        &self.0.target
    }

    fn map_response_http_version(response: &Response) -> HttpVersion {
        match response.version() {
            Version::HTTP_11 => HttpVersion::one_point_one(),
            _ => todo!(),
        }
    }

    fn map_response_headers(response: &Response) -> HashMap<String, String> {
        let mut headers = HashMap::new();
        for (key, value) in response.headers() {
            headers.insert(
                key.to_string(),
                value.to_str().expect("Shoud work").to_string(),
            );
        }

        headers
    }

    fn map_response_status_code_and_text(response: &Response) -> (HttpStatusCode, String) {
        let response_status = response.status().to_string();
        let mut status_split = response_status.splitn(2, ' ');
        let status_code = status_split
            .next()
            .unwrap()
            .to_string()
            .try_into()
            .expect("Invalid status code");
        let status_text = status_split.next().unwrap().to_string();

        (status_code, status_text)
    }
}

impl Fetch for HttpRequestFetcher {
    async fn fetch(&self) -> std::result::Result<HttpResponse, Box<dyn std::error::Error + Send>> {
        let client = Client::new();

        let response = client
            .request(self.request_method(), self.request_url())
            .send()
            .await
            .expect("Request should have executed");

        let http_version = Self::map_response_http_version(&response);
        let headers = Self::map_response_headers(&response);
        let (status_code, status_text) = Self::map_response_status_code_and_text(&response);
        let body = response.text().await.ok();

        Ok(HttpResponse {
            http_version,
            status_code,
            status_text,
            headers,
            body,
        })
    }
}

impl From<HttpRequest> for HttpRequestFetcher {
    fn from(value: HttpRequest) -> Self {
        Self(value)
    }
}

/// Executes requests from an [`RequestParamsFromClient`].
///
/// ```ignore
/// let response: HttpResponse = Into::<RequestParamsFromClient>::into(params).fetch().await?;
/// ```
impl From<RequestParamsFromClient> for HttpRequestFetcher {
    fn from(params: RequestParamsFromClient) -> Self {
        let reqfile = template(
            &params.reqfile,
            params.env.as_deref(),
            &params.prompts,
            &params.secrets,
            &HashMap::new(),
        )
        .unwrap();

        Self(reqfile.request)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use pretty_assertions::assert_eq;
    use types::http::HttpStatusCode;

    use crate::Fetch;

    #[tokio::test]
    async fn test_real_http_request_fetch() {
        let http_request = HttpRequest {
            verb: types::http::HttpVerb("GET".to_owned()),
            target: "https://example.com".to_owned(),
            http_version: HttpVersion::one_point_one(),
            headers: vec![],
            body: None,
        };

        let fetcher: HttpRequestFetcher = http_request.into();
        let response = fetcher
            .fetch()
            .await
            .expect("Should be able to make real HTTP request");

        assert_eq!(HttpVersion::one_point_one(), response.http_version);
        assert_eq!(HttpStatusCode::new(200), response.status_code);

        assert_eq!(
            Some("text/html"),
            response.headers.get("content-type").map(|x| x.as_str())
        );

        assert_eq!("OK", response.status_text);

        assert!(response.body.is_some());

        let body = response.body.unwrap();

        assert!(body.contains("<h1>Example Domain</h1>"));
        assert!(body.contains("<p>This domain is for use in illustrative examples in documents. You may use this\n    domain in literature without prior coordination or asking for permission.</p>"));
    }

    #[tokio::test]
    async fn test_real_request_params_fetch() {
        let params: RequestParamsFromClient = RequestParamsFromClient {
            reqfile: r#"
```%request
GET http://example.com HTTP/1.1
```
            "#
            .to_string(),
            env: None,
            vars: HashMap::new(),
            prompts: HashMap::new(),
            secrets: HashMap::new(),
        };

        let fetcher: HttpRequestFetcher = params.into();
        let response = fetcher
            .fetch()
            .await
            .expect("Should be able to make real HTTP request");

        assert_eq!(HttpVersion::one_point_one(), response.http_version);
        assert_eq!(HttpStatusCode::new(200), response.status_code);

        assert_eq!(
            Some("text/html"),
            response.headers.get("content-type").map(|x| x.as_str())
        );

        assert_eq!("OK", response.status_text);

        assert!(response.body.is_some());

        let body = response.body.unwrap();

        assert!(body.contains("<h1>Example Domain</h1>"));
        assert!(body.contains("<p>This domain is for use in illustrative examples in documents. You may use this\n    domain in literature without prior coordination or asking for permission.</p>"));
    }
}
