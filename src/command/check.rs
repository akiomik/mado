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

        config.lint.quiet = self.quiet;

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
        let mut violations = runner.run()?;
        violations.sort_by(self.config.lint.output_format.sorter());

        if violations.is_empty() {
            if !self.config.lint.quiet {
                println!("All checks passed!");
            }

            return Ok(ExitCode::SUCCESS);
        }

        let mut output = BufWriter::new(io::stdout().lock());
        let num_violations = violations.len();
        for violation in violations {
            match self.config.lint.output_format {
                Format::Concise => {
                    writeln!(output, "{}", Concise::new(&violation)).into_diagnostic()?;
                }
                Format::Mdl => writeln!(output, "{}", Mdl::new(&violation)).into_diagnostic()?,
                Format::Markdownlint => {
                    writeln!(output, "{}", Markdownlint::new(&violation)).into_diagnostic()?;
                }
            }
        }

        if num_violations == 1 {
            writeln!(output, "\nFound 1 error.").into_diagnostic()?;
        } else {
            writeln!(output, "\nFound {num_violations} errors.").into_diagnostic()?;
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
    fn options_to_config_none_none() {
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
    fn options_to_config_none_some() {
        let options = Options {
            config_path: None,
            output_format: Some(Format::Mdl),
            quiet: false,
        };
        let actual = options.to_config().unwrap();
        let mut expected = Config::default();
        expected.lint.output_format = Format::Mdl;
        expected.lint.md013.code_blocks = false;
        expected.lint.md013.tables = false;
        assert_eq!(actual, expected);
    }

    #[test]
    fn options_to_config_some_none() {
        let options = Options {
            config_path: Some(Path::new("mado.toml").to_path_buf()),
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
    fn options_to_config_some_some() {
        let options = Options {
            config_path: Some(Path::new("mado.toml").to_path_buf()),
            output_format: Some(Format::Mdl),
            quiet: false,
        };
        let actual = options.to_config().unwrap();
        let mut expected = Config::default();
        expected.lint.output_format = Format::Mdl;
        expected.lint.md013.code_blocks = false;
        expected.lint.md013.tables = false;
        assert_eq!(actual, expected);
    }
}
