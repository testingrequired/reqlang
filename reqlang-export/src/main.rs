use clap::builder::TypedValueParser;
use diagnostics::Diagnoser;
use std::{collections::HashMap, error::Error, fmt::Display, fs, process::exit};

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

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
enum Format {
    Http,
    Curl,
    Javascript,
    Powershell,
}

impl Display for Format {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Format::Http => write!(f, "http"),
            Format::Curl => write!(f, "curl"),
            Format::Javascript => write!(f, "javascript"),
            Format::Powershell => write!(f, "powershell"),
        }
    }
}

impl std::str::FromStr for Format {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "http" => Ok(Self::Http),
            "curl" => Ok(Self::Curl),
            "javascript" => Ok(Self::Javascript),
            "powershell" => Ok(Self::Powershell),
            _ => Err(format!("Unknown format: {s}")),
        }
    }
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

    match args.format {
        Format::Http => {
            println!("{}", request);
        }
        Format::Curl => {
            let verb = if request.verb == "GET" {
                "".to_string()
            } else {
                format!("-X {}", request.verb)
            };

            let target = request.target;
            let headers: String = request
                .headers
                .iter()
                .map(|x| format!(r#"-H "{}: {}""#, x.0, x.1))
                .collect::<Vec<String>>()
                .join(" ");

            let data = match request.body {
                Some(body) => match body.is_empty() {
                    true => "".to_string(),
                    false => format!("-d '{body}'"),
                },
                None => "".to_string(),
            };

            println!(
                "curl {} {} --http{} {} {}",
                verb, target, request.http_version, headers, data
            );
        }
        Format::Powershell => {
            let headers: Vec<String> = request
                .headers
                .iter()
                .map(|x| format!(r#"'{}' = '{}'"#, x.0, x.1))
                .collect();

            let header_values = format!("{}", headers.join("; "));

            let header_arg = if headers.is_empty() {
                ""
            } else {
                "-Headers $headers"
            };

            let body_arg = if request.body.is_some() {
                "-Body $body"
            } else {
                ""
            };

            let body_value = request.body.unwrap_or_default();

            println!(
                "$headers = @{{ {} }}\n$body = '{}'\nInvoke-RestMethod -HttpVersion {} -Uri {} -Method {} {} {}",
                header_values,
                body_value,
                request.http_version,
                request.target,
                request.verb,
                header_arg,
                body_arg
            );
        }
        Format::Javascript => {
            println!("Exporting to javascript isn't support yet");
        }
    };
}
