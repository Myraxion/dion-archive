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
}

#[derive(clap::Args, Debug)]
pub struct SetArgs {
    /// Target file or directory path
    pub target_path: String,

    /// Comment text content
    pub comment: String,
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
