use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "dion", version, about = "Windows native CLI for Total Commander descript.ion")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Read comment for a target file or directory
    #[command(alias = "view", alias = "cat")]
    Get(GetArgs),

    /// Set or update comment for a target file or directory
    Set(SetArgs),

    /// Remove comment for a target file or directory
    #[command(alias = "rm", alias = "del")]
    Unset(UnsetArgs),
}

#[derive(clap::Args, Debug)]
pub struct UnsetArgs {
    /// Target file or directory path
    pub target_path: String,
}

#[derive(clap::Args, Debug)]
pub struct SetArgs {
    /// Target file or directory path
    pub target_path: String,

    /// Comment text content (use '-' for standard input)
    pub comment: Option<String>,

    /// Read comment text from standard input (stdin)
    #[arg(long)]
    pub stdin: bool,

    /// Edit comment interactively in terminal using external editor
    #[arg(short = 'e', long = "edit")]
    pub edit: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub enum SetInputSource {
    Direct(String),
    Stdin,
    Editor,
}

pub fn resolve_set_input_source(
    comment: Option<&str>,
    stdin: bool,
    edit: bool,
) -> Result<SetInputSource, crate::error::DionError> {
    let positional_source = match comment {
        None => None,
        Some("-") => Some(SetInputSource::Stdin),
        Some(s) => Some(SetInputSource::Direct(s.to_string())),
    };

    let stdin_source = if stdin {
        Some(SetInputSource::Stdin)
    } else {
        None
    };

    let editor_source = if edit {
        Some(SetInputSource::Editor)
    } else {
        None
    };

    match (positional_source, stdin_source, editor_source) {
        (Some(src), None, None) => Ok(src),
        (None, Some(src), None) => Ok(src),
        (None, None, Some(src)) => Ok(src),
        (None, None, None) => Err(crate::error::DionError::Usage(
            "missing comment input source: specify a comment argument, '-' or '--stdin', or '-e'".to_string(),
        )),
        _ => Err(crate::error::DionError::Usage(
            "conflicting comment input sources: specify only one of comment argument, '--stdin'/'-', or '-e'".to_string(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_input_source_single_direct() {
        let res = resolve_set_input_source(Some("hello"), false, false).unwrap();
        assert_eq!(res, SetInputSource::Direct("hello".to_string()));
    }

    #[test]
    fn test_resolve_input_source_dash_as_stdin() {
        let res = resolve_set_input_source(Some("-"), false, false).unwrap();
        assert_eq!(res, SetInputSource::Stdin);
    }

    #[test]
    fn test_resolve_input_source_stdin_flag() {
        let res = resolve_set_input_source(None, true, false).unwrap();
        assert_eq!(res, SetInputSource::Stdin);
    }

    #[test]
    fn test_resolve_input_source_editor_flag() {
        let res = resolve_set_input_source(None, false, true).unwrap();
        assert_eq!(res, SetInputSource::Editor);
    }

    #[test]
    fn test_resolve_input_source_missing_all_returns_usage_error() {
        let res = resolve_set_input_source(None, false, false);
        let err = res.unwrap_err();
        assert_eq!(err.exit_code(), 2);
    }

    #[test]
    fn test_resolve_input_source_conflicts_return_usage_error() {
        // Direct + stdin flag
        let err1 = resolve_set_input_source(Some("text"), true, false).unwrap_err();
        assert_eq!(err1.exit_code(), 2);

        // Direct + edit flag
        let err2 = resolve_set_input_source(Some("text"), false, true).unwrap_err();
        assert_eq!(err2.exit_code(), 2);

        // Dash + stdin flag
        let err3 = resolve_set_input_source(Some("-"), true, false).unwrap_err();
        assert_eq!(err3.exit_code(), 2);

        // Dash + edit flag
        let err4 = resolve_set_input_source(Some("-"), false, true).unwrap_err();
        assert_eq!(err4.exit_code(), 2);

        // stdin flag + edit flag
        let err5 = resolve_set_input_source(None, true, true).unwrap_err();
        assert_eq!(err5.exit_code(), 2);

        // all three
        let err6 = resolve_set_input_source(Some("text"), true, true).unwrap_err();
        assert_eq!(err6.exit_code(), 2);
    }
}

#[derive(clap::Args, Debug)]
pub struct GetArgs {
    /// Target file or directory path
    pub target_path: String,

    /// Print raw un-decoded physical storage format
    #[arg(long, conflicts_with = "json")]
    pub raw: bool,

    /// Output as structured JSON object
    #[arg(long, conflicts_with = "raw")]
    pub json: bool,

    /// Suppress error output when comment is not found
    #[arg(short, long)]
    pub quiet: bool,
}
