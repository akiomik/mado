use comrak::nodes::{AstNode, NodeValue};

pub trait Matcher {
    fn is_match<'a>(&self, node: &'a AstNode<'a>) -> bool;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeValueMatcher {
    pred: fn(&NodeValue) -> bool,
}

impl NodeValueMatcher {
    pub fn new(pred: fn(&NodeValue) -> bool) -> Self {
        Self { pred }
    }
}

impl Matcher for NodeValueMatcher {
    fn is_match<'a>(&self, node: &'a AstNode<'a>) -> bool {
        (self.pred)(&node.data.borrow().value)
    }
}
