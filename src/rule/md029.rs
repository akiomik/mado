use std::path::PathBuf;

use comrak::nodes::{AstNode, ListType, NodeValue};
use miette::Result;
use serde::{Deserialize, Serialize};

use crate::{Document, violation::Violation};

use super::{Metadata, RuleLike, Tag};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum OrderedListStyle {
    One,
    Ordered,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct MD029 {
    style: OrderedListStyle,
}

impl MD029 {
    const METADATA: Metadata = Metadata {
        name: "MD029",
        description: "Ordered list item prefix",
        tags: &[Tag::Ol],
        aliases: &["ol-prefix"],
    };

    pub const DEFAULT_STYLE: OrderedListStyle = OrderedListStyle::One;

    #[inline]
    #[must_use]
    pub const fn new(style: OrderedListStyle) -> Self {
        Self { style }
    }

    // Iterative on purpose: recursing once per nested container overflows the
    // stack on deeply nested blockquotes. Items are visited in document order,
    // which also reaches lists inside blockquotes (`> 1. Foo`).
    fn check_items<'a>(
        &self,
        root: &'a AstNode<'a>,
        path: &PathBuf,
        violations: &mut Vec<Violation>,
    ) {
        for item_node in root.descendants() {
            let NodeValue::Item(item) = item_node.data.borrow().value else {
                continue;
            };
            let Some(parent) = item_node.parent() else {
                continue;
            };
            let NodeValue::List(list) = parent.data.borrow().value else {
                continue;
            };
            if list.list_type != ListType::Ordered {
                continue;
            }

            let is_violated = match self.style {
                OrderedListStyle::One => item.start != 1,
                OrderedListStyle::Ordered => item_node
                    .previous_sibling()
                    .and_then(|prev| match prev.data.borrow().value {
                        NodeValue::Item(prev_item) => Some(prev_item.start),
                        _ => None,
                    })
                    .is_some_and(|prev_start| item.start != prev_start + 1),
            };

            if is_violated {
                let position = item_node.data.borrow().sourcepos;
                violations.push(self.to_violation(path.clone(), position));
            }
        }
    }
}

impl Default for MD029 {
    #[inline]
    fn default() -> Self {
        Self {
            style: Self::DEFAULT_STYLE,
        }
    }
}

impl RuleLike for MD029 {
    #[inline]
    fn metadata(&self) -> &'static Metadata {
        &Self::METADATA
    }

    #[inline]
    fn check(&self, doc: &Document) -> Result<Vec<Violation>> {
        let mut violations = vec![];

        self.check_items(doc.ast, &doc.path, &mut violations);

        Ok(violations)
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use comrak::{Arena, nodes::Sourcepos};
    use indoc::indoc;
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn check_errors_one() -> Result<()> {
        let text = indoc! {"
            1. Do this.
            2. Do that.
            3. Done.
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD029::default();
        let actual = rule.check(&doc)?;
        let expected = vec![
            rule.to_violation(path.clone(), Sourcepos::from((2, 1, 2, 11))),
            rule.to_violation(path, Sourcepos::from((3, 1, 3, 8))),
        ];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_errors_ordered() -> Result<()> {
        let text = indoc! {"
            1. Do this.
            1. Do that.
            1. Done.
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD029::new(OrderedListStyle::Ordered);
        let actual = rule.check(&doc)?;
        let expected = vec![
            rule.to_violation(path.clone(), Sourcepos::from((2, 1, 2, 11))),
            rule.to_violation(path, Sourcepos::from((3, 1, 3, 8))),
        ];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_errors_recursive() -> Result<()> {
        let text = indoc! {"
            * Parent list
                1. Do this.
                2. Do that.
                3. Done.
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD029::default();
        let actual = rule.check(&doc)?;
        let expected = vec![
            rule.to_violation(path.clone(), Sourcepos::from((3, 5, 3, 15))),
            rule.to_violation(path, Sourcepos::from((4, 5, 4, 12))),
        ];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_errors_in_blockquote() -> Result<()> {
        let text = indoc! {"
            > 1. Do this.
            > 2. Do that.
            > 3. Done.
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD029::default();
        let actual = rule.check(&doc)?;
        let expected = vec![
            rule.to_violation(path.clone(), Sourcepos::from((2, 3, 2, 13))),
            rule.to_violation(path, Sourcepos::from((3, 3, 3, 10))),
        ];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_errors_in_nested_blockquote() -> Result<()> {
        let text = indoc! {"
            > > 1. Do this.
            > > 2. Do that.
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD029::default();
        let actual = rule.check(&doc)?;
        let expected = vec![rule.to_violation(path, Sourcepos::from((2, 5, 2, 15)))];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_no_errors_one() -> Result<()> {
        let text = indoc! {"
            1. Do this.
            1. Do that.
            1. Done.
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text)?;
        let rule = MD029::default();
        let actual = rule.check(&doc)?;
        let expected = vec![];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_no_errors_ordered() -> Result<()> {
        let text = indoc! {"
            1. Do this.
            2. Do that.
            3. Done.
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text)?;
        let rule = MD029::new(OrderedListStyle::Ordered);
        let actual = rule.check(&doc)?;
        let expected = vec![];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_no_errors_in_blockquote() -> Result<()> {
        let text = indoc! {"
            > 1. Do this.
            > 1. Do that.
            > 1. Done.
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text)?;
        let rule = MD029::default();
        let actual = rule.check(&doc)?;
        let expected = vec![];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_no_errors_recursive() -> Result<()> {
        let text = indoc! {"
            * Parent list
                1. Do this.
                1. Do that.
                1. Done.
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text)?;
        let rule = MD029::default();
        let actual = rule.check(&doc)?;
        let expected = vec![];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_does_not_overflow_the_stack_on_deeply_nested_blockquotes() -> Result<()> {
        let text = ">".repeat(5000) + " * a";
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text)?;
        let actual = MD029::default().check(&doc)?;
        assert_eq!(actual, vec![]);
        Ok(())
    }
}
