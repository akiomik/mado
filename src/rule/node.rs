use std::path::PathBuf;

use comrak::nodes::{AstNode, NodeValue};
use miette::Result;

use crate::Violation;

use super::NewRuleLike;

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct NodeValueMatcher {
    pred: fn(&NodeValue) -> bool,
}

impl NodeValueMatcher {
    #[inline]
    #[must_use]
    pub fn new(pred: fn(&NodeValue) -> bool) -> Self {
        Self { pred }
    }

    #[inline]
    #[must_use]
    pub fn is_match<'a>(&self, node: &'a AstNode<'a>) -> bool {
        (self.pred)(&node.data.borrow().value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::exhaustive_structs)]
pub struct NodeContext {
    pub path: PathBuf,
    pub level: usize,
    pub list_level: Option<usize>,
}

pub trait NodeRule: NewRuleLike {
    #[must_use]
    fn matcher(&self) -> NodeValueMatcher;

    fn run<'a>(&mut self, ctx: &NodeContext, node: &'a AstNode<'a>) -> Result<Vec<Violation>>;
}
