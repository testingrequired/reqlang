#[cfg(test)]
mod cli_integration_tests {
    use std::fs;

    use assert_cmd::Command;
    use reqlang::{ast, parse, ParseResult};

    macro_rules! assert_command {
        ($command:expr) => {{
            let mut args: Vec<&str> = $command.split_whitespace().collect();

            let command_name = args.remove(0);

            let mut cmd = Command::cargo_bin(command_name).unwrap();

            for arg in args {
                cmd.arg(arg);
            }

            let assert = cmd.assert();
            assert
        }};
    }

    #[test]
    fn no_args() {
        let mut cmd = Command::cargo_bin("reqlang").unwrap();

        let assert = cmd.assert();

        let expected_stderr = textwrap::dedent(
            "
            Command to work with request files

            Usage: reqlang [COMMAND]

            Commands:
              export  Export request to specified format
              ast     Produce an AST for a request file
              parse   Parse a request file
              run     Run a request file
              help    Print this message or the help of the given subcommand(s)

            Options:
              -h, --help     Print help
              -V, --version  Print version
            ",
        )
        .trim_start()
        .to_string();

        assert.failure().stderr(expected_stderr);
    }

    #[test]
    fn ast() {
        let expected_ast =
            fs::read_to_string("../examples/valid/as_markdown.reqlang.ast.txt").unwrap();

        let assert = assert_command!("reqlang ast ../examples/valid/as_markdown.reqlang");

        assert.success().stdout(format!("{expected_ast}\n"));
    }

    #[test]
    fn invalid_subcommand() {
        let assert = assert_command!("reqlang foobar");

        let expected_stderr = textwrap::dedent(
            "
            error: unrecognized subcommand 'foobar'

            Usage: reqlang [COMMAND]

            For more information, try '--help'.
            ",
        )
        .trim_start()
        .to_string();

        assert.failure().stderr(expected_stderr);
    }

    #[test]
    fn parses_valid_reqfile() {
        let reqfile_path = "../examples/valid/post.reqlang";

        let cmd = format!("reqlang parse {reqfile_path}");
        let assert = assert_command!(cmd);

        let reqfile_source = fs::read_to_string(reqfile_path).unwrap();
        let ast = ast::Ast::from(&reqfile_source);
        let parsed_reqfile = parse(&ast).unwrap();
        let mut parse_results: ParseResult = parsed_reqfile.into();

        let assert = assert.success();
        let output = assert.get_output();
        let mut output_deserialized: ParseResult = serde_json::from_slice(&output.stdout).unwrap();

        pretty_assertions::assert_eq!(parse_results.envs.sort(), output_deserialized.envs.sort());
        pretty_assertions::assert_eq!(parse_results.vars.sort(), output_deserialized.vars.sort());
        pretty_assertions::assert_eq!(
            parse_results.prompts.sort(),
            output_deserialized.prompts.sort()
        );
        pretty_assertions::assert_eq!(
            parse_results.secrets.sort(),
            output_deserialized.secrets.sort()
        );

        pretty_assertions::assert_eq!(parse_results.request, output_deserialized.request);
    }

    #[test]
    fn parses_invalid_reqfile() {
        let reqfile_path = "../examples/invalid/empty.reqlang";

        let cmd = format!("reqlang parse {reqfile_path}");
        let assert = assert_command!(cmd);

        assert
            .failure()
            .code(1)
            .stderr("Invalid request file\n")
            .stdout(concat!(
                "[\n",
                "  {\n",
                "    \"range\": {\n",
                "      \"start\": {\n",
                "        \"line\": 0,\n",
                "        \"character\": 0\n",
                "      },\n",
                "      \"end\": {\n",
                "        \"line\": 0,\n",
                "        \"character\": 0\n",
                "      }\n",
                "    },\n",
                "    \"severity\": 1,\n",
                "    \"message\": \"ParseError: Request file requires a request be defined\"\n",
                "  }\n",
                "]\n"
            ));
    }

    #[test]
    fn export_no_args() {
        (assert_command!("reqlang export"))
            .failure()
            .stderr(concat!(
                "error: the following required arguments were not provided:\n",
                "  <path>\n",
                "\n",
                "Usage: reqlang export <path>\n",
                "\n",
                "For more information, try '--help'.\n"
            ));
    }

    #[test]
    fn export_missing_prompt() {
        (assert_command!(
            "reqlang export ../examples/valid/post.reqlang -e test -S super_secret_value=123"
        ))
        .failure()
        .code(1)
        .stderr("Invalid request file or errors when exporting\n")
        .stdout(concat!(
            "[\n",
            "  {\n",
            "    \"range\": {\n",
            "      \"start\": {\n",
            "        \"line\": 0,\n",
            "        \"character\": 0\n",
            "      },\n",
            "      \"end\": {\n",
            "        \"line\": 0,\n",
            "        \"character\": 0\n",
            "      }\n",
            "    },\n",
            "    \"severity\": 1,\n",
            "    \"message\": \"ResolverError: Prompt required but not passed: prompt_value\"\n",
            "  }\n",
            "]\n"
        ));
    }

    #[test]
    fn export_missing_secret() {
        (assert_command!("reqlang export ../examples/valid/post.reqlang -e test -P prompt_value=foo"))
            .failure()
            .code(1)
            .stderr("Invalid request file or errors when exporting\n")
            .stdout(concat!(
                "[\n",
                "  {\n",
                "    \"range\": {\n",
                "      \"start\": {\n",
                "        \"line\": 0,\n",
                "        \"character\": 0\n",
                "      },\n",
                "      \"end\": {\n",
                "        \"line\": 0,\n",
                "        \"character\": 0\n",
                "      }\n",
                "    },\n",
                "    \"severity\": 1,\n",
                "    \"message\": \"ResolverError: Secret required but not passed: super_secret_value\"\n",
                "  }\n",
                "]\n"
            ));
    }

    #[test]
    fn export_to_default_format() {
        (assert_command!(
            "reqlang export ../examples/valid/status_code.reqlang -P status_code=404"
        ))
        .success()
        .stdout(concat!(
            "{\n",
            "  \"verb\": \"GET\",\n",
            "  \"target\": \"https://httpbin.org/status/404\",\n",
            "  \"http_version\": \"1.1\",\n",
            "  \"headers\": [],\n",
            "  \"body\": \"\"\n",
            "}\n"
        ));
    }

    #[test]
    fn export_to_http() {
        (assert_command!(
            "reqlang export ../examples/valid/status_code.reqlang -f http -P status_code=200"
        ))
        .success()
        .stdout("GET https://httpbin.org/status/200 HTTP/1.1\n\n");
    }

    #[test]
    fn export_to_curl() {
        (assert_command!(
            "reqlang export ../examples/valid/status_code.reqlang -f curl -P status_code=204"
        ))
        .success()
        .stdout("curl https://httpbin.org/status/204 --http1.1 -v\n");
    }

    #[test]
    fn export_to_invalid_format() {
        (assert_command!("reqlang export ../examples/valid/status_code.reqlang -f invalid"))
            .failure()
            .stderr(concat!(
                "error: invalid value 'invalid' for '--format <format>'\n",
                "  [possible values: http, curl, json]\n",
                "\n",
                "For more information, try '--help'.\n"
            ));
    }

    #[test]
    fn export_invalid_prompt_value_using_space() {
        (assert_command!(
            "reqlang export ../examples/valid/status_code.reqlang -P status_code 404"
        ))
        .failure()
        .stderr(concat!(
            "error: unexpected argument \'404\' found\n",
            "\n",
            "Usage: reqlang export [OPTIONS] <path>\n",
            "\n",
            "For more information, try '--help'.\n"
        ));
    }

    #[test]
    fn export_invalid_prompt_value_just_key() {
        let assert =
            assert_command!("reqlang export ../examples/valid/status_code.reqlang -P status_code");

        assert.failure().stderr(concat!(
            "error: invalid value 'status_code' for '--prompt <prompts>': should be formatted as key=value pair: `status_code`\n",
            "\n",
            "For more information, try '--help'.\n"
        ));
    }

    #[test]
    fn export_invalid_prompt_value_just_value() {
        let assert = assert_command!("reqlang export ../examples/valid/status_code.reqlang -P 404");

        assert.failure().stderr(concat!(
            "error: invalid value '404' for '--prompt <prompts>': should be formatted as key=value pair: `404`\n",
            "\n",
            "For more information, try '--help'.\n"
        ));
    }

    #[test]
    fn export_invalid_env() {
        (assert_command!(
            "reqlang export ../examples/valid/post.reqlang -e dev -S super_secret_value=123 -P prompt_value=456"
        ))
        .failure()
        .code(1)
        .stderr("Invalid request file or errors when exporting\n")
        .stdout(concat!(
            "[\n",
            "  {\n",
            "    \"range\": {\n",
            "      \"start\": {\n",
            "        \"line\": 1,\n",
            "        \"character\": 0\n",
            "      },\n",
            "      \"end\": {\n",
            "        \"line\": 14,\n",
            "        \"character\": 26\n",
            "      }\n",
            "    },\n",
            "    \"severity\": 1,\n",
            "    \"message\": \"ResolverError: 'dev' is not a defined environment in the request file\"\n",
            "  }\n",
            "]\n"
        ));
    }

    #[test]
    fn export_no_envs_defined() {
        (assert_command!(
            "reqlang export ../examples/valid/status_code.reqlang -e dev -P status_code=200"
        ))
        .failure()
        .code(1)
        .stderr("Invalid request file or errors when exporting\n")
        .stdout(concat!(
            "[\n",
            "  {\n",
            "    \"range\": {\n",
            "      \"start\": {\n",
            "        \"line\": 1,\n",
            "        \"character\": 0\n",
            "      },\n",
            "      \"end\": {\n",
            "        \"line\": 3,\n",
            "        \"character\": 16\n",
            "      }\n",
            "    },\n",
            "    \"severity\": 1,\n",
            "    \"message\": \"ResolverError: Trying to resolve the environment 'dev' but no environments are defined in the request file\"\n",
            "  }\n",
            "]\n"
        ));
    }

    #[test]
    fn run_status_code_request_file() {
        (assert_command!(
            "reqlang run ../examples/valid/status_code.reqlang --prompt status_code=200"
        ))
        .success()
        .code(0);
    }

    #[test]
    fn run_status_code_request_file_with_response_assertion() {
        (assert_command!(
            "reqlang run ../examples/valid/status_code.reqlang --prompt status_code=200 --test"
        ))
        .success()
        .code(0);
    }

    #[test]
    fn run_invalid_prompt_value_using_space() {
        let assert =
            assert_command!("reqlang run ../examples/valid/status_code.reqlang -P status_code 404");

        assert.failure().stderr(concat!(
            "error: unexpected argument \'404\' found\n",
            "\n",
            "Usage: reqlang run [OPTIONS] <path>\n",
            "\n",
            "For more information, try '--help'.\n"
        ));
    }

    #[test]
    fn run_invalid_prompt_value_just_key() {
        let assert =
            assert_command!("reqlang run ../examples/valid/status_code.reqlang -P status_code");

        assert.failure().stderr(concat!(
            "error: invalid value 'status_code' for '--prompt <prompts>': should be formatted as key=value pair: `status_code`\n",
            "\n",
            "For more information, try '--help'.\n"
        ));
    }

    #[test]
    fn run_invalid_prompt_value_just_value() {
        let assert = assert_command!("reqlang run ../examples/valid/status_code.reqlang -P 404");

        assert.failure().stderr(concat!(
            "error: invalid value '404' for '--prompt <prompts>': should be formatted as key=value pair: `404`\n",
            "\n",
            "For more information, try '--help'.\n"
        ));
    }

    #[test]
    fn run_with_invalid_format() {
        let assert =
            assert_command!("reqlang run ../examples/valid/status_code.reqlang -f invalid");

        assert.failure().stderr(concat!(
            "error: invalid value 'invalid' for '--format <format>'\n",
            "  [possible values: http, json]\n",
            "\n",
            "For more information, try '--help'.\n"
        ));
    }

    #[test]
    fn run_mismatch_response_with_response_assertion() {
        let expected_stderr = textwrap::dedent(
            r#"
              Response assertion failed:
              
              -HTTP/1.1 201 Created
              +HTTP/1.1 200 OK
              -x-test-value: ...
              
               {
                 "slideshow": {
              -    "author": "Yours Truly",
              +    "author": "Yours Truly", 
              +    "date": "date of publication", 
                   "slides": [
                     {
                       "title": "Wake up to WonderWidgets!", 
                       "type": "all"
                     }, 
                     {
                       "items": [
                         "Why <em>WonderWidgets</em> are great", 
                         "Who <em>buys</em> WonderWidgets"
                       ], 
                       "title": "Overview", 
                       "type": "all"
                     }
                   ], 
              -    "title": "Test Slide Show"
              -  },
              -  "extra": true
              +    "title": "Sample Slide Show"
              +  }
               }
              -
              
            "#,
        )
        .trim_start()
        .to_string();

        (assert_command!("reqlang run ../examples/valid/mismatch_response.reqlang --test"))
            .failure()
            .code(1)
            .stderr(expected_stderr);
    }

    #[test]
    fn run_mismatch_response_without_response_assertion() {
        assert_command!("reqlang run ../examples/valid/mismatch_response.reqlang")
            .success()
            .code(0);
    }

    #[test]
    fn run_invalid_env() {
        (assert_command!(
            "reqlang run ../examples/valid/post.reqlang -e dev -S super_secret_value=123 -P prompt_value=456"
        ))
        .failure()
        .code(1)
        .stderr("Invalid request file or errors with input\n")
        .stdout(concat!(
            "[\n",
            "  {\n",
            "    \"range\": {\n",
            "      \"start\": {\n",
            "        \"line\": 1,\n",
            "        \"character\": 0\n",
            "      },\n",
            "      \"end\": {\n",
            "        \"line\": 14,\n",
            "        \"character\": 26\n",
            "      }\n",
            "    },\n",
            "    \"severity\": 1,\n",
            "    \"message\": \"ResolverError: 'dev' is not a defined environment in the request file\"\n",
            "  }\n",
            "]\n"
        ));
    }

    #[test]
    fn run_no_envs_defined() {
        (assert_command!(
            "reqlang run ../examples/valid/status_code.reqlang -e dev -P status_code=200"
        ))
        .failure()
        .code(1)
        .stderr("Invalid request file or errors with input\n")
        .stdout(concat!(
            "[\n",
            "  {\n",
            "    \"range\": {\n",
            "      \"start\": {\n",
            "        \"line\": 1,\n",
            "        \"character\": 0\n",
            "      },\n",
            "      \"end\": {\n",
            "        \"line\": 3,\n",
            "        \"character\": 16\n",
            "      }\n",
            "    },\n",
            "    \"severity\": 1,\n",
            "    \"message\": \"ResolverError: Trying to resolve the environment 'dev' but no environments are defined in the request file\"\n",
            "  }\n",
            "]\n"
        ));
    }

    #[test]
    fn run_undefined_in_envs() {
        let expected_stderr = textwrap::dedent(
            r#"
              [
                {
                  "range": {
                    "start": {
                      "line": 1,
                      "character": 0
                    },
                    "end": {
                      "line": 1,
                      "character": 14
                    }
                  },
                  "severity": 1,
                  "message": "ParseError: Variable 'foo' is not defined in any environment or no environments are defined"
                }
              ]
              "#,
        )
        .trim_start()
        .to_string();

        (assert_command!("reqlang run ../examples/invalid/undefined_in_envs.reqlang"))
            .failure()
            .code(1)
            .stdout(expected_stderr);
    }

    #[test]
    fn run_undefined_in_envs_b() {
        let expected_stderr = textwrap::dedent(
            r#"
              [
                {
                  "range": {
                    "start": {
                      "line": 1,
                      "character": 0
                    },
                    "end": {
                      "line": 3,
                      "character": 6
                    }
                  },
                  "severity": 1,
                  "message": "ParseError: Variable 'foo' is not defined in any environment or no environments are defined"
                }
              ]
              "#,
        )
        .trim_start()
        .to_string();

        (assert_command!("reqlang run ../examples/invalid/undefined_in_envs_b.reqlang"))
            .failure()
            .code(1)
            .stdout(expected_stderr);
    }

    #[test]
    fn run_undefined_in_env() {
        let expected_stderr = textwrap::dedent(
            r#"
              [
                {
                  "range": {
                    "start": {
                      "line": 1,
                      "character": 0
                    },
                    "end": {
                      "line": 6,
                      "character": 12
                    }
                  },
                  "severity": 1,
                  "message": "ParseError: Variable 'foo' is undefined in the environment 'local'"
                }
              ]
              "#,
        )
        .trim_start()
        .to_string();

        (assert_command!("reqlang run ../examples/invalid/undefined_in_env.reqlang"))
            .failure()
            .code(1)
            .stdout(expected_stderr);
    }
}
