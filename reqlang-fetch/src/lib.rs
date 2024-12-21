use std::{collections::HashMap, future::Future};

use parser::template;
use reqwest::{Client, Method, Version};
use types::{
    http::{HttpRequest, HttpResponse, HttpVersion},
    RequestParamsFromClient,
};

/// Execute HTTP Request
///
/// Write an implementation that returns an [`HttpResponse`]
pub trait Fetch {
    fn fetch(
        &self,
    ) -> impl Future<Output = Result<HttpResponse, Box<dyn std::error::Error + Send>>> + Send;
}

/// Execute HTTP Request using [`HttpRequest`].
///
/// ## Usage
///
/// ```ignore
/// let fetcher: HttpRequestFetcher = http_request.into();
/// let response: HttpResponse = fetcher.fetch().await?;
/// ```
pub struct HttpRequestFetcher(pub HttpRequest);

impl Fetch for HttpRequestFetcher {
    async fn fetch(&self) -> std::result::Result<HttpResponse, Box<dyn std::error::Error + Send>> {
        let http_request = &self.0;

        let url = &http_request.target;

        let request_method: Method = match http_request.verb.0.as_str() {
            "GET" => Method::GET,
            "POST" => Method::POST,
            _ => todo!(),
        };

        let client = Client::new();
        let response = client
            .request(request_method, url)
            .send()
            .await
            .expect("Request should have executed");

        let response_http_version = match response.version() {
            Version::HTTP_11 => HttpVersion::one_point_one(),
            _ => todo!(),
        };

        let response_headers = {
            let mut headers = HashMap::new();
            for (key, value) in response.headers() {
                headers.insert(
                    key.to_string(),
                    value.to_str().expect("Shoud work").to_string(),
                );
            }

            headers
        };

        let (status_code, status_text) = {
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
        };

        let response_body = response.text().await.ok();

        let response = HttpResponse {
            http_version: response_http_version,
            status_code,
            status_text,
            headers: response_headers,
            body: response_body,
        };

        Ok(response)
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
            &params.env,
            &params.prompts,
            &params.secrets,
            HashMap::new(),
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
            Some("text/html; charset=UTF-8"),
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
            ---
GET http://example.com HTTP/1.1

            "#
            .to_string(),
            env: "default".to_string(),
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
            Some("text/html; charset=UTF-8"),
            response.headers.get("content-type").map(|x| x.as_str())
        );

        assert_eq!("OK", response.status_text);

        assert!(response.body.is_some());

        let body = response.body.unwrap();

        assert!(body.contains("<h1>Example Domain</h1>"));
        assert!(body.contains("<p>This domain is for use in illustrative examples in documents. You may use this\n    domain in literature without prior coordination or asking for permission.</p>"));
    }
}
