use std::path::PathBuf;

use comrak::nodes::{AstNode, ListType, NodeValue, Sourcepos};
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

    /// Width of an ordered list marker as written, e.g. 2 for `9.`, 3 for `10.`,
    /// 4 for `007.`.
    ///
    /// `NodeList::padding` counts the marker itself, so the marker width has to be
    /// subtracted before the remainder can be compared against `ol_single`/`ol_multi`.
    /// Assuming a fixed width makes every item from 10 onwards look over-indented.
    ///
    /// The width is read back from the line rather than derived from
    /// `NodeList::start` because `CommonMark` permits leading zeros: `007.` and `7.`
    /// share an ordinal but not a width, and it is the written width that `padding`
    /// counts.
    ///
    /// `None` means the marker could not be read back, and the caller then reports
    /// nothing: an unmeasurable marker is not evidence of a violation, and this rule
    /// has no way to express one the author could act on.
    fn ordered_marker_width(lines: &[String], position: Sourcepos) -> Option<usize> {
        let line = lines.get(position.start.line.checked_sub(1)?)?;
        let rest = line.get(position.start.column.checked_sub(1)?..)?;
        let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
        if digits == 0 {
            return None;
        }

        match rest.as_bytes().get(digits) {
            Some(b'.' | b')') => Some(digits + 1),
            _ => None,
        }
    }

    fn check_recursive<'a>(
        &self,
        root: &'a AstNode<'a>,
        path: &PathBuf,
        lines: &[String],
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

                        let position = item_node.data.borrow().sourcepos;
                        let ordered_threshold = |configured: usize| {
                            Self::ordered_marker_width(lines, position)
                                .map(|width| configured + width)
                        };

                        let is_violated = match (is_multi, list.list_type) {
                            (true, ListType::Bullet) => item.padding > self.ul_multi + 1,
                            (true, ListType::Ordered) => ordered_threshold(self.ol_multi)
                                .is_some_and(|threshold| item.padding > threshold),
                            (false, ListType::Bullet) => item.padding > self.ul_single + 1,
                            (false, ListType::Ordered) => ordered_threshold(self.ol_single)
                                .is_some_and(|threshold| item.padding > threshold),
                        };

                        if is_violated {
                            let violation = self.to_violation(path.clone(), position);
                            violations.push(violation);
                        }

                        self.check_recursive(item_node, path, lines, violations);
                    }
                }
            } else {
                // See the comment on the equivalent branch in MD029: lists reached
                // through a blockquote or other non-item container must still be
                // checked. `ordered_marker_width` needs no adjustment for them,
                // because `sourcepos` columns already account for the `> ` prefix.
                self.check_recursive(node, path, lines, violations);
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

        self.check_recursive(doc.ast, &doc.path, &doc.lines, &mut violations);

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
            rule.to_violation(path.clone(), Sourcepos::from((1, 1, 3, 20))),
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
            rule.to_violation(path.clone(), Sourcepos::from((1, 1, 3, 21))),
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
    fn check_errors_ol_multi_digit_marker() -> Result<()> {
        let text = indoc! {"
            9.   Foo
            10.   Bar
            100.   Baz
            11.  Qux
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD030::default();
        let actual = rule.check(&doc)?;
        let expected = vec![
            rule.to_violation(path.clone(), Sourcepos::from((1, 1, 1, 8))),
            rule.to_violation(path.clone(), Sourcepos::from((2, 1, 2, 9))),
            rule.to_violation(path.clone(), Sourcepos::from((3, 1, 3, 10))),
            rule.to_violation(path, Sourcepos::from((4, 1, 4, 8))),
        ];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_errors_ol_leading_zero_marker() -> Result<()> {
        let text = indoc! {"
            007.  Foo
            01.  Bar
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD030::default();
        let actual = rule.check(&doc)?;
        let expected = vec![
            rule.to_violation(path.clone(), Sourcepos::from((1, 1, 1, 9))),
            rule.to_violation(path, Sourcepos::from((2, 1, 2, 8))),
        ];
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
    fn check_errors_in_blockquote_ul() -> Result<()> {
        let text = indoc! {"
            > *   Foo
            > *   Bar
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD030::default();
        let actual = rule.check(&doc)?;
        let expected = vec![
            rule.to_violation(path.clone(), Sourcepos::from((1, 3, 1, 9))),
            rule.to_violation(path, Sourcepos::from((2, 3, 2, 9))),
        ];
        assert_eq!(actual, expected);
        Ok(())
    }

    // The marker width is read back from the source line, so this also covers the
    // `> ` prefix being accounted for by `sourcepos` columns.
    #[test]
    fn check_errors_in_blockquote_ol_multi_digit_marker() -> Result<()> {
        let text = indoc! {"
            > 9.   Foo
            > 10.   Bar
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD030::default();
        let actual = rule.check(&doc)?;
        let expected = vec![
            rule.to_violation(path.clone(), Sourcepos::from((1, 3, 1, 10))),
            rule.to_violation(path, Sourcepos::from((2, 3, 2, 11))),
        ];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_no_errors_in_blockquote() -> Result<()> {
        let text = indoc! {"
            > * Foo
            > * Bar
            >
            > 10. Baz
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

    #[test]
    fn check_no_errors_ol_leading_zero_marker() -> Result<()> {
        let text = indoc! {"
            007. Foo
            01. Bar
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

    #[test]
    fn check_no_errors_ol_multi_digit_marker() -> Result<()> {
        let text = indoc! {"
            8. Foo
            9. Bar
            10. Baz
            11. Qux

            99. Foo
            100. Bar

            10. Foo
                Second paragraph
            11. Bar
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
