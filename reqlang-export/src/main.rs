use clap::builder::TypedValueParser;
use std::{collections::HashMap, fmt::Display, fs, process::exit};

use clap::Parser;

/// Export a request file to another format
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to request file
    path: String,
    /// Format to export
    #[arg(
        long,
        default_value_t = Format::Http,
        value_parser = clap::builder::PossibleValuesParser::new(["http", "curl", "javascript", "powershell"])
            .map(|s| s.parse::<Format>().unwrap()),
    )]
    format: Format,
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

    let contents = fs::read_to_string(args.path).expect("Should have been able to read the file");

    let reqfile = parser::resolve(&contents, "dev", HashMap::new(), HashMap::new());

    let reqfile = match reqfile {
        Ok(reqfile) => reqfile,
        Err(err) => {
            eprintln!("There were errors parsing request file:\n\n{:#?}", err);
            exit(1);
        }
    };

    let request = reqfile.request.0;

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
