use comrak::nodes::{AstNode, NodeHeading, NodeValue};
use miette::Result;

use crate::violation::Violation;
use crate::Document;

use super::{
    node::{NodeContext, NodeRule, NodeValueMatcher},
    NewRuleLike, RuleLike, RuleMetadata,
};

#[derive(Default, Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct MD019;

impl MD019 {
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self {}
    }
}

impl RuleLike for MD019 {
    #[inline]
    fn name(&self) -> &'static str {
        "MD019"
    }

    #[inline]
    fn description(&self) -> &'static str {
        "Multiple spaces after hash on atx style header"
    }

    #[inline]
    fn tags(&self) -> Vec<&'static str> {
        vec!["headers", "atx", "spaces"]
    }

    #[inline]
    fn aliases(&self) -> Vec<&'static str> {
        vec!["no-multiple-space-atx"]
    }

    #[inline]
    fn check(&self, doc: &Document) -> Result<Vec<Violation>> {
        let mut violations = vec![];

        for node in doc.ast.children() {
            if let NodeValue::Heading(NodeHeading {
                setext: false,
                level,
                ..
            }) = &node.data.borrow().value
            {
                if let Some(text_node) = node.first_child() {
                    if let NodeValue::Text(_) = text_node.data.borrow().value {
                        let heading_position = node.data.borrow().sourcepos;
                        let text_position = text_node.data.borrow().sourcepos;
                        let expected_text_offset =
                            heading_position.start.column + (*level as usize) + 1;
                        if text_position.start.column > expected_text_offset {
                            let violation = self.to_violation(doc.path.clone(), heading_position);
                            violations.push(violation);
                        }
                    }
                }
            }
        }

        Ok(violations)
    }
}

impl NewRuleLike for MD019 {
    fn metadata(&self) -> RuleMetadata {
        RuleMetadata {
            name: "MD019",
            description: "Multiple spaces after hash on atx style header",
            tags: vec!["headers", "atx", "spaces"],
            aliases: vec!["no-multiple-space-atx"],
        }
    }
}

impl NodeRule for MD019 {
    fn matcher(&self) -> NodeValueMatcher {
        NodeValueMatcher::new(|node| {
            matches!(node, NodeValue::Heading(NodeHeading { setext: false, .. }))
        })
    }

    fn run<'a>(&mut self, ctx: &NodeContext, node: &'a AstNode<'a>) -> Result<Vec<Violation>> {
        let mut violations = vec![];

        if let Some(text_node) = node.first_child() {
            if let NodeValue::Heading(NodeHeading { level, .. }) = &node.data.borrow().value {
                if let NodeValue::Text(_) = text_node.data.borrow().value {
                    let heading_position = node.data.borrow().sourcepos;
                    let text_position = text_node.data.borrow().sourcepos;
                    let expected_text_offset =
                        heading_position.start.column + (*level as usize) + 1;
                    if text_position.start.column > expected_text_offset {
                        let violation = self.to_violation(ctx.path.clone(), heading_position);
                        violations.push(violation);
                    }
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
        let text = "#  Header 1

##  Header 2"
            .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let ast = parse_document(&arena, &text, &Options::default());
        let doc = Document {
            path: path.clone(),
            ast,
            text,
        };
        let rule = MD019::new();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![
            rule.to_violation(path.clone(), Sourcepos::from((1, 1, 1, 11))),
            rule.to_violation(path, Sourcepos::from((3, 1, 3, 12))),
        ];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_no_errors() {
        let text = "# Header 1

## Header 2"
            .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let ast = parse_document(&arena, &text, &Options::default());
        let doc = Document { path, ast, text };
        let rule = MD019::new();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_no_errors_with_setext() {
        let text = "  Header 1
========"
            .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let ast = parse_document(&arena, &text, &Options::default());
        let doc = Document { path, ast, text };
        let rule = MD019::new();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_no_errors_for_multiple_children() {
        let text = "# Header with `code` and text".to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let ast = parse_document(&arena, &text, &Options::default());
        let doc = Document {
            path: path.clone(),
            ast,
            text,
        };
        let rule = MD019::new();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![];
        assert_eq!(actual, expected);
    }
}
