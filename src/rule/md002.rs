use comrak::nodes::{AstNode, NodeValue};
use miette::Result;

use crate::violation::Violation;
use crate::Document;

use super::{
    node::{NodeContext, NodeRule, NodeValueMatcher},
    NewRuleLike, RuleLike, RuleMetadata,
};

#[derive(Debug, Default, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct State {
    header_seen: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct MD002 {
    pub level: u8,
    pub state: State,
}

impl MD002 {
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

impl Default for MD002 {
    #[inline]
    fn default() -> Self {
        Self {
            level: Self::DEFAULT_LEVEL,
            state: State::default(),
        }
    }
}

impl RuleLike for MD002 {
    #[inline]
    fn name(&self) -> &'static str {
        "MD002"
    }

    #[inline]
    fn description(&self) -> &'static str {
        "First header should be a top level header"
    }

    #[inline]
    fn tags(&self) -> Vec<&'static str> {
        vec!["headers"]
    }

    #[inline]
    fn aliases(&self) -> Vec<&'static str> {
        vec!["first-header-h1"]
    }

    #[inline]
    fn check(&self, doc: &Document) -> Result<Vec<Violation>> {
        for node in doc.ast.children() {
            if let NodeValue::Heading(heading) = node.data.borrow().value {
                if heading.level != self.level {
                    let position = node.data.borrow().sourcepos;
                    let violation = self.to_violation(doc.path.clone(), position);

                    return Ok(vec![violation]);
                }

                break;
            }
        }

        Ok(vec![])
    }
}

impl NewRuleLike for MD002 {
    fn metadata(&self) -> RuleMetadata {
        RuleMetadata {
            name: "MD002",
            description: "First header should be a top level header",
            tags: vec!["headers"],
            aliases: vec!["first-header-h1"],
        }
    }

    fn reset(&mut self) {
        self.state = State::default();
    }
}

impl NodeRule for MD002 {
    fn matcher(&self) -> NodeValueMatcher {
        NodeValueMatcher::new(|node| matches!(node, NodeValue::Heading(_)))
    }

    fn run<'a>(&mut self, ctx: &NodeContext, node: &'a AstNode<'a>) -> Result<Vec<Violation>> {
        let mut violations = vec![];
        if let NodeValue::Heading(heading) = node.data.borrow().value {
            if !self.state.header_seen && heading.level != self.level {
                let position = node.data.borrow().sourcepos;
                let violation = self.to_violation(ctx.path.clone(), position);
                violations.push(violation);
            }

            self.state.header_seen = true;
        }

        return Ok(violations);
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
        let text = "## This isn't a H1 header

### Another header"
            .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let ast = parse_document(&arena, &text, &Options::default());
        let doc = Document {
            path: path.clone(),
            ast,
            text,
        };
        let rule = MD002::default();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![rule.to_violation(path, Sourcepos::from((1, 1, 1, 25)))];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_errors_with_level() {
        let text = "# Start with a H1 header

## Then use a H2 for subsections"
            .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let ast = parse_document(&arena, &text, &Options::default());
        let doc = Document {
            path: path.clone(),
            ast,
            text,
        };
        let rule = MD002::new(2);
        let actual = rule.check(&doc).unwrap();
        let expected = vec![rule.to_violation(path, Sourcepos::from((1, 1, 1, 24)))];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_no_errors() {
        let text = "# Start with a H1 header

## Then use a H2 for subsections"
            .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let ast = parse_document(&arena, &text, &Options::default());
        let doc = Document { path, ast, text };
        let rule = MD002::default();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_no_errors_with_level() {
        let text = "## This isn't a H1 header

### Another header"
            .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let ast = parse_document(&arena, &text, &Options::default());
        let doc = Document { path, ast, text };
        let rule = MD002::new(2);
        let actual = rule.check(&doc).unwrap();
        let expected = vec![];
        assert_eq!(actual, expected);
    }
}
