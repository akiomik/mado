use std::path::PathBuf;

use clap::Subcommand;

use crate::output::Format;

pub mod check;

#[derive(Subcommand)]
#[non_exhaustive]
pub enum Command {
    /// Check markdown on the given files or directories
    Check {
        /// List of files or directories to check
        #[arg(default_value = ".")]
        files: Vec<PathBuf>,

        /// Output format for violations. The default format is "concise"
        #[arg(value_enum, long = "output-format")]
        output_format: Option<Format>,

        /// Only log errors
        #[arg(long, default_value_t = false)]
        quiet: bool,
    },
}
