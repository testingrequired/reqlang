use clap::builder::TypedValueParser;
use clap::Parser;
use std::{collections::HashMap, fs, process::exit};

use reqlang::{
    diagnostics::Diagnoser,
    errors::ReqlangError,
    export::{export, Format},
    parse, template, Spanned,
};

use std::error::Error;

/// Run a request file
#[derive(Parser, Debug)]
#[command(name="reqlang", author, version, about, long_about = None)]
struct Args {
    /// Path to request file
    path: String,

    /// Resolve with an environment
    #[arg(short, long)]
    env: Option<String>,

    /// Pass prompt values to resolve with
    #[arg(short = 'P', value_parser = parse_key_val::<String, String>)]
    prompts: Vec<(String, String)>,

    /// Pass secret values to resolve with
    #[arg(short = 'S', value_parser = parse_key_val::<String, String>)]
    secrets: Vec<(String, String)>,

    /// Format to export
    #[arg(
        short,
        long,
        default_value_t = Format::HttpMessage,
        value_parser = clap::builder::PossibleValuesParser::new(["http", "curl"])
            .map(|s| s.parse::<Format>().unwrap()),
    )]
    format: Format,
}

/// Parse a single key-value pair
fn parse_key_val<T, U>(s: &str) -> Result<(T, U), Box<dyn Error + Send + Sync + 'static>>
where
    T: std::str::FromStr,
    T::Err: Error + Send + Sync + 'static,
    U: std::str::FromStr,
    U::Err: Error + Send + Sync + 'static,
{
    let pos = s
        .find('=')
        .ok_or_else(|| format!("invalid KEY=value: no `=` found in `{s}`"))?;
    Ok((s[..pos].parse()?, s[pos + 1..].parse()?))
}

fn map_errs(errs: &[Spanned<ReqlangError>]) -> String {
    let err = errs
        .iter()
        .map(|x| format!("{} ({:?})", x.0, x.1))
        .collect::<Vec<_>>()
        .join("\n- ");

    format!("Errors:\n\n- {err}")
}

fn main() {
    let args = Args::parse();

    let contents = fs::read_to_string(args.path).expect("Should have been able to read the file");

    match args.env {
        Some(env) => {
            let prompts: HashMap<String, String> = args.prompts.into_iter().collect();
            let secrets: HashMap<String, String> = args.secrets.into_iter().collect();

            let diagnostics =
                Diagnoser::get_diagnostics_with_env(&contents, &env, &prompts, &secrets);

            if !diagnostics.is_empty() {
                eprintln!("{diagnostics:#?}");
                return;
            }

            let reqfile = template(&contents, &env, &prompts, &secrets, HashMap::default());

            let reqfile = match reqfile {
                Ok(reqfile) => reqfile,
                Err(errs) => {
                    let err = map_errs(&errs);
                    eprintln!("{err}");
                    exit(1);
                }
            };

            let exported_request = export(&reqfile.request, args.format);

            println!("{}", exported_request);
        }
        None => {
            let diagnostics = Diagnoser::get_diagnostics(&contents);

            if !diagnostics.is_empty() {
                eprintln!("{diagnostics:#?}");
                return;
            }

            let reqfile = parse(&contents);

            let reqfile = match reqfile {
                Ok(reqfile) => reqfile,
                Err(errs) => {
                    let err = map_errs(&errs);
                    eprintln!("{err}");
                    exit(1);
                }
            };

            println!("{:#?}", reqfile);
        }
    };
}

#[cfg(test)]
mod tests {
    use assert_cmd::Command;

    #[test]
    fn no_args() {
        let mut cmd = Command::cargo_bin("reqlang").unwrap();

        let assert = cmd.assert();

        assert.failure().stderr("error: the following required arguments were not provided:\n  <PATH>\n\nUsage: reqlang <PATH>\n\nFor more information, try \'--help\'.\n");
    }

    #[test]
    fn export_to_default_format() {
        let mut cmd = Command::cargo_bin("reqlang").unwrap();

        let assert = cmd
            .arg("../examples/valid/status_code.reqlang")
            .arg("-e")
            .arg("default")
            .arg("-P")
            .arg("status_code=201")
            .assert();

        assert
            .success()
            .stdout("GET https://httpbin.org/status/201 HTTP/1.1\n\n");
    }

    #[test]
    fn export_to_http() {
        let mut cmd = Command::cargo_bin("reqlang").unwrap();

        let assert = cmd
            .arg("../examples/valid/status_code.reqlang")
            .arg("-e")
            .arg("default")
            .arg("-f")
            .arg("http")
            .arg("-P")
            .arg("status_code=201")
            .assert();

        assert
            .success()
            .stdout("GET https://httpbin.org/status/201 HTTP/1.1\n\n");
    }

    #[test]
    fn export_to_curl() {
        let mut cmd = Command::cargo_bin("reqlang").unwrap();

        let assert = cmd
            .arg("../examples/valid/status_code.reqlang")
            .arg("-e")
            .arg("default")
            .arg("-f")
            .arg("curl")
            .arg("-P")
            .arg("status_code=201")
            .assert();

        assert
            .success()
            .stdout("curl https://httpbin.org/status/201 --http1.1 -v\n");
    }

    #[test]
    fn export_to_invalid_format() {
        let mut cmd = Command::cargo_bin("reqlang").unwrap();

        let assert = cmd
            .arg("../examples/valid/status_code.reqlang")
            .arg("-e")
            .arg("default")
            .arg("-f")
            .arg("invalid")
            .arg("-P")
            .arg("status_code=201")
            .assert();

        assert
            .failure()
            .stderr("error: invalid value \'invalid\' for \'--format <FORMAT>\'\n  [possible values: http, curl]\n\nFor more information, try \'--help\'.\n");
    }
}
