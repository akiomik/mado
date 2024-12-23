use comrak::nodes::{AstNode, NodeHeading, NodeValue};
use miette::Result;

use crate::{violation::Violation, Document};

use super::{
    node::{NodeContext, NodeValueMatcher},
    NewRuleLike, NodeRule, RuleLike, RuleMetadata,
};

#[derive(Default, Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct State {
    pub maybe_prev_level: Option<u8>,
}

#[derive(Default, Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct MD001 {
    state: State,
}

impl MD001 {
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self {
            state: State::default(),
        }
    }
}

impl RuleLike for MD001 {
    #[inline]
    fn name(&self) -> &'static str {
        "MD001"
    }

    #[inline]
    fn description(&self) -> &'static str {
        "Header levels should only increment by one level at a time"
    }

    #[inline]
    fn tags(&self) -> Vec<&'static str> {
        vec!["headers"]
    }

    #[inline]
    fn aliases(&self) -> Vec<&'static str> {
        vec!["header-increment"]
    }

    #[inline]
    fn check(&self, doc: &Document) -> Result<Vec<Violation>> {
        let mut violations = vec![];
        let mut maybe_prev_level = None;

        for node in doc.ast.children() {
            if let NodeValue::Heading(heading) = node.data.borrow().value {
                if let Some(prev_level) = maybe_prev_level {
                    if heading.level > prev_level + 1 {
                        let position = node.data.borrow().sourcepos;
                        let violation = self.to_violation(doc.path.clone(), position);
                        violations.push(violation);
                    }
                }

                maybe_prev_level = Some(heading.level);
            }
        }

        Ok(violations)
    }
}

impl NewRuleLike for MD001 {
    fn metadata(&self) -> RuleMetadata {
        RuleMetadata {
            name: "MD001",
            description: "Header levels should only increment by one level at a time",
            tags: vec!["headers"],
            aliases: vec!["header-increment"],
        }
    }
}

impl NodeRule for MD001 {
    fn matcher(&self) -> NodeValueMatcher {
        NodeValueMatcher::new(|node| matches!(node, NodeValue::Heading(_)))
    }

    fn run<'a>(&mut self, ctx: &NodeContext, node: &'a AstNode<'a>) -> Result<Vec<Violation>> {
        let mut violations = vec![];

        if let NodeValue::Heading(NodeHeading { level, .. }) = node.data.borrow().value {
            if let Some(prev_level) = self.state.maybe_prev_level {
                if level > prev_level + 1 {
                    let position = node.data.borrow().sourcepos;
                    let violation = self.to_violation(ctx.path.clone(), position);
                    violations.push(violation);
                }
            }

            self.state.maybe_prev_level = Some(level);
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
        let text = "# Header 1

### Header 3

## Another Header 2

#### Header 4"
            .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let ast = parse_document(&arena, &text, &Options::default());
        let doc = Document {
            path: path.clone(),
            ast,
            text,
        };
        let rule = MD001::new();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![
            rule.to_violation(path.clone(), Sourcepos::from((3, 1, 3, 12))),
            rule.to_violation(path, Sourcepos::from((7, 1, 7, 13))),
        ];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_no_errors() {
        let text = "# Header 1

## Header 2

### Header 3

#### Header 4

## Another Header 2

### Another Header 3"
            .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let ast = parse_document(&arena, &text, &Options::default());
        let doc = Document { path, ast, text };
        let rule = MD001::new();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_no_errors_no_top_level() {
        let text = "## This isn't a H1 header".to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let ast = parse_document(&arena, &text, &Options::default());
        let doc = Document { path, ast, text };
        let rule = MD001::new();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![];
        assert_eq!(actual, expected);
    }
}
