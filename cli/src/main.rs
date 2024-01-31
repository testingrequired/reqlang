use std::{collections::HashMap, fs, process::exit};
use clap::builder::TypedValueParser;
use clap::Parser;

use diagnostics::Diagnoser;
use export::{export, Format};


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
        default_value_t = Format::Http,
        value_parser = clap::builder::PossibleValuesParser::new(["http", "curl", "javascript", "powershell"])
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

            let reqfile = parser::template(&contents, &env, &prompts, &secrets);

            let reqfile = match reqfile {
                Ok(reqfile) => reqfile,
                Err(err) => {
                    let err = err
                        .into_iter()
                        .map(|x| format!("{} ({:?})", x.0, x.1))
                        .collect::<Vec<_>>()
                        .join("\n- ");
                    eprintln!("Errors:\n\n- {err}");
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

            let reqfile = parser::parse(&contents);

            let reqfile = match reqfile {
                Ok(reqfile) => reqfile,
                Err(err) => {
                    let err = err
                        .into_iter()
                        .map(|x| format!("{} ({:?})", x.0, x.1))
                        .collect::<Vec<_>>()
                        .join("\n- ");
                    eprintln!("Errors:\n\n- {err}");
                    exit(1);
                }
            };

            println!("{:#?}", reqfile);
        }
    };
}
