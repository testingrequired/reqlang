use std::{collections::HashMap, future::Future};

use parser::template;
use reqwest::{Client, Method, Version};
use types::{
    http::{HttpResponse, HttpVersion},
    RequestParamsFromClient,
};

/// Fetch trait for making HTTP requests
pub trait Fetch {
    fn fetch(
        &self,
    ) -> impl Future<Output = Result<HttpResponse, Box<dyn std::error::Error + Send>>> + Send;
}

/// Make a request from params sent from the client
pub struct RequestParamsFromClientFetcher<'a>(pub &'a RequestParamsFromClient);

impl<'a> Fetch for RequestParamsFromClientFetcher<'a> {
    async fn fetch(&self) -> std::result::Result<HttpResponse, Box<dyn std::error::Error + Send>> {
        let params = self.0;

        // The request file text
        let text = &params.reqfile;

        // The environment to execute the request in
        let env = params.env.as_str();

        // Provider values are template values provided by the client
        let mut provider = HashMap::new();
        provider.insert("env".to_string(), env.to_string());

        // Template the reqfile
        let reqfile = template(&text, env, &params.prompts, &params.secrets, provider)
            .expect("Should have templated");

        let request_method: Method = match reqfile.request.verb.to_string().as_str() {
            "GET" => Method::GET,
            "POST" => Method::POST,
            _ => todo!(),
        };

        let request_url = reqfile.request.target;

        let mut request_builder = Client::new().request(request_method, request_url);

        for (key, value) in &reqfile.request.headers {
            request_builder = request_builder.header(key, value);
        }

        if let Some(body) = reqfile.request.body {
            if !body.is_empty() {
                request_builder = request_builder.body(body);
            }
        }

        // Execute the request and get the response
        let response = request_builder.send().await.expect("Should not error");

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
