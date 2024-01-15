use std::{fs, process::exit};

use clap::Parser;
use grammar::reqlang::DocumentParser;
use lexer::Lexer;

/// Run a request file
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to request file
    path: String,
}

fn main() {
    let args = Args::parse();

    let contents = fs::read_to_string(args.path).expect("Should have been able to read the file");

    let lexer = Lexer::new(&contents);

    let parser = DocumentParser::new();

    let document = match parser.parse(lexer) {
        Ok(program) => program,
        Err(err) => {
            eprintln!("Errors parsing file: {:?}", err);
            exit(1);
        }
    };

    println!("{:#?}", document);
}
