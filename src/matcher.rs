use comrak::nodes::{AstNode, NodeValue};

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
