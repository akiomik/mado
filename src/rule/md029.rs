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

    fn check_recursive<'a>(
        &self,
        root: &'a AstNode<'a>,
        path: &PathBuf,
        violations: &mut Vec<Violation>,
    ) {
        for node in root.children() {
            if let NodeValue::List(list) = node.data.borrow().value {
                let mut maybe_prev_start = None;

                for item_node in node.children() {
                    if let NodeValue::Item(item) = item_node.data.borrow().value {
                        if list.list_type == ListType::Ordered {
                            let is_violated = match self.style {
                                OrderedListStyle::One => item.start != 1,
                                OrderedListStyle::Ordered => maybe_prev_start
                                    .is_some_and(|prev_start| item.start != prev_start + 1),
                            };

                            if is_violated {
                                let position = item_node.data.borrow().sourcepos;
                                let violation = self.to_violation(path.clone(), position);
                                violations.push(violation);
                            }

                            maybe_prev_start = Some(item.start);
                        }

                        self.check_recursive(item_node, path, violations);
                    }
                }
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

        self.check_recursive(doc.ast, &doc.path, &mut violations);

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
}
