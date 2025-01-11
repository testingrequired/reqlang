use clap::builder::PossibleValuesParser;
use clap::{Arg, ArgMatches, Command};
use std::{collections::HashMap, fs, process::exit};

use reqlang::{
    diagnostics::Diagnoser,
    errors::ReqlangError,
    export::{export, Format},
    template, Spanned,
};

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

fn map_errs(errs: &[Spanned<ReqlangError>]) -> String {
    let err = errs
        .iter()
        .map(|x| format!("{} ({:?})", x.0, x.1))
        .collect::<Vec<_>>()
        .join("\n- ");

    format!("Errors:\n\n- {err}")
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

    let diagnostics = Diagnoser::get_diagnostics_with_env(&contents, env, &prompts, &secrets);

    if !diagnostics.is_empty() {
        eprintln!("{diagnostics:#?}");
        exit(1);
    }

    let provider_values = HashMap::from([(String::from("env"), env.clone())]);

    let reqfile = template(&contents, env, &prompts, &secrets, &provider_values);

    let reqfile = match reqfile {
        Ok(reqfile) => reqfile,
        Err(errs) => {
            let err = map_errs(&errs);
            eprintln!("{err}");
            exit(1);
        }
    };

    let exported_request = export(&reqfile.request, format);

    println!("{}", exported_request);
}

fn validate_command(matches: &ArgMatches) {
    let path = matches.get_one::<String>("path").unwrap();
    let contents = fs::read_to_string(path).expect("Should have been able to read the file");

    let diagnostics = Diagnoser::get_diagnostics(&contents);

    if !diagnostics.is_empty() {
        eprintln!("{diagnostics:#?}");
        exit(1);
    }

    println!("Valid!");
}

fn main() {
    let matches = Command::new("reqlang")
        .version("1.0")
        .author("Your Name <your.email@example.com>")
        .about("CLI for reqlang")
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
                        .default_value("http")
                        .value_parser(PossibleValuesParser::new(["http", "curl"]))
                        .help("Format to export"),
                ),
        )
        .subcommand(
            Command::new("validate")
                .about("Validate a request file")
                .arg(Arg::new("path").required(true).help("Path to request file")),
        )
        .get_matches();

    match matches.subcommand() {
        Some(("export", sub_matches)) => export_command(sub_matches),
        Some(("validate", sub_matches)) => validate_command(sub_matches),
        _ => eprintln!("No valid subcommand provided. Use --help for more information."),
    }
}

#[cfg(test)]
mod tests {
    use assert_cmd::Command;

    #[test]
    fn no_args() {
        let mut cmd = Command::cargo_bin("reqlang").unwrap();

        let assert = cmd.assert();

        assert
            .success()
            .stderr("No valid subcommand provided. Use --help for more information.\n");
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

        assert.failure().code(1).stderr(concat!(
            "[\n",
            "    Diagnosis {\n",
            "        range: DiagnosisRange {\n",
            "            start: DiagnosisPosition {\n",
            "                line: 0,\n",
            "                character: 0,\n",
            "            },\n",
            "            end: DiagnosisPosition {\n",
            "                line: 0,\n",
            "                character: 0,\n",
            "            },\n",
            "        },\n",
            "        severity: Some(\n",
            "            DiagnosisSeverity(\n",
            "                1,\n",
            "            ),\n",
            "        ),\n",
            "        message: \"ResolverError: Prompt required but not passed: prompt_value\",\n",
            "    },\n",
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

        assert.failure().code(1).stderr(concat!(
            "[\n",
            "    Diagnosis {\n",
            "        range: DiagnosisRange {\n",
            "            start: DiagnosisPosition {\n",
            "                line: 0,\n",
            "                character: 0,\n",
            "            },\n",
            "            end: DiagnosisPosition {\n",
            "                line: 0,\n",
            "                character: 0,\n",
            "            },\n",
            "        },\n",
            "        severity: Some(\n",
            "            DiagnosisSeverity(\n",
            "                1,\n",
            "            ),\n",
            "        ),\n",
            "        message: \"ResolverError: Secret required but not passed: super_secret_value\",\n",
            "    },\n",
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

        assert
            .success()
            .stdout("GET https://httpbin.org/status/404 HTTP/1.1\n\n");
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
            "  [possible values: http, curl]\n",
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
