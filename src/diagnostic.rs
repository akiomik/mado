use core::cmp::Ordering;
use std::path::PathBuf;

use crate::Violation;

#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Diagnostic {
    Violation(Violation),
    IoError(IoError),
    LintError(LintError),
}

impl Diagnostic {
    #[inline]
    #[must_use]
    pub const fn path(&self) -> &PathBuf {
        match self {
            Self::Violation(violation) => violation.path(),
            Self::IoError(error) => error.path(),
            Self::LintError(error) => error.path(),
        }
    }
}

impl PartialOrd for Diagnostic {
    #[inline]
    #[must_use]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Diagnostic {
    #[inline]
    #[must_use]
    fn cmp(&self, other: &Self) -> Ordering {
        let path_cmp = self.path().cmp(other.path());
        if path_cmp != Ordering::Equal {
            return path_cmp;
        }

        match (self, other) {
            (Self::Violation(violation), Self::Violation(other_violation)) => {
                violation.cmp(other_violation)
            }
            (Self::IoError(error), Self::IoError(other_error)) => error.cmp(other_error),
            (Self::LintError(error), Self::LintError(other_error)) => error.cmp(other_error),
            (Self::IoError(_) | Self::LintError(_), Self::Violation(_)) => Ordering::Less,
            (Self::Violation(_), Self::IoError(_) | Self::LintError(_)) => Ordering::Greater,
            (Self::IoError(error), Self::LintError(other_error)) => {
                error.message().cmp(&other_error.message)
            }
            (Self::LintError(error), Self::IoError(other_error)) => {
                error.message().cmp(&other_error.message)
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IoError {
    path: PathBuf,
    message: String,
}

impl IoError {
    pub const fn new(path: PathBuf, message: String) -> Self {
        Self { path, message }
    }

    pub const fn path(&self) -> &PathBuf {
        &self.path
    }

    pub const fn message(&self) -> &String {
        &self.message
    }
}

impl PartialOrd for IoError {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for IoError {
    #[inline]
    #[must_use]
    fn cmp(&self, other: &Self) -> Ordering {
        let path_cmp = self.path.cmp(&other.path);
        if path_cmp != Ordering::Equal {
            return path_cmp;
        }

        self.message.cmp(&other.message)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LintError {
    path: PathBuf,
    message: String,
}

impl LintError {
    pub const fn new(path: PathBuf, message: String) -> Self {
        Self { path, message }
    }

    pub const fn path(&self) -> &PathBuf {
        &self.path
    }

    pub const fn message(&self) -> &String {
        &self.message
    }
}

impl PartialOrd for LintError {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for LintError {
    #[inline]
    #[must_use]
    fn cmp(&self, other: &Self) -> Ordering {
        let path_cmp = self.path.cmp(&other.path);
        if path_cmp != Ordering::Equal {
            return path_cmp;
        }

        self.message.cmp(&other.message)
    }
}
