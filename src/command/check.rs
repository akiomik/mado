use std::io::Write as _;
use std::io::{self, BufWriter};
use std::path::PathBuf;
use std::process::ExitCode;

use ignore::WalkParallel;
use miette::IntoDiagnostic as _;
use miette::Result;

use crate::output::{Concise, Format, Markdownlint, Mdl};
use crate::service::runner::ParallelLintRunner;
use crate::service::walker::WalkParallelBuilder;
use crate::Config;

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::exhaustive_structs)]
pub struct Options {
    pub config_path: Option<PathBuf>,
    pub output_format: Option<Format>,
    pub quiet: bool,
}

impl Options {
    #[inline]
    pub fn to_config(self) -> Result<Config> {
        let mut config = match self.config_path {
            Some(config_path) => Config::load(&config_path)?,
            None => Config::resolve()?,
        };

        if let Some(format) = self.output_format {
            config.lint.output_format = format;
        }

        // Respect config
        config.lint.quiet |= self.quiet;

        Ok(config)
    }
}

pub struct Checker {
    walker: WalkParallel,
    config: Config,
}

impl Checker {
    #[inline]
    pub fn new(patterns: &[PathBuf], config: Config) -> Result<Self> {
        let walker = WalkParallelBuilder::build(patterns)?;

        Ok(Self { walker, config })
    }

    #[inline]
    pub fn check(self) -> Result<ExitCode> {
        let runner = ParallelLintRunner::new(self.walker, self.config.clone(), 100);
        let mut diagnostics = runner.run()?;
        diagnostics.sort_by(self.config.lint.output_format.sorter());

        if diagnostics.is_empty() {
            if !self.config.lint.quiet {
                println!("All checks passed!");
            }
        }

        let mut output = BufWriter::new(io::stdout().lock());
        let num_diagnostics = diagnostics.len();
        for diagnostic in diagnostics {
            match self.config.lint.output_format {
                Format::Concise => {
                    writeln!(output, "{}", Concise::new(&diagnostic)).into_diagnostic()?;
                }
                Format::Mdl => writeln!(output, "{}", Mdl::new(&diagnostic)).into_diagnostic()?,
                Format::Markdownlint => {
                    writeln!(output, "{}", Markdownlint::new(&diagnostic)).into_diagnostic()?;
                }
            }
        }

        if num_diagnostics == 1 {
            writeln!(output, "\nFound 1 error.").into_diagnostic()?;
        } else {
            writeln!(output, "\nFound {num_diagnostics} errors.").into_diagnostic()?;
        }

        Ok(ExitCode::FAILURE)
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn options_to_config_none_none_false() {
        let options = Options {
            config_path: None,
            output_format: None,
            quiet: false,
        };
        let actual = options.to_config().unwrap();
        let mut expected = Config::default();
        expected.lint.md013.code_blocks = false;
        expected.lint.md013.tables = false;
        assert_eq!(actual, expected);
    }

    #[test]
    fn options_to_config_some_some_true() {
        let options = Options {
            config_path: Some(Path::new("mado.toml").to_path_buf()),
            output_format: Some(Format::Mdl),
            quiet: true,
        };
        let actual = options.to_config().unwrap();
        let mut expected = Config::default();
        expected.lint.output_format = Format::Mdl;
        expected.lint.quiet = true;
        expected.lint.md013.code_blocks = false;
        expected.lint.md013.tables = false;
        assert_eq!(actual, expected);
    }
}
