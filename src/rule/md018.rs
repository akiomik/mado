use comrak::nodes::{AstNode, NodeValue};
use miette::Result;

use crate::violation::Violation;
use crate::Document;

use super::{
    node::{NodeContext, NodeRule, NodeValueMatcher},
    NewRuleLike, RuleLike, RuleMetadata,
};

#[derive(Default, Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct MD018;

impl MD018 {
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self {}
    }
}

impl RuleLike for MD018 {
    #[inline]
    fn name(&self) -> &'static str {
        "MD018"
    }

    #[inline]
    fn description(&self) -> &'static str {
        "No space after hash on atx style header"
    }

    #[inline]
    fn tags(&self) -> Vec<&'static str> {
        vec!["headers", "atx", "spaces"]
    }

    #[inline]
    fn aliases(&self) -> Vec<&'static str> {
        vec!["no-missing-space-atx"]
    }

    #[inline]
    fn check(&self, doc: &Document) -> Result<Vec<Violation>> {
        let mut violations = vec![];

        for node in doc.ast.children() {
            if let NodeValue::Paragraph = node.data.borrow().value {
                for child_node in node.children() {
                    if let NodeValue::Text(text) = &child_node.data.borrow().value {
                        let position = node.data.borrow().sourcepos;
                        if position.start.column == 1 && text.starts_with('#') {
                            let violation = self.to_violation(doc.path.clone(), position);
                            violations.push(violation);
                        }
                    }
                }
            }
        }

        Ok(violations)
    }
}

impl NewRuleLike for MD018 {
    fn metadata(&self) -> RuleMetadata {
        RuleMetadata {
            name: "MD018",
            description: "No space after hash on atx style header",
            tags: vec!["headers", "atx", "spaces"],
            aliases: vec!["no-missing-space-atx"],
        }
    }
}

impl NodeRule for MD018 {
    fn matcher(&self) -> NodeValueMatcher {
        NodeValueMatcher::new(|node| {
            matches!(
                node,
                NodeValue::Text(text)
                if text.starts_with('#')
            )
        })
    }

    fn run<'a>(&mut self, ctx: &NodeContext, node: &'a AstNode<'a>) -> Result<Vec<Violation>> {
        let mut violations = vec![];

        if let Some(parent_node) = node.parent() {
            if let NodeValue::Paragraph = &parent_node.data.borrow().value {
                if let NodeValue::Text(_) = &node.data.borrow().value {
                    let position = node.data.borrow().sourcepos;
                    if position.start.column == 1 {
                        let violation = self.to_violation(ctx.path.clone(), position);
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
        let text = "#Header 1

##Header 2"
            .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let ast = parse_document(&arena, &text, &Options::default());
        let doc = Document {
            path: path.clone(),
            ast,
            text,
        };
        let rule = MD018::default();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![
            rule.to_violation(path.clone(), Sourcepos::from((1, 1, 1, 9))),
            rule.to_violation(path, Sourcepos::from((3, 1, 3, 10))),
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
        let rule = MD018::default();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_no_errors_with_issue_number() {
        let text = "# Header 1

See [#4649](https://example.com) for details."
            .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let ast = parse_document(&arena, &text, &Options::default());
        let doc = Document { path, ast, text };
        let rule = MD018::default();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_no_errors_with_list() {
        let text = "* # Header 1".to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let ast = parse_document(&arena, &text, &Options::default());
        let doc = Document { path, ast, text };
        let rule = MD018::default();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![];
        assert_eq!(actual, expected);
    }
}
