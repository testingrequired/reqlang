use std::{collections::HashMap, future::Future};

use crate::types::{
    RequestParamsFromClient,
    http::{HttpRequest, HttpResponse, HttpStatusCode, HttpVersion},
};
use reqwest::{Client, Method, Response, Version};

use crate::templater::template;

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

    fn body(&self) -> String {
        self.0.body.clone().unwrap_or_default()
    }

    fn request_headers(&self) -> Vec<(&str, &str)> {
        self.0
            .headers
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect()
    }

    fn map_response_http_version(response: &Response) -> HttpVersion {
        match response.version() {
            Version::HTTP_11 => HttpVersion::one_point_one(),
            _ => todo!(),
        }
    }

    fn map_response_headers(response: &Response) -> Vec<(String, String)> {
        let mut headers = vec![];
        for (key, value) in response.headers() {
            headers.push((
                key.to_string(),
                value.to_str().expect("Shoud work").to_string(),
            ));
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

        let mut request = client.request(self.request_method(), self.request_url());

        for header in self.request_headers().into_iter() {
            request = request.header(header.0, header.1);
        }

        request = request.body(self.body());

        let response = request.send().await.expect("Request should have executed");

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
        let mut params_provider_values: HashMap<String, String> = params.provider_values.clone();

        let mut provider_values: HashMap<String, String> = HashMap::new();

        if let Some(env) = &params.env {
            provider_values.insert("env".to_string(), env.clone());
        }

        params_provider_values.extend(provider_values);

        let reqfile = template(
            &params.reqfile,
            params.env.as_deref(),
            &params.prompts,
            &params.secrets,
            &params_provider_values,
        )
        .unwrap();

        Self(reqfile.request)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use crate::types::http::{HttpStatusCode, HttpVerb};
    use httptest::{
        Expectation, Server,
        matchers::{all_of, contains, request},
        responders::status_code,
    };
    use pretty_assertions::assert_eq;

    #[tokio::test]
    async fn test_real_http_request_fetch() {
        let server = Server::run();

        server.expect(
            Expectation::matching(all_of![
                request::method("POST"),
                request::path("/test"),
                request::headers(contains(("content-type", "application/json"))),
                request::headers(contains(("x-test", "foo"))),
                request::body("test body")
            ])
            .respond_with(status_code(200).body("test response!")),
        );

        let url = server.url("/test");

        let http_request = HttpRequest {
            verb: HttpVerb("POST".to_owned()),
            target: url.to_string(),
            http_version: HttpVersion::one_point_one(),
            headers: vec![
                ("content-type".to_string(), "application/json".to_string()),
                ("x-test".to_string(), "foo".to_string()),
            ],
            body: Some("test body".to_string()),
        };

        let fetcher: HttpRequestFetcher = http_request.into();
        let response = fetcher
            .fetch()
            .await
            .expect("Should be able to make real HTTP request");

        assert_eq!(HttpVersion::one_point_one(), response.http_version);
        assert_eq!(HttpStatusCode::new(200), response.status_code);

        assert_eq!("OK", response.status_text);

        assert_eq!(Some("test response!".to_string()), response.body);
    }

    #[tokio::test]
    async fn test_real_request_params_fetch() {
        let server = Server::run();

        server.expect(
            Expectation::matching(all_of![
                request::method("POST"),
                request::path("/test"),
                request::headers(contains(("content-type", "application/json"))),
                request::headers(contains(("x-test", "bar"))),
                request::body("test body\n\n")
            ])
            .respond_with(status_code(200).body("test response!")),
        );

        let url = server.url("/test");

        let params: RequestParamsFromClient = RequestParamsFromClient {
            reqfile: format!(
                r#"
```%request
POST {url} HTTP/1.1
content-type: application/json
x-test: {{{{@foo}}}}

test body
```
            "#
            )
            .to_string(),
            env: None,
            vars: HashMap::new(),
            prompts: HashMap::new(),
            secrets: HashMap::new(),
            provider_values: HashMap::from([("foo".to_string(), "bar".to_string())]),
        };

        let fetcher: HttpRequestFetcher = params.into();
        let response = fetcher
            .fetch()
            .await
            .expect("Should be able to make real HTTP request");

        assert_eq!(HttpVersion::one_point_one(), response.http_version);
        assert_eq!(HttpStatusCode::new(200), response.status_code);

        assert_eq!("OK", response.status_text);

        assert_eq!(Some("test response!".to_string()), response.body);
    }
}
