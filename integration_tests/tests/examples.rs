use pretty_assertions::assert_eq;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use reqlang::{
    fetch::{Fetch, HttpRequestFetcher},
    http::{HttpResponse, HttpStatusCode, HttpVersion},
    template,
};

#[rstest::rstest]
fn integration_valid(#[files("../examples/valid/*.reqlang")] path: PathBuf) {
    let source = fs::read_to_string(path).expect("Should get text from file");

    assert!(reqlang::parse(&source).is_ok());
}

#[rstest::rstest]
fn integration_invalid(#[files("../examples/invalid/*.reqlang")] path: PathBuf) {
    let source = fs::read_to_string(path).expect("Should get text from file");

    assert!(reqlang::parse(&source).is_err());
}

#[tokio::test]
async fn integration_status_code_reqfile() {
    let path = PathBuf::from("../examples/valid/status_code.reqlang");
    let source = fs::read_to_string(path).expect("unable to read test file");

    let env = "default";
    let prompts = HashMap::from([("status_code".to_string(), "201".to_string())]);
    let secrets = HashMap::new();
    let provider_values = HashMap::from([("env".to_string(), "default".to_string())]);

    let reqfile = template(&source, env, &prompts, &secrets, provider_values)
        .expect("Request files should have templated");

    let fetcher: HttpRequestFetcher = reqfile.request.into();

    let response: HttpResponse = fetcher.fetch().await.expect("Request should have executed");

    assert_eq!(HttpVersion::one_point_one(), response.http_version);
    assert_eq!(HttpStatusCode::new(201), response.status_code);
}
