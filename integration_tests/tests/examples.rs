use std::fs;
use std::path::PathBuf;

#[rstest::rstest]
fn integration_valid(#[files("../examples/valid/*.reqlang")] path: PathBuf) {
    let source = fs::read_to_string(path).expect("unable to read test file");

    assert!(reqlang::parse(&source).is_ok());
}

#[rstest::rstest]
fn integration_invalid(#[files("../examples/invalid/*.reqlang")] path: PathBuf) {
    let source = fs::read_to_string(path).expect("unable to read test file");

    assert!(reqlang::parse(&source).is_err());
}
