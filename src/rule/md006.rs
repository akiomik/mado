use comrak::nodes::{AstNode, ListType, NodeList, NodeValue};
use miette::Result;

use crate::{violation::Violation, Document};

use super::{
    node::{NodeContext, NodeValueMatcher},
    NewRuleLike, NodeRule, RuleLike, RuleMetadata,
};

#[derive(Default, Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct MD006;

impl MD006 {
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self {}
    }
}

impl RuleLike for MD006 {
    #[inline]
    fn name(&self) -> &'static str {
        "MD006"
    }

    #[inline]
    fn description(&self) -> &'static str {
        "Consider starting bulleted lists at the beginning of the line"
    }

    #[inline]
    fn tags(&self) -> Vec<&'static str> {
        vec!["bullet", "ul", "indentation"]
    }

    #[inline]
    fn aliases(&self) -> Vec<&'static str> {
        vec!["ul-start-left"]
    }

    #[inline]
    fn check(&self, doc: &Document) -> Result<Vec<Violation>> {
        let mut violations = vec![];

        for node in doc.ast.children() {
            if let NodeValue::List(NodeList {
                list_type: ListType::Bullet,
                ..
            }) = node.data.borrow().value
            {
                for item_node in node.children() {
                    if let NodeValue::Item(item) = item_node.data.borrow().value {
                        if item.marker_offset > 0 {
                            let position = item_node.data.borrow().sourcepos;
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

impl NewRuleLike for MD006 {
    fn metadata(&self) -> RuleMetadata {
        RuleMetadata {
            name: "MD006",
            description: "Consider starting bulleted lists at the beginning of the line",
            tags: vec!["bullet", "ul", "indentation"],
            aliases: vec!["ul-start-left"],
        }
    }
}

impl NodeRule for MD006 {
    fn matcher(&self) -> NodeValueMatcher {
        NodeValueMatcher::new(|value| {
            matches!(
                value,
                NodeValue::Item(NodeList { marker_offset, .. }) if *marker_offset > 0
            )
        })
    }

    fn run<'a>(&mut self, ctx: &NodeContext, node: &'a AstNode<'a>) -> Result<Vec<Violation>> {
        if let Some(list_level) = ctx.list_level {
            if list_level == 1 {
                let position = node.data.borrow().sourcepos;
                let violation = self.to_violation(ctx.path.clone(), position);
                return Ok(vec![violation]);
            }
        }

        Ok(vec![])
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

  * List item
  * List item"
            .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let ast = parse_document(&arena, &text, &Options::default());
        let doc = Document {
            path: path.clone(),
            ast,
            text,
        };
        let rule = MD006::new();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![
            rule.to_violation(path.clone(), Sourcepos::from((3, 3, 3, 13))),
            rule.to_violation(path, Sourcepos::from((4, 3, 4, 13))),
        ];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_no_errors() {
        let text = "Some test

* List item
* List item"
            .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let ast = parse_document(&arena, &text, &Options::default());
        let doc = Document { path, ast, text };
        let rule = MD006::new();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_no_errors_with_ordered_list() {
        let text = "Some test

 1. Ordered list item
 2. Ordered list item"
            .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let ast = parse_document(&arena, &text, &Options::default());
        let doc = Document { path, ast, text };
        let rule = MD006::new();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_no_errors_with_nested_list() {
        let text = "* List
    * List item
    * List item"
            .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let ast = parse_document(&arena, &text, &Options::default());
        let doc = Document { path, ast, text };
        let rule = MD006::new();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![];
        assert_eq!(actual, expected);
    }
}
