use clap::builder::TypedValueParser;
use std::{fmt::Display, fs, process::exit};

use clap::Parser;
use grammar::reqlang::DocumentParser;
use lexer::Lexer;

/// Export a request file to another format
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to request file
    path: String,
    /// Format to export
    #[arg(
        long,
        default_value_t = Format::Curl,
        value_parser = clap::builder::PossibleValuesParser::new(["curl", "javascript", "powershell"])
            .map(|s| s.parse::<Format>().unwrap()),
    )]
    format: Format,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
enum Format {
    Curl,
    Javascript,
    Powershell,
}

impl Display for Format {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
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

    let lexer = Lexer::new(&contents);

    let parser = DocumentParser::new();

    let document = match parser.parse(lexer) {
        Ok(program) => program,
        Err(err) => {
            eprintln!("There were errors parsing request file:\n\n{:#?}", err);
            exit(1);
        }
    };

    match args.format {
        Format::Curl => {
            match document.request.verb.as_str() {
                "GET" => {
                    println!("curl {}", document.request.target);
                }
                _ => {
                    println!(
                        "curl -X {} {}",
                        document.request.verb, document.request.target
                    );
                }
            };
        }
        Format::Powershell => {
            println!(
                "Invoke-RestMethod -Uri {} -Method {}",
                document.request.target, document.request.verb
            );
        }
        Format::Javascript => {
            let code = format!(
                "(async () => fetch(\"{}\", {{\n\t\"method\": \"{}\"\n}})\n\t.then(res => res.text())\n\t.then(text => console.log(text)\n))();",
                document.request.target, document.request.verb
            );

            println!("{code}");
        }
    };
}
