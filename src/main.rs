mod cli;
mod codec;
mod error;
mod path;

use clap::Parser;
use cli::{Cli, Commands, GetArgs};
use codec::{decode_comment, IonFile, EOL};
use error::DionError;
use path::resolve_target;
use serde::Serialize;
use std::fs;
use std::io::ErrorKind;

#[derive(Serialize)]
struct JsonComment<'a> {
    path: &'a str,
    name: &'a str,
    comment: &'a str,
}

fn handle_get(args: GetArgs) -> Result<(), DionError> {
    let target = resolve_target(&args.target_path)?;
    let ion_path = target.parent_dir.join("descript.ion");

    let bytes = match fs::read(&ion_path) {
        Ok(b) => b,
        Err(e) if e.kind() == ErrorKind::NotFound => {
            return Err(DionError::NotFound(args.target_path));
        }
        Err(e) => return Err(DionError::Io(e)),
    };

    let ion_file = IonFile::parse(&bytes)?;

    let entry = ion_file
        .find_entry(&target.entry_name)
        .ok_or_else(|| DionError::NotFound(args.target_path.clone()))?;

    if args.raw {
        print!("{}{EOL}", entry.raw_comment);
        return Ok(());
    }

    let decoded = decode_comment(&entry.raw_comment);
    if args.json {
        let obj = JsonComment {
            path: &args.target_path,
            name: &target.entry_name,
            comment: &decoded,
        };
        let json_str = serde_json::to_string(&obj)
            .map_err(|e| DionError::MalformedEntry(e.to_string()))?;
        print!("{json_str}{EOL}");
    } else {
        print!("{decoded}{EOL}");
    }

    Ok(())
}

fn run(cli: Cli) -> Result<(), DionError> {
    match cli.command {
        Commands::Get(args) => handle_get(args),
    }
}

fn main() {
    let cli = Cli::parse();
    let quiet = match &cli.command {
        Commands::Get(args) => args.quiet,
    };

    if let Err(err) = run(cli) {
        let code = err.exit_code();
        if !(code == 1 && quiet) {
            eprintln!("error: {err}");
        }
        std::process::exit(code);
    }
}
