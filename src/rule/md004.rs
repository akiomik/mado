use std::path::PathBuf;

use comrak::nodes::{AstNode, ListType, NodeList, NodeValue};
use miette::Result;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};

use crate::Document;
use crate::violation::Violation;

use super::{Metadata, RuleLike, Tag};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum ListStyle {
    Consistent,
    Asterisk,
    Plus,
    Dash,
    Sublist,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct MD004 {
    style: ListStyle,
}

impl MD004 {
    const METADATA: Metadata = Metadata {
        name: "MD004",
        description: "Unordered list style",
        tags: &[Tag::Bullet, Tag::Ul],
        aliases: &["ul-style"],
    };

    pub const DEFAULT_LIST_STYLE: ListStyle = ListStyle::Consistent;

    #[inline]
    #[must_use]
    pub const fn new(style: ListStyle) -> Self {
        Self { style }
    }

    fn check_recursive<'a>(
        &self,
        root: &'a AstNode<'a>,
        path: &PathBuf,
        violations: &mut Vec<Violation>,
        initial_maybe_list_char: Option<char>,
        levels: &mut FxHashMap<usize, char>,
        level: usize,
    ) {
        let mut maybe_list_char = initial_maybe_list_char;

        for node in root.children() {
            if let NodeValue::List(_) = node.data.borrow().value {
                for item_node in node.children() {
                    if let NodeValue::Item(NodeList {
                        list_type: ListType::Bullet,
                        bullet_char,
                        ..
                    }) = item_node.data.borrow().value
                    {
                        let is_violated = match &self.style {
                            ListStyle::Consistent => maybe_list_char
                                .is_some_and(|list_char| bullet_char as char != list_char),
                            ListStyle::Asterisk => bullet_char != b'*',
                            ListStyle::Plus => bullet_char != b'+',
                            ListStyle::Dash => bullet_char != b'-',
                            ListStyle::Sublist => levels
                                .get(&level)
                                .is_some_and(|list_char| bullet_char as char != *list_char),
                        };

                        if is_violated {
                            let position = item_node.data.borrow().sourcepos;
                            let violation = self.to_violation(path.clone(), position);
                            violations.push(violation);
                        }

                        if maybe_list_char.is_none() {
                            maybe_list_char = Some(bullet_char as char);
                        }

                        if self.style == ListStyle::Sublist {
                            levels.entry(level).or_insert(bullet_char as char);
                        }
                    }

                    self.check_recursive(
                        item_node,
                        path,
                        violations,
                        maybe_list_char,
                        levels,
                        level + 1,
                    );
                }
            }
        }
    }
}

impl Default for MD004 {
    #[inline]
    fn default() -> Self {
        Self {
            style: Self::DEFAULT_LIST_STYLE,
        }
    }
}

impl RuleLike for MD004 {
    #[inline]
    fn metadata(&self) -> &'static Metadata {
        &Self::METADATA
    }

    #[inline]
    fn check(&self, doc: &Document) -> Result<Vec<Violation>> {
        let mut violations = vec![];
        let mut levels = FxHashMap::default();

        self.check_recursive(doc.ast, &doc.path, &mut violations, None, &mut levels, 1);

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
    fn check_errors_for_consistent() -> Result<()> {
        let text = indoc! {"
            * Item 1
            + Item 2
            - Item 3
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD004::default();
        let actual = rule.check(&doc)?;
        let expected = vec![
            rule.to_violation(path.clone(), Sourcepos::from((2, 1, 2, 8))),
            rule.to_violation(path, Sourcepos::from((3, 1, 3, 8))),
        ];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_errors_for_asterisk() -> Result<()> {
        let text = indoc! {"
            * Item 1
            + Item 2
            - Item 3
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD004::new(ListStyle::Asterisk);
        let actual = rule.check(&doc)?;
        let expected = vec![
            rule.to_violation(path.clone(), Sourcepos::from((2, 1, 2, 8))),
            rule.to_violation(path, Sourcepos::from((3, 1, 3, 8))),
        ];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_errors_for_plus() -> Result<()> {
        let text = indoc! {"
            * Item 1
            + Item 2
            - Item 3
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD004::new(ListStyle::Plus);
        let actual = rule.check(&doc)?;
        let expected = vec![
            rule.to_violation(path.clone(), Sourcepos::from((1, 1, 1, 8))),
            rule.to_violation(path, Sourcepos::from((3, 1, 3, 8))),
        ];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_errors_for_dash() -> Result<()> {
        let text = indoc! {"
            * Item 1
            + Item 2
            - Item 3
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD004::new(ListStyle::Dash);
        let actual = rule.check(&doc)?;
        let expected = vec![
            rule.to_violation(path.clone(), Sourcepos::from((1, 1, 1, 8))),
            rule.to_violation(path, Sourcepos::from((2, 1, 2, 8))),
        ];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_errors_for_sublist() -> Result<()> {
        let text = indoc! {"
            * Item 1
            * Item 2
              - Item 2a
                + Item 2a1
              - Item 2b
            * Item 3

            Other stuff

            - Item 1
            - Item 2
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD004::new(ListStyle::Sublist);
        let actual = rule.check(&doc)?;
        let expected = vec![
            rule.to_violation(path.clone(), Sourcepos::from((10, 1, 10, 8))),
            rule.to_violation(path, Sourcepos::from((11, 1, 11, 8))),
        ];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_no_errors_for_consistent() -> Result<()> {
        let text = indoc! {"
            * Item 1
            * Item 2
            * Item 3
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text)?;
        let rule = MD004::default();
        let actual = rule.check(&doc)?;
        let expected = vec![];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_no_errors_for_asterisk() -> Result<()> {
        let text = indoc! {"
            * Item 1
            * Item 2
            * Item 3
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text)?;
        let rule = MD004::new(ListStyle::Asterisk);
        let actual = rule.check(&doc)?;
        let expected = vec![];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_no_errors_for_plus() -> Result<()> {
        let text = indoc! {"
            + Item 1
            + Item 2
            + Item 3
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text)?;
        let rule = MD004::new(ListStyle::Plus);
        let actual = rule.check(&doc)?;
        let expected = vec![];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_no_errors_for_dash() -> Result<()> {
        let text = indoc! {"
            - Item 1
            - Item 2
            - Item 3
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text)?;
        let rule = MD004::new(ListStyle::Dash);
        let actual = rule.check(&doc)?;
        let expected = vec![];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_no_errors_for_sublist() -> Result<()> {
        let text = indoc! {"
            * Item 1
            * Item 2
              - Item 2a
                + Item 2a1
              - Item 2b
            * Item 3

            Other stuff

            * Item 1
            * Item 2
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text)?;
        let rule = MD004::new(ListStyle::Sublist);
        let actual = rule.check(&doc)?;
        let expected = vec![];
        assert_eq!(actual, expected);
        Ok(())
    }

    // NOTE: This test case is marked as a violation in markdownlint
    #[test]
    fn check_no_errors_with_blockquote() -> Result<()> {
        let text = indoc! {"
            >- Item 1
            >- Item 2
            >- Item 3
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text)?;
        let rule = MD004::default();
        let actual = rule.check(&doc)?;
        let expected = vec![];
        assert_eq!(actual, expected);
        Ok(())
    }
}
