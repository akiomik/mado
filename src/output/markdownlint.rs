use core::cmp::Ordering;
use core::fmt::{Display, Error, Formatter, Result};

use colored::Colorize as _;

use crate::Diagnostic;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Markdownlint<'a> {
    diagnostic: &'a Diagnostic,
}

impl<'a> Markdownlint<'a> {
    pub const fn new(diagnostic: &'a Diagnostic) -> Self {
        Self { diagnostic }
    }

    #[cfg(test)]
    pub const fn diagnostic(&self) -> &'a Diagnostic {
        self.diagnostic
    }
}

impl Display for Markdownlint<'_> {
    // TODO: Add `expected` and `actual`
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self.diagnostic {
            Diagnostic::Violation(violation) => {
                let path = violation.path().to_str().ok_or(Error)?;
                write!(
                    f,
                    "{}",
                    format!(
                        "{}:{}:{} {}/{} {}",
                        path,
                        violation.position().start.line,
                        violation.position().start.column,
                        violation.name(),
                        violation.alias(),
                        violation.description()
                    )
                    .red()
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

impl PartialOrd for Markdownlint<'_> {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Markdownlint<'_> {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
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
        let actual = Markdownlint::new(&diagnostic).to_string();
        let expected = "\u{1b}[31mfile.md:0:1 name/alias description\u{1b}[0m";
        assert_eq!(actual, expected);
    }
}
