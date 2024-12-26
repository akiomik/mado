use std::path::PathBuf;

use miette::Result;

use crate::Violation;

use super::NewRuleLike;

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct LineMatcher {
    pred: fn(&str) -> bool,
}

impl LineMatcher {
    #[inline]
    #[must_use]
    pub fn new(pred: fn(&str) -> bool) -> Self {
        Self { pred }
    }

    #[inline]
    #[must_use]
    pub fn is_match(&self, line: &str) -> bool {
        (self.pred)(line)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::exhaustive_structs)]
pub struct LineContext {
    pub path: PathBuf,
    pub lineno: usize,
}

pub trait LineRule: NewRuleLike {
    #[must_use]
    fn matcher(&self) -> LineMatcher;

    fn run(&self, ctx: &LineContext, line: &str) -> Result<Vec<Violation>>;
}
