use comrak::nodes::{AstNode, NodeValue};
use miette::Result;

use crate::{violation::Violation, Document};

use super::{
    node::{NodeContext, NodeRule, NodeValueMatcher},
    NewRuleLike, RuleLike, RuleMetadata,
};

#[derive(Default, Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct MD023;

impl MD023 {
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self {}
    }
}

impl RuleLike for MD023 {
    #[inline]
    fn name(&self) -> &'static str {
        "MD023"
    }

    #[inline]
    fn description(&self) -> &'static str {
        "Headers must start at the beginning of the line"
    }

    #[inline]
    fn tags(&self) -> Vec<&'static str> {
        vec!["headers", "spaces"]
    }

    #[inline]
    fn aliases(&self) -> Vec<&'static str> {
        vec!["header-start-left"]
    }

    #[inline]
    fn check(&self, doc: &Document) -> Result<Vec<Violation>> {
        let mut violations = vec![];

        for node in doc.ast.children() {
            if let NodeValue::Heading(_) = node.data.borrow().value {
                let position = node.data.borrow().sourcepos;
                if position.start.column > 1 {
                    let violation = self.to_violation(doc.path.clone(), position);
                    violations.push(violation);
                }
            }
        }

        Ok(violations)
    }
}

impl NewRuleLike for MD023 {
    fn metadata(&self) -> RuleMetadata {
        RuleMetadata {
            name: "MD023",
            description: "Headers must start at the beginning of the line",
            tags: vec!["headers", "spaces"],
            aliases: vec!["header-start-left"],
        }
    }

    fn reset(&mut self) {}
}

impl NodeRule for MD023 {
    fn matcher(&self) -> NodeValueMatcher {
        NodeValueMatcher::new(|node| matches!(node, NodeValue::Heading(_)))
    }

    fn run<'a>(&mut self, ctx: &NodeContext, node: &'a AstNode<'a>) -> Result<Vec<Violation>> {
        let mut violations = vec![];

        let position = node.data.borrow().sourcepos;
        if position.start.column > 1 {
            let violation = self.to_violation(ctx.path.clone(), position);
            violations.push(violation);
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
        let text = "Some text

  # Indented header"
            .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let ast = parse_document(&arena, &text, &Options::default());
        let doc = Document {
            path: path.clone(),
            ast,
            text,
        };
        let rule = MD023::new();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![rule.to_violation(path.clone(), Sourcepos::from((3, 3, 3, 19)))];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_no_errors() {
        let text = "Some text

# Header"
            .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let ast = parse_document(&arena, &text, &Options::default());
        let doc = Document { path, ast, text };
        let rule = MD023::new();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![];
        assert_eq!(actual, expected);
    }
}
