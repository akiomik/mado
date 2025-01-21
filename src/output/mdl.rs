use core::cmp::Ordering;
use core::fmt::{Display, Error, Formatter, Result};

use colored::Colorize as _;

use crate::Diagnostic;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mdl<'a> {
    diagnostic: &'a Diagnostic,
}

impl<'a> Mdl<'a> {
    pub const fn new(diagnostic: &'a Diagnostic) -> Self {
        Self { diagnostic }
    }

    #[cfg(test)]
    pub const fn diagnostic(&self) -> &'a Diagnostic {
        self.diagnostic
    }
}

impl Display for Mdl<'_> {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self.diagnostic {
            Diagnostic::Violation(violation) => {
                let path = violation.path().to_str().ok_or(Error)?;
                write!(
                    f,
                    "{}{}{}{} {} {}",
                    path.bold(),
                    ":".blue(),
                    violation.position().start.line,
                    ":".blue(),
                    violation.name().red().bold(),
                    violation.description()
                )
            }
            Diagnostic::IoError(error) => {
                let path = error.path().to_str().ok_or(Error)?;
                write!(f, "{}{} {}", path.bold(), ":".blue(), error.message())
            }
            Diagnostic::LintError(error) => {
                let path = error.path().to_str().ok_or(Error)?;
                write!(f, "{}{} {}", path.bold(), ":".blue(), error.message())
            }
        }
    }
}

impl PartialOrd for Mdl<'_> {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Mdl<'_> {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        let path_cmp = self.diagnostic.path().cmp(other.diagnostic.path());
        if path_cmp != Ordering::Equal {
            return path_cmp;
        }

        if let Diagnostic::Violation(violation) = self.diagnostic {
            if let Diagnostic::Violation(other_violation) = other.diagnostic {
                let name_cmp = violation.name().cmp(other_violation.name());
                if name_cmp != Ordering::Equal {
                    return name_cmp;
                }

                return violation
                    .position()
                    .start
                    .cmp(&other_violation.position().start);
            }
        }

        self.diagnostic.cmp(other.diagnostic)
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use comrak::nodes::Sourcepos;
    use pretty_assertions::assert_eq;

    use crate::{rule::Metadata, Violation};

    use super::*;

    const METADATA: Metadata = Metadata {
        name: "name",
        description: "description",
        aliases: &["alias"],
        tags: &["tags"],
    };

    #[test]
    fn display_fmt() {
        let path = Path::new("file.md").to_path_buf();
        let position = Sourcepos::from((0, 1, 3, 5));
        let violation = Violation::new(path, &METADATA, position);
        let diagnostic = Diagnostic::Violation(violation);
        let actual = Mdl::new(&diagnostic).to_string();
        let expected = "\u{1b}[1mfile.md\u{1b}[0m\u{1b}[34m:\u{1b}[0m0\u{1b}[34m:\u{1b}[0m \u{1b}[1;31mname\u{1b}[0m description";
        assert_eq!(actual, expected);
    }
}
