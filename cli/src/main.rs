use std::{collections::HashMap, fs, process::exit};

use clap::Parser;

/// Run a request file
#[derive(Parser, Debug)]
#[command(name="reqlang", author, version, about, long_about = None)]
struct Args {
    /// Path to request file
    path: String,
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

    println!("{:#?}", reqfile);
}
