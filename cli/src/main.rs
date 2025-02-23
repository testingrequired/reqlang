use clap::builder::PossibleValuesParser;
use clap::{crate_authors, crate_description, crate_version, Arg, ArgMatches, Command};
use reqlang::{
    assert_response::assert_response, export_response, parse, Ast, HttpRequestFetcher, ParseResult,
    ResponseFormat,
};
use std::{collections::HashMap, fs, process::exit};

use reqlang::{diagnostics::get_diagnostics, export, template, Fetch, RequestFormat};

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

    let env: Option<&str> = matches.get_one::<String>("env").map(|x| x.as_str());

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
        .map(|f| f.parse::<RequestFormat>().unwrap())
        .unwrap_or(RequestFormat::HttpMessage);

    let contents = fs::read_to_string(path).expect("Should have been able to read the file");

    let provider_values = HashMap::from([(
        String::from("env"),
        env.map(|x| x.to_string()).unwrap_or_default(),
    )]);

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
    let ast = Ast::new(&contents);

    match parse(&ast) {
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

async fn run_command(matches: &ArgMatches) {
    // CLI Args

    let path = matches.get_one::<String>("path").unwrap();

    let env = matches.get_one::<String>("env").map(|s| s.as_str());

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
        .map(|f| f.parse::<ResponseFormat>().unwrap())
        .unwrap_or(ResponseFormat::HttpMessage);

    let is_testing_response = matches.get_flag("test");

    // Read the request file

    let contents = fs::read_to_string(path).expect("Should have been able to read the file");
    let provider_values = HashMap::from([(
        String::from("env"),
        env.map(|x| x.to_string()).unwrap_or_default(),
    )]);
    let reqfile = template(&contents, env, &prompts, &secrets, &provider_values);

    // Execute the request

    match reqfile {
        Ok(reqfile) => {
            let fetcher: HttpRequestFetcher = reqfile.request.into();
            let response = fetcher.fetch().await;

            match &response {
                Ok(response) => {
                    // Format the response as specified by the `--format` flag
                    let formatted_response = export_response(response, format);

                    println!("{}", formatted_response);

                    // Check if the `--test` flag was passed
                    if is_testing_response {
                        // Check if the request file has a response assertion defined
                        if let Some(expected_response) = &reqfile.response {
                            // Compare the actual response with the expected response
                            if let Err(diffs) = assert_response(expected_response, response) {
                                eprintln!("Response assertion failed:\n{diffs}");

                                exit(1);
                            }
                        }
                    }

                    exit(0);
                }
                Err(err) => {
                    eprintln!("Error occurred while making the request: {err:?}");
                    exit(1);
                }
            }
        }
        Err(errs) => {
            let diagnostics = get_diagnostics(&errs, &contents);

            if !diagnostics.is_empty() {
                eprintln!("Invalid request file or errors with input");
                let json = serde_json::to_string_pretty(&diagnostics).unwrap();
                println!("{json}");
                exit(1);
            }
        }
    };
}

#[tokio::main]
async fn main() {
    let path_arg = Arg::new("path").required(true).help("Path to request file");

    let matches = Command::new("reqlang")
        .version(crate_version!())
        .author(crate_authors!("\n"))
        .about(crate_description!())
        .arg_required_else_help(true)
        .subcommand(
            Command::new("export")
                .about("Export request to specified format")
                .arg(path_arg.clone())
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
                .arg(path_arg.clone()),
        )
        .subcommand(
            Command::new("run")
                .about("Run a request file")
                .arg(path_arg.clone())
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
                        .help("Input a prompt value"),
                )
                .arg(
                    Arg::new("secrets")
                        .short('S')
                        .long("secret")
                        .value_parser(parse_key_val::<String, String>)
                        .help("Input a secret value"),
                )
                .arg(
                    Arg::new("format")
                        .short('f')
                        .long("format")
                        .default_value("http")
                        .value_parser(PossibleValuesParser::new(["http", "json"]))
                        .help("Format the response"),
                )
                .arg(
                    // an bool flag called test
                    Arg::new("test")
                        .short('t')
                        .long("test")
                        .num_args(0)
                        .help("Test if the response matches the expected response, if defined"),
                ),
        )
        .get_matches();

    match matches.subcommand() {
        Some(("export", sub_matches)) => export_command(sub_matches),
        Some(("parse", sub_matches)) => parse_command(sub_matches),
        Some(("run", sub_matches)) => run_command(sub_matches).await,
        _ => eprintln!("Invalid subcommand. Use --help for more information."),
    }
}
