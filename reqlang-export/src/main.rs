use clap::builder::TypedValueParser;
use diagnostics::Diagnoser;
use export::Format;
use std::{collections::HashMap, error::Error, fs, process::exit};

use clap::Parser;

/// Export a request file to another format
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to request file
    path: String,
    /// Format to export
    #[arg(
        short,
        long,
        default_value_t = Format::Http,
        value_parser = clap::builder::PossibleValuesParser::new(["http", "curl", "javascript", "powershell"])
            .map(|s| s.parse::<Format>().unwrap()),
    )]
    format: Format,
    /// Resolve with an environment
    #[arg(short, long)]
    env: String,

    /// Pass prompt values to resolve with
    #[arg(short = 'P', value_parser = parse_key_val::<String, String>)]
    prompts: Vec<(String, String)>,

    /// Pass secret values to resolve with
    #[arg(short = 'S', value_parser = parse_key_val::<String, String>)]
    secrets: Vec<(String, String)>,
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

    let prompts: HashMap<String, String> = args.prompts.into_iter().collect();
    let secrets: HashMap<String, String> = args.secrets.into_iter().collect();

    let contents = fs::read_to_string(args.path).expect("Should have been able to read the file");

    let diagnostics = Diagnoser::get_diagnostics_with_env(&contents, &args.env, &prompts, &secrets);

    if !diagnostics.is_empty() {
        eprintln!("{diagnostics:#?}");
        return;
    }

    let reqfile = parser::template(&contents, &args.env, &prompts, &secrets);

    let reqfile = match reqfile {
        Ok(reqfile) => reqfile,
        Err(err) => {
            eprintln!("There were errors parsing request file:\n\n{:#?}", err);
            exit(1);
        }
    };

    let request = reqfile.request;

    println!("{}", export::export(&request, args.format));
}
