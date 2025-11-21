use std::path::PathBuf;

use comrak::nodes::{AstNode, ListType, NodeValue};
use miette::Result;

use crate::{Document, violation::Violation};

use super::{Metadata, RuleLike, Tag};

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct MD030 {
    ul_single: usize,
    ol_single: usize,
    ul_multi: usize,
    ol_multi: usize,
}

impl MD030 {
    const METADATA: Metadata = Metadata {
        name: "MD030",
        description: "Spaces after list markers",
        tags: &[Tag::Ol, Tag::Ul, Tag::Whitespace],
        aliases: &["list-marker-space"],
    };

    pub const DEFAULT_UL_SINGLE: usize = 1;
    pub const DEFAULT_OL_SINGLE: usize = 1;
    pub const DEFAULT_UL_MULTI: usize = 1;
    pub const DEFAULT_OL_MULTI: usize = 1;

    #[inline]
    #[must_use]
    pub const fn new(ul_single: usize, ol_single: usize, ul_multi: usize, ol_multi: usize) -> Self {
        Self {
            ul_single,
            ol_single,
            ul_multi,
            ol_multi,
        }
    }

    fn check_recursive<'a>(
        &self,
        root: &'a AstNode<'a>,
        path: &PathBuf,
        violations: &mut Vec<Violation>,
    ) {
        for node in root.children() {
            if let NodeValue::List(list) = node.data.borrow().value {
                for item_node in node.children() {
                    if let NodeValue::Item(item) = item_node.data.borrow().value {
                        // true if multiple Paragraph
                        let mut is_multi = item_node.children().count() > 1;

                        // Check for single Paragraph with multiple lines
                        if !is_multi
                            && let Some(child_node) = item_node.first_child()
                            && child_node.data.borrow().value == NodeValue::Paragraph
                        {
                            for inline_node in child_node.children() {
                                if inline_node.data.borrow().value == NodeValue::SoftBreak {
                                    is_multi = true;
                                }
                            }
                        }

                        let is_violated = match (is_multi, list.list_type) {
                            (true, ListType::Bullet) => item.padding > self.ul_multi + 1,
                            (true, ListType::Ordered) => item.padding > self.ol_multi + 2,
                            (false, ListType::Bullet) => item.padding > self.ul_single + 1,
                            (false, ListType::Ordered) => item.padding > self.ol_single + 2,
                        };

                        if is_violated {
                            let position = item_node.data.borrow().sourcepos;
                            let violation = self.to_violation(path.clone(), position);
                            violations.push(violation);
                        }

                        self.check_recursive(item_node, path, violations);
                    }
                }
            }
        }
    }
}

impl Default for MD030 {
    #[inline]
    fn default() -> Self {
        Self {
            ul_single: Self::DEFAULT_UL_SINGLE,
            ol_single: Self::DEFAULT_OL_SINGLE,
            ul_multi: Self::DEFAULT_UL_MULTI,
            ol_multi: Self::DEFAULT_OL_MULTI,
        }
    }
}

impl RuleLike for MD030 {
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
    fn check_errors_ul() -> Result<()> {
        let text = indoc! {"
            *   Foo
                Second paragraph
            *   Bar
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD030::default();
        let actual = rule.check(&doc)?;
        let expected = vec![
            rule.to_violation(path.clone(), Sourcepos::from((1, 1, 2, 20))),
            rule.to_violation(path, Sourcepos::from((3, 1, 3, 7))),
        ];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_errors_ul_with_newline() -> Result<()> {
        let text = indoc! {"
            *   Foo

                Second paragraph

            *   Bar
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD030::default();
        let actual = rule.check(&doc)?;
        let expected = vec![
            rule.to_violation(path.clone(), Sourcepos::from((1, 1, 4, 0))),
            rule.to_violation(path, Sourcepos::from((5, 1, 5, 7))),
        ];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_errors_ul_with_ul_single() -> Result<()> {
        let text = indoc! {"
            *   Foo
                Second paragraph
            *   Bar
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD030::new(3, 1, 1, 1);
        let actual = rule.check(&doc)?;
        let expected = vec![rule.to_violation(path, Sourcepos::from((1, 1, 2, 20)))];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_errors_ul_with_ul_multi() -> Result<()> {
        let text = indoc! {"
            *   Foo
                Second paragraph
            *   Bar
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD030::new(1, 1, 3, 1);
        let actual = rule.check(&doc)?;
        let expected = vec![rule.to_violation(path, Sourcepos::from((3, 1, 3, 7)))];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_errors_ol() -> Result<()> {
        let text = indoc! {"
            1.   Foo
                 Second paragraph
            1.   Bar
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD030::default();
        let actual = rule.check(&doc)?;
        let expected = vec![
            rule.to_violation(path.clone(), Sourcepos::from((1, 1, 2, 21))),
            rule.to_violation(path, Sourcepos::from((3, 1, 3, 8))),
        ];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_errors_ol_with_newline() -> Result<()> {
        let text = indoc! {"
            1.   Foo

                 Second paragraph

            1.   Bar
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD030::default();
        let actual = rule.check(&doc)?;
        let expected = vec![
            rule.to_violation(path.clone(), Sourcepos::from((1, 1, 4, 0))),
            rule.to_violation(path, Sourcepos::from((5, 1, 5, 8))),
        ];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_errors_ol_with_ol_single() -> Result<()> {
        let text = indoc! {"
            1.   Foo
                 Second paragraph
            1.   Bar
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD030::new(1, 3, 1, 1);
        let actual = rule.check(&doc)?;
        let expected = vec![rule.to_violation(path, Sourcepos::from((1, 1, 2, 21)))];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_errors_ol_with_ol_multi() -> Result<()> {
        let text = indoc! {"
            1.   Foo
                 Second paragraph
            1.   Bar
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD030::new(1, 1, 1, 3);
        let actual = rule.check(&doc)?;
        let expected = vec![rule.to_violation(path, Sourcepos::from((3, 1, 3, 8)))];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_errors_nested() -> Result<()> {
        let text = indoc! {"
            * Parent list
                1.  Foo
                    Second paragraph
                2.  Bar
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD030::default();
        let actual = rule.check(&doc)?;
        let expected = vec![
            rule.to_violation(path.clone(), Sourcepos::from((2, 5, 3, 24))),
            rule.to_violation(path, Sourcepos::from((4, 5, 4, 11))),
        ];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_no_errors() -> Result<()> {
        let text = indoc! {"
            * Foo
            * Bar
            * Baz

            1. Foo
            1. Bar
            1. Baz

            1. Foo
               * Bar
               * Baz
            1. Qux
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text)?;
        let rule = MD030::default();
        let actual = rule.check(&doc)?;
        let expected = vec![];
        assert_eq!(actual, expected);
        Ok(())
    }
}
