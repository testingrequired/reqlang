#[cfg(test)]
mod integration_tests {
    use pretty_assertions::assert_eq;
    use std::collections::HashMap;
    use std::fs;
    use std::path::PathBuf;

    use reqlang::{
        ast,
        fetch::{Fetch, HttpRequestFetcher},
        types::http::{HttpResponse, HttpStatusCode, HttpVersion},
    };

    #[rstest::rstest]
    fn integration_valid(#[files("../examples/valid/expr_reference_env.reqlang")] path: PathBuf) {
        let source = fs::read_to_string(path).expect("text should have been read from file");
        let ast = ast::Ast::from(&source);

        assert!(reqlang::parser::parse(&ast).is_ok());
    }

    #[rstest::rstest]
    fn integration_invalid(#[files("../examples/invalid/*.reqlang")] path: PathBuf) {
        let source = fs::read_to_string(path).expect("text should have been read from file");
        let ast = ast::Ast::from(&source);

        assert!(reqlang::parser::parse(&ast).is_err());
    }

    #[tokio::test]
    async fn integration_status_code_reqfile() {
        let path = PathBuf::from("../examples/valid/status_code.reqlang");
        let source = fs::read_to_string(path).expect("text should have been read from file");

        let prompts = HashMap::from([("status_code".to_string(), "201".to_string())]);
        let secrets = HashMap::new();
        let provider_values = HashMap::new();

        let reqfile =
            reqlang::templater::template(&source, None, &prompts, &secrets, &provider_values)
                .expect("request file should have been templated");

        let fetcher: HttpRequestFetcher = reqfile.request.into();

        let response: HttpResponse = fetcher.fetch().await.expect("request should have executed");

        assert_eq!(HttpVersion::one_point_one(), response.http_version);
        assert_eq!(HttpStatusCode::new(201), response.status_code);
    }
}
