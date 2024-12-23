use std::path::PathBuf;

use comrak::nodes::{AstNode, NodeValue};
use miette::Result;

use crate::Violation;

use super::NewRuleLike;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeValueMatcher {
    pred: fn(&NodeValue) -> bool,
}

impl NodeValueMatcher {
    #[inline]
    pub fn new(pred: fn(&NodeValue) -> bool) -> Self {
        Self { pred }
    }

    #[inline]
    pub fn is_match<'a>(&self, node: &'a AstNode<'a>) -> bool {
        (self.pred)(&node.data.borrow().value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeContext {
    pub path: PathBuf,
    pub level: usize,
    pub list_level: Option<usize>,
}

pub trait NodeRule: NewRuleLike {
    fn matcher(&self) -> NodeValueMatcher;

    fn run<'a>(&mut self, ctx: &NodeContext, node: &'a AstNode<'a>) -> Result<Vec<Violation>>;
}
