use std::path::PathBuf;

use miette::Result;

use crate::Violation;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineMatcher {
    pred: fn(&str) -> bool,
}

impl LineMatcher {
    #[inline]
    pub fn new(pred: fn(&str) -> bool) -> Self {
        Self { pred }
    }

    #[inline]
    pub fn is_match<'a>(&self, line: &str) -> bool {
        (self.pred)(line)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineContext {
    pub path: PathBuf,
    pub lineno: usize,
}

pub trait LineRule: Send {
    fn matcher(&self) -> LineMatcher;

    fn run<'a>(&self, ctx: &LineContext, line: &str) -> Result<Vec<Violation>>;
}
