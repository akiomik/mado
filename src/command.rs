use std::path::PathBuf;

use clap::{Subcommand, ValueHint};
use clap_complete::Shell;
use globset::Glob;

use crate::output::Format;

pub mod check;
pub mod generate_shell_completion;

#[derive(Subcommand)]
#[non_exhaustive]
pub enum Command {
    /// Check markdown on the given files or directories
    Check {
        /// List of files or directories to check
        #[arg(default_value = ".", value_hint = ValueHint::AnyPath)]
        files: Vec<PathBuf>,

        /// Output format for violations. The default format is "concise"
        #[arg(value_enum, long = "output-format")]
        output_format: Option<Format>,

        /// Only log errors
        #[arg(long, default_value_t = false)]
        quiet: bool,

        /// List of file patterns to exclude from linting
        #[arg(long, value_delimiter = ',')]
        exclude: Option<Vec<Glob>>,
    },
    /// Generate shell completion
    GenerateShellCompletion {
        /// Shell to generate a completion script
        shell: Shell,
    },
}
