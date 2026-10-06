mod cli;
mod codec;
mod error;
mod input;
mod list;
mod path;
mod storage;

use clap::Parser;
use cli::{resolve_set_input_source, Cli, Commands, GetArgs, SetArgs, SetInputSource, UnsetArgs};
use codec::{decode_comment, encode_comment, IonFile, EOL};
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

fn load_or_init_ion(path: &std::path::Path) -> Result<IonFile, DionError> {
    if path.exists() {
        let bytes = fs::read(path).map_err(DionError::Io)?;
        IonFile::parse(&bytes)
    } else {
        Ok(IonFile::new())
    }
}

fn handle_set(args: SetArgs) -> Result<(), DionError> {
    let source = resolve_set_input_source(
        args.comment.as_deref(),
        args.stdin,
        args.edit,
    )?;

    let target = resolve_target(&args.target_path)?;
    let ion_path = target.parent_dir.join("descript.ion");

    let comment = match source {
        SetInputSource::Direct(s) => s,
        SetInputSource::Stdin => input::read_comment_from_reader(std::io::stdin())?,
        SetInputSource::Editor => {
            if !input::is_tty() {
                return Err(DionError::Usage(
                    "interactive editor (-e) requires a TTY terminal".to_string(),
                ));
            }
            let initial = if ion_path.exists() {
                let ion_file = load_or_init_ion(&ion_path)?;
                ion_file
                    .find_entry(&target.entry_name)
                    .map(|entry| decode_comment(&entry.raw_comment).into_owned())
                    .unwrap_or_default()
            } else {
                String::new()
            };
            input::run_editor(&initial)?
        }
    };

    if comment.trim().is_empty() {
        return handle_unset(UnsetArgs {
            target_path: args.target_path,
        });
    }

    let mut ion_file = load_or_init_ion(&ion_path)?;

    let encoded_comment = encode_comment(&comment);
    ion_file.update_or_insert(&target.entry_name, &encoded_comment);

    let bytes = ion_file.serialize()?;
    storage::atomic_write_ion(&target.parent_dir, &bytes)?;

    Ok(())
}

fn handle_unset(args: UnsetArgs) -> Result<(), DionError> {
    let target = resolve_target(&args.target_path)?;
    let ion_path = target.parent_dir.join("descript.ion");

    let bytes = match fs::read(&ion_path) {
        Ok(b) => b,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(DionError::Io(e)),
    };
    let mut ion_file = IonFile::parse(&bytes)?;

    let removed = ion_file.remove(&target.entry_name);

    if ion_file.is_empty() {
        storage::remove_ion(&target.parent_dir)?;
    } else if removed {
        let bytes = ion_file.serialize()?;
        storage::atomic_write_ion(&target.parent_dir, &bytes)?;
    }

    Ok(())
}

fn run(cli: Cli) -> Result<(), DionError> {
    match cli.command {
        Commands::Get(args) => handle_get(args),
        Commands::Set(args) => handle_set(args),
        Commands::Unset(args) => handle_unset(args),
        Commands::List(args) => list::handle_list(args),
    }
}

fn main() {
    let cli = Cli::parse();
    let quiet = match &cli.command {
        Commands::Get(args) => args.quiet,
        Commands::Set(_) | Commands::Unset(_) | Commands::List(_) => false,
    };

    if let Err(err) = run(cli) {
        let code = err.exit_code();
        if !(code == 1 && quiet) {
            eprintln!("error: {err}");
        }
        std::process::exit(code);
    }
}
