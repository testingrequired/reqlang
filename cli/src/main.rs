use std::{collections::HashMap, fs, process::exit};

use clap::Parser;

use diagnostics::Diagnoser;

/// Run a request file
#[derive(Parser, Debug)]
#[command(name="reqlang", author, version, about, long_about = None)]
struct Args {
    /// Path to request file
    path: String,
    /// Resolve with an environment
    #[arg(short, long)]
    env: Option<String>,
}

fn main() {
    let args = Args::parse();

    let contents = fs::read_to_string(args.path).expect("Should have been able to read the file");

    match args.env {
        Some(env) => {
            let diagnostics = Diagnoser::get_diagnostics_with_env(
                &contents,
                &env,
                HashMap::new(),
                HashMap::new(),
            );

            if !diagnostics.is_empty() {
                eprintln!("{diagnostics:#?}");
                return;
            }

            let reqfile = parser::resolve(&contents, &env, HashMap::new(), HashMap::new());

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
