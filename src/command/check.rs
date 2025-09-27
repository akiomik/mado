use std::io::Read as _;
use std::io::{self, BufWriter, IsTerminal as _, Write as _};
use std::path::PathBuf;
use std::process::ExitCode;

use globset::Glob;
use miette::IntoDiagnostic as _;
use miette::Result;

use crate::output::{Concise, Format, Markdownlint, Mdl};
use crate::service::runner::{LintRunner, ParallelLintRunner, StringLintRunner};
use crate::Config;

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::exhaustive_structs)]
pub struct Options {
    pub config_path: Option<PathBuf>,
    pub output_format: Option<Format>,
    pub quiet: bool,
    pub exclude: Option<Vec<Glob>>,
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

        if let Some(exclude) = self.exclude {
            config.lint.exclude = exclude;
        }

        Ok(config)
    }
}

pub struct Checker {
    runner: LintRunner,
    config: Config,
}

fn stdin_input() -> Option<String> {
    let stdin = io::stdin();
    if stdin.is_terminal() {
        return None;
    }

    let mut buffer = String::new();
    io::stdin().lock().read_to_string(&mut buffer).ok()?;

    if buffer.is_empty() {
        None
    } else {
        Some(buffer)
    }
}

impl Checker {
    #[inline]
    pub fn new(patterns: &[PathBuf], config: Config) -> Result<Self> {
        let runner = match stdin_input() {
            Some(input) => {
                LintRunner::String(Box::new(StringLintRunner::new(input, config.clone())))
            }
            None => LintRunner::Parallel(Box::new(ParallelLintRunner::new(
                patterns,
                config.clone(),
                100,
            )?)),
        };

        Ok(Self { runner, config })
    }

    #[inline]
    pub fn check(self) -> Result<ExitCode> {
        let mut violations = self.runner.run()?;
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
    fn options_to_config_none_none_false_none() -> Result<()> {
        let options = Options {
            config_path: None,
            output_format: None,
            quiet: false,
            exclude: None,
        };
        let actual = options.to_config()?;
        let mut expected = Config::default();
        expected.lint.md013.code_blocks = false;
        expected.lint.md013.tables = false;
        expected.lint.md024.allow_different_nesting = true;
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn options_to_config_some_some_true_some() -> Result<()> {
        let exclude = vec![Glob::new("README.md").into_diagnostic()?];
        let options = Options {
            config_path: Some(Path::new("mado.toml").to_path_buf()),
            output_format: Some(Format::Mdl),
            quiet: true,
            exclude: Some(exclude.clone()),
        };
        let actual = options.to_config()?;
        let mut expected = Config::default();
        expected.lint.output_format = Format::Mdl;
        expected.lint.quiet = true;
        expected.lint.exclude = exclude;
        expected.lint.md013.code_blocks = false;
        expected.lint.md013.tables = false;
        expected.lint.md024.allow_different_nesting = true;
        assert_eq!(actual, expected);
        Ok(())
    }
}
