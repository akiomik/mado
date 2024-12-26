use comrak::nodes::{AstNode, NodeHeading, NodeValue};
use miette::Result;

use crate::{violation::Violation, Document};

use super::{
    node::{NodeContext, NodeRule, NodeValueMatcher},
    NewRuleLike, RuleLike, RuleMetadata,
};

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct State {
    top_level_header_seen: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct MD025 {
    level: u8,
    state: State,
}

impl MD025 {
    pub const DEFAULT_LEVEL: u8 = 1;

    #[inline]
    #[must_use]
    pub fn new(level: u8) -> Self {
        Self {
            level,
            state: State::default(),
        }
    }
}

impl Default for MD025 {
    #[inline]
    fn default() -> Self {
        Self {
            level: Self::DEFAULT_LEVEL,
            state: State::default(),
        }
    }
}

impl RuleLike for MD025 {
    #[inline]
    fn name(&self) -> &'static str {
        "MD025"
    }

    #[inline]
    fn description(&self) -> &'static str {
        "Multiple top level headers in the same document"
    }

    #[inline]
    fn tags(&self) -> Vec<&'static str> {
        vec!["headers"]
    }

    #[inline]
    fn aliases(&self) -> Vec<&'static str> {
        vec!["single-h1"]
    }

    #[inline]
    fn check(&self, doc: &Document) -> Result<Vec<Violation>> {
        let mut violations = vec![];
        let mut seen_top_level_header = false;

        for node in doc.ast.children() {
            if let NodeValue::Heading(heading) = node.data.borrow().value {
                if heading.level == self.level {
                    if seen_top_level_header {
                        let position = node.data.borrow().sourcepos;
                        let violation = self.to_violation(doc.path.clone(), position);
                        violations.push(violation);
                    } else {
                        seen_top_level_header = true;
                    }
                }
            }
        }

        Ok(violations)
    }
}

impl NewRuleLike for MD025 {
    #[inline]
    fn metadata(&self) -> RuleMetadata {
        RuleMetadata {
            name: "MD025",
            description: "Multiple top level headers in the same document",
            tags: vec!["headers"],
            aliases: vec!["single-h1"],
        }
    }

    #[inline]
    fn reset(&mut self) {
        self.state = State::default();
    }
}

impl NodeRule for MD025 {
    #[inline]
    fn matcher(&self) -> NodeValueMatcher {
        NodeValueMatcher::new(|node| matches!(node, NodeValue::Heading(_)))
    }

    #[inline]
    fn run<'a>(&mut self, ctx: &NodeContext, node: &'a AstNode<'a>) -> Result<Vec<Violation>> {
        let mut violations = vec![];

        if let NodeValue::Heading(NodeHeading { level, .. }) = node.data.borrow().value {
            if level == self.level {
                if self.state.top_level_header_seen {
                    let position = node.data.borrow().sourcepos;
                    let violation = self.to_violation(ctx.path.clone(), position);
                    violations.push(violation);
                } else {
                    self.state.top_level_header_seen = true;
                }
            }
        }

        Ok(violations)
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use comrak::{nodes::Sourcepos, parse_document, Arena, Options};
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn check_errors() {
        let text = "# Top level header

# Another top level header"
            .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let ast = parse_document(&arena, &text, &Options::default());
        let doc = Document {
            path: path.clone(),
            ast,
            text,
        };
        let rule = MD025::default();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![rule.to_violation(path.clone(), Sourcepos::from((3, 1, 3, 26)))];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_errors_with_level() {
        let text = "## Top level header

## Another top level header"
            .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let ast = parse_document(&arena, &text, &Options::default());
        let doc = Document {
            path: path.clone(),
            ast,
            text,
        };
        let rule = MD025::new(2);
        let actual = rule.check(&doc).unwrap();
        let expected = vec![rule.to_violation(path.clone(), Sourcepos::from((3, 1, 3, 27)))];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_no_errors() {
        let text = "# Title

## Header

## Another header"
            .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let ast = parse_document(&arena, &text, &Options::default());
        let doc = Document { path, ast, text };
        let rule = MD025::default();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_no_errors_with_level() {
        let text = "## Title

### Header

### Another header"
            .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let ast = parse_document(&arena, &text, &Options::default());
        let doc = Document { path, ast, text };
        let rule = MD025::new(2);
        let actual = rule.check(&doc).unwrap();
        let expected = vec![];
        assert_eq!(actual, expected);
    }
}
