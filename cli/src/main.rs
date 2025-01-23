use clap::builder::PossibleValuesParser;
use clap::{crate_authors, crate_description, crate_version, Arg, ArgMatches, Command};
use reqlang::{parse, ParseResult};
use std::{collections::HashMap, fs, process::exit};

use reqlang::{diagnostics::get_diagnostics, export, template, Format};

use std::error::Error;

/// Parse a single key-value pair
fn parse_key_val<T, U>(value: &str) -> Result<(T, U), Box<dyn Error + Send + Sync + 'static>>
where
    T: std::str::FromStr,
    T::Err: Error + Send + Sync + 'static,
    U: std::str::FromStr,
    U::Err: Error + Send + Sync + 'static,
{
    let n = 2;

    let parts: Vec<&str> = value.splitn(n, '=').collect();

    if parts.len() != n {
        return Err(format!("should be formatted as key=value pair: `{value}`").into());
    }

    let key = parts[0].parse()?;
    let value = parts[1].parse()?;

    Ok((key, value))
}

fn export_command(matches: &ArgMatches) {
    let path = matches.get_one::<String>("path").unwrap();

    let default_env = String::from("default");
    let env = matches.get_one::<String>("env").unwrap_or(&default_env);

    let prompts = matches
        .get_many::<(String, String)>("prompts")
        .map(|values| values.cloned().collect::<HashMap<String, String>>())
        .unwrap_or_default();

    let secrets = matches
        .get_many::<(String, String)>("secrets")
        .map(|values| values.cloned().collect::<HashMap<String, String>>())
        .unwrap_or_default();

    let format = matches
        .get_one::<String>("format")
        .map(|f| f.parse::<Format>().unwrap())
        .unwrap_or(Format::HttpMessage);

    let contents = fs::read_to_string(path).expect("Should have been able to read the file");

    let provider_values = HashMap::from([(String::from("env"), env.clone())]);

    let reqfile = template(&contents, env, &prompts, &secrets, &provider_values);

    match reqfile {
        Ok(reqfile) => {
            let exported_request = export(&reqfile.request, format);

            println!("{}", exported_request);
        }
        Err(errs) => {
            let diagnostics = get_diagnostics(&errs, &contents);

            if !diagnostics.is_empty() {
                eprintln!("Invalid request file or errors when exporting");
                let json = serde_json::to_string_pretty(&diagnostics).unwrap();
                println!("{json}");
                exit(1);
            }
        }
    };
}

fn parse_command(matches: &ArgMatches) {
    let path = matches.get_one::<String>("path").unwrap();
    let contents = fs::read_to_string(path).expect("Should have been able to read the file");

    match parse(&contents) {
        Ok(parsed_reqfile) => {
            let parse_results: ParseResult = parsed_reqfile.into();

            let json = serde_json::to_string_pretty(&parse_results).unwrap();

            println!("{json}");
        }
        Err(errs) => {
            let diagnostics = get_diagnostics(&errs, &contents);

            if !diagnostics.is_empty() {
                eprintln!("Invalid request file");
                let json = serde_json::to_string_pretty(&diagnostics).unwrap();
                println!("{json}");
                exit(1);
            }
        }
    }
}

fn main() {
    let matches = Command::new("reqlang")
        .version(crate_version!())
        .author(crate_authors!("\n"))
        .about(crate_description!())
        .subcommand(
            Command::new("export")
                .about("Export request to specified format")
                .arg(Arg::new("path").required(true).help("Path to request file"))
                .arg(
                    Arg::new("env")
                        .short('e')
                        .long("env")
                        .help("Resolve with an environment"),
                )
                .arg(
                    Arg::new("prompts")
                        .short('P')
                        .long("prompt")
                        .value_parser(parse_key_val::<String, String>)
                        .help("Pass prompt values to resolve with"),
                )
                .arg(
                    Arg::new("secrets")
                        .short('S')
                        .long("secret")
                        .value_parser(parse_key_val::<String, String>)
                        .help("Pass secret values to resolve with"),
                )
                .arg(
                    Arg::new("format")
                        .short('f')
                        .long("format")
                        .default_value("json")
                        .value_parser(PossibleValuesParser::new(["http", "curl", "json"]))
                        .help("Format to export"),
                ),
        )
        .subcommand(
            Command::new("parse")
                .about("Parse a request file")
                .arg(Arg::new("path").required(true).help("Path to request file")),
        )
        .get_matches();

    match matches.subcommand() {
        Some(("export", sub_matches)) => export_command(sub_matches),
        Some(("parse", sub_matches)) => parse_command(sub_matches),
        _ => eprintln!("No valid subcommand provided. Use --help for more information."),
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use assert_cmd::Command;
    use reqlang::{parse, ParseResult};

    #[test]
    fn no_args() {
        let mut cmd = Command::cargo_bin("reqlang").unwrap();

        let assert = cmd.assert();

        assert
            .success()
            .stderr("No valid subcommand provided. Use --help for more information.\n");
    }

    #[test]
    fn parses_valid_reqfile() {
        let mut cmd = Command::cargo_bin("reqlang").unwrap();

        let reqfile_path = "../examples/valid/post.reqlang";

        let assert = cmd.arg("parse").arg(reqfile_path).assert();

        let reqfile_source = fs::read_to_string(reqfile_path).unwrap();
        let parsed_reqfile = parse(&reqfile_source).unwrap();
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
        let mut cmd = Command::cargo_bin("reqlang").unwrap();

        let reqfile_path = "../examples/invalid/empty.reqlang";

        let assert = cmd.arg("parse").arg(reqfile_path).assert();

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
        let mut cmd = Command::cargo_bin("reqlang").unwrap();

        let assert = cmd.arg("export").assert();

        assert.failure().stderr(concat!(
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
        let mut cmd = Command::cargo_bin("reqlang").unwrap();

        let assert = cmd
            .arg("export")
            .arg("../examples/valid/post.reqlang")
            .arg("-e")
            .arg("dev")
            .arg("-S")
            .arg("super_secret_value=123")
            .assert();

        assert
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
        let mut cmd = Command::cargo_bin("reqlang").unwrap();

        let assert = cmd
            .arg("export")
            .arg("../examples/valid/post.reqlang")
            .arg("-e")
            .arg("dev")
            .arg("-P")
            .arg("prompt_value=foo")
            .assert();

        assert
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
        let mut cmd = Command::cargo_bin("reqlang").unwrap();

        let assert = cmd
            .arg("export")
            .arg("../examples/valid/status_code.reqlang")
            .arg("-P")
            .arg("status_code=404")
            .assert();

        assert.success().stdout(concat!(
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
        let mut cmd = Command::cargo_bin("reqlang").unwrap();

        let assert = cmd
            .arg("export")
            .arg("../examples/valid/status_code.reqlang")
            .arg("-f")
            .arg("http")
            .arg("-P")
            .arg("status_code=200")
            .assert();

        assert
            .success()
            .stdout("GET https://httpbin.org/status/200 HTTP/1.1\n\n");
    }

    #[test]
    fn export_to_curl() {
        let mut cmd = Command::cargo_bin("reqlang").unwrap();

        let assert = cmd
            .arg("export")
            .arg("../examples/valid/status_code.reqlang")
            .arg("-f")
            .arg("curl")
            .arg("-P")
            .arg("status_code=204")
            .assert();

        assert
            .success()
            .stdout("curl https://httpbin.org/status/204 --http1.1 -v\n");
    }

    #[test]
    fn export_to_invalid_format() {
        let mut cmd = Command::cargo_bin("reqlang").unwrap();

        let assert = cmd
            .arg("export")
            .arg("../examples/valid/status_code.reqlang")
            .arg("-f")
            .arg("invalid")
            .assert();

        assert.failure().stderr(concat!(
            "error: invalid value 'invalid' for '--format <format>'\n",
            "  [possible values: http, curl, json]\n",
            "\n",
            "For more information, try '--help'.\n"
        ));
    }

    #[test]
    fn export_invalid_prompt_value_using_space() {
        let mut cmd = Command::cargo_bin("reqlang").unwrap();

        let assert = cmd
            .arg("export")
            .arg("../examples/valid/status_code.reqlang")
            .arg("-P")
            .arg("status_code 404")
            .assert();

        assert.failure().stderr(concat!(
            "error: invalid value 'status_code 404' for '--prompt <prompts>': should be formatted as key=value pair: `status_code 404`\n",
            "\n",
            "For more information, try '--help'.\n"
        ));
    }

    #[test]
    fn export_invalid_prompt_value_just_key() {
        let mut cmd = Command::cargo_bin("reqlang").unwrap();

        let assert = cmd
            .arg("export")
            .arg("../examples/valid/status_code.reqlang")
            .arg("-P")
            .arg("status_code")
            .assert();

        assert.failure().stderr(concat!(
            "error: invalid value 'status_code' for '--prompt <prompts>': should be formatted as key=value pair: `status_code`\n",
            "\n",
            "For more information, try '--help'.\n"
        ));
    }

    #[test]
    fn export_invalid_prompt_value_just_value() {
        let mut cmd = Command::cargo_bin("reqlang").unwrap();

        let assert = cmd
            .arg("export")
            .arg("../examples/valid/status_code.reqlang")
            .arg("-P")
            .arg("404")
            .assert();

        assert.failure().stderr(concat!(
            "error: invalid value '404' for '--prompt <prompts>': should be formatted as key=value pair: `404`\n",
            "\n",
            "For more information, try '--help'.\n"
        ));
    }
}
