use comrak::nodes::{AstNode, ListType, NodeList, NodeValue};
use miette::Result;

use crate::{violation::Violation, Document};

use super::{
    node::{NodeContext, NodeRule, NodeValueMatcher},
    NewRuleLike, RuleLike, RuleMetadata,
};

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct MD007 {
    indent: usize,
}

impl MD007 {
    pub const DEFAULT_INDENT: usize = 4;

    #[inline]
    #[must_use]
    pub fn new(indent: usize) -> Self {
        Self { indent }
    }
}

impl Default for MD007 {
    #[inline]
    fn default() -> Self {
        Self {
            indent: Self::DEFAULT_INDENT,
        }
    }
}

impl RuleLike for MD007 {
    #[inline]
    fn name(&self) -> &'static str {
        "MD007"
    }

    #[inline]
    fn description(&self) -> &'static str {
        "Unordered list indentation"
    }

    #[inline]
    fn tags(&self) -> Vec<&'static str> {
        vec!["bullet", "ul", "indentation"]
    }

    #[inline]
    fn aliases(&self) -> Vec<&'static str> {
        vec!["ul-indent"]
    }

    #[inline]
    fn check(&self, doc: &Document) -> Result<Vec<Violation>> {
        let mut violations = vec![];
        let mut maybe_prev_indent = None;

        for node in doc.ast.descendants() {
            if let NodeValue::Item(item) = node.data.borrow().value {
                let position = node.data.borrow().sourcepos;
                let indent = position.start.column - 1;

                if item.list_type == ListType::Bullet {
                    let level_indent = match maybe_prev_indent {
                        Some(prev_indent) if indent > prev_indent => indent - prev_indent,
                        _ => indent,
                    };

                    if level_indent != 0 && level_indent != self.indent {
                        let violation = self.to_violation(doc.path.clone(), position);
                        violations.push(violation);
                    }
                }

                maybe_prev_indent = Some(indent);
            }
        }

        Ok(violations)
    }
}

impl NewRuleLike for MD007 {
    fn metadata(&self) -> RuleMetadata {
        RuleMetadata {
            name: "MD007",
            description: "Unordered list indentation",
            tags: vec!["bullet", "ul", "indentation"],
            aliases: vec!["ul-indent"],
        }
    }
}

impl NodeRule for MD007 {
    fn matcher(&self) -> NodeValueMatcher {
        NodeValueMatcher::new(|node| {
            matches!(
                node,
                NodeValue::Item(NodeList { list_type, .. }) if *list_type == ListType::Bullet
            )
        })
    }

    fn run<'a>(&mut self, ctx: &NodeContext, node: &'a AstNode<'a>) -> Result<Vec<Violation>> {
        let mut violations = vec![];

        if let NodeValue::Item(_) = node.data.borrow().value {
            let position = node.data.borrow().sourcepos;
            let indent = position.start.column - 1;

            if indent != 0 && indent != self.indent {
                let violation = self.to_violation(ctx.path.clone(), position);
                violations.push(violation);
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
        let text = "* List item
   * Nested list item indented by 3 spaces
       * More nested list item indented by 4 spaces
* List item
   * Nested list item indented by 3 spaces
       * More nested list item indented by 4 spaces"
            .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let ast = parse_document(&arena, &text, &Options::default());
        let doc = Document {
            path: path.clone(),
            ast,
            text,
        };
        let rule = MD007::default();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![
            rule.to_violation(path.clone(), Sourcepos::from((2, 4, 3, 51))),
            rule.to_violation(path.clone(), Sourcepos::from((5, 4, 6, 51))),
        ];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_errors_for_multiple_indentation() {
        let text = "* List item
    * Nested list item indented by 4 spaces
        * More nested list item indented by 4 spaces
* List item
    * Nested list item indented by 4 spaces
        * More nested list item indented by 4 spaces"
            .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let ast = parse_document(&arena, &text, &Options::default());
        let doc = Document {
            path: path.clone(),
            ast,
            text,
        };
        let rule = MD007::new(2);
        let actual = rule.check(&doc).unwrap();
        let expected = vec![
            rule.to_violation(path.clone(), Sourcepos::from((2, 5, 3, 52))),
            rule.to_violation(path.clone(), Sourcepos::from((3, 9, 3, 52))),
            rule.to_violation(path.clone(), Sourcepos::from((5, 5, 6, 52))),
            rule.to_violation(path, Sourcepos::from((6, 9, 6, 52))),
        ];
        assert_eq!(actual, expected);
    }

    // TODO: This should be passed
    //     #[test]
    //     fn check_errors_with_ol() {
    //         let text = "* List item
    //    1. Nested list item indented by 3 spaces
    //        * More nested list item indented by 4 spaces
    // * List item
    //    1. Nested list item indented by 3 spaces
    //        * More nested list item indented by 4 spaces"
    //             .to_owned();
    //         let path = Path::new("test.md").to_path_buf();
    //         let arena = Arena::new();
    //         let ast = parse_document(&arena, &text, &Options::default());
    //         let doc = Document {
    //             path: path.clone(),
    //             ast,
    //             text,
    //         };
    //         let rule = MD007::default();
    //         let actual = rule.check(&doc).unwrap();
    //         let expected = vec![
    //             rule.to_violation(path.clone(), Sourcepos::from((3, 8, 3, 51))),
    //             rule.to_violation(path, Sourcepos::from((6, 8, 6, 51))),
    //         ];
    //         assert_eq!(actual, expected);
    //     }

    #[test]
    fn check_no_errors() {
        let text = "* List item
    * Nested list item indented by 4 spaces
        * More nested list item indented by 4 spaces
* List Item
    * Nested list item indented by 4 spaces
        * More nested list item indented by 4 spaces"
            .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let ast = parse_document(&arena, &text, &Options::default());
        let doc = Document { path, ast, text };
        let rule = MD007::default();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_no_errors_ol() {
        let text = "* List item
   1. Nested list item indented by 3 spaces
* List Item
   1. Nested list item indented by 3 spaces"
            .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let ast = parse_document(&arena, &text, &Options::default());
        let doc = Document { path, ast, text };
        let rule = MD007::default();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![];
        assert_eq!(actual, expected);
    }
}
