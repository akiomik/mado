use comrak::nodes::{ListType, NodeValue, Sourcepos};
use miette::Result;

use crate::{Document, violation::Violation};

use super::{Metadata, RuleLike, Tag};

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct MD007 {
    indent: usize,
}

impl MD007 {
    const METADATA: Metadata = Metadata {
        name: "MD007",
        description: "Unordered list indentation",
        tags: &[Tag::Bullet, Tag::Ul, Tag::Indentation],
        aliases: &["ul-indent"],
    };

    pub const DEFAULT_INDENT: usize = 4;

    #[inline]
    #[must_use]
    pub const fn new(indent: usize) -> Self {
        Self { indent }
    }

    /// Indentation of a list item relative to the blockquote that contains it, i.e.
    /// 0 for the `*` in `> * Foo` and 2 for the one in `>   * Foo`.
    ///
    /// `sourcepos` columns are absolute, so they count the `> ` prefix as
    /// indentation. Measuring from the last `>` on the line instead is what keeps a
    /// correctly indented quoted list from being reported. One space or tab after
    /// the marker belongs to the prefix rather than to the indentation, per
    /// `CommonMark`, so it is dropped.
    ///
    /// `None` means the line could not be read back, and the caller then skips the
    /// item: a position we cannot measure is not evidence of a violation. No input
    /// is known to produce it, since a list item inside a blockquote always carries
    /// a `>` on its own start line, but the lookup stays checked rather than
    /// relying on that.
    fn blockquote_indent(lines: &[String], position: Sourcepos) -> Option<usize> {
        let indent = position.start.column.checked_sub(1)?;
        let line = lines.get(position.start.line.checked_sub(1)?)?;
        let prefix = line.get(..indent)?;
        let marker = prefix.rfind('>')?;
        let remainder = prefix.get(marker + 1..)?;
        Some(
            remainder
                .strip_prefix([' ', '\t'])
                .unwrap_or(remainder)
                .len(),
        )
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
    fn metadata(&self) -> &'static Metadata {
        &Self::METADATA
    }

    #[inline]
    fn check(&self, doc: &Document) -> Result<Vec<Violation>> {
        let mut violations = vec![];
        let mut maybe_prev_indent = None;

        for node in doc.ast.descendants() {
            if let NodeValue::Item(item) = node.data.borrow().value {
                let position = node.data.borrow().sourcepos;
                let mut maybe_indent = position.start.column.checked_sub(1);

                let mut maybe_ancestor = node.parent();
                while let Some(ancestor) = maybe_ancestor {
                    if ancestor.data.borrow().value == NodeValue::BlockQuote {
                        maybe_indent = Self::blockquote_indent(&doc.lines, position);
                        break;
                    }
                    maybe_ancestor = ancestor.parent();
                }

                // An item whose indentation cannot be measured is skipped entirely,
                // leaving `maybe_prev_indent` untouched so the next item is compared
                // against the last position we could actually read.
                let Some(indent) = maybe_indent else {
                    continue;
                };

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

#[cfg(test)]
mod tests {
    use std::path::Path;

    use comrak::{Arena, nodes::Sourcepos};
    use indoc::indoc;
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn check_errors() -> Result<()> {
        let text = indoc! {"
            * List item
               * Nested list item indented by 3 spaces
                   * More nested list item indented by 4 spaces
            * List item
               * Nested list item indented by 3 spaces
                   * More nested list item indented by 4 spaces
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD007::default();
        let actual = rule.check(&doc)?;
        let expected = vec![
            rule.to_violation(path.clone(), Sourcepos::from((2, 4, 3, 51))),
            rule.to_violation(path, Sourcepos::from((5, 4, 6, 51))),
        ];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_errors_for_multiple_indentation() -> Result<()> {
        let text = indoc! {"
            * List item
                * Nested list item indented by 4 spaces
                    * More nested list item indented by 4 spaces
            * List item
                * Nested list item indented by 4 spaces
                    * More nested list item indented by 4 spaces
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD007::new(2);
        let actual = rule.check(&doc)?;
        let expected = vec![
            rule.to_violation(path.clone(), Sourcepos::from((2, 5, 3, 52))),
            rule.to_violation(path.clone(), Sourcepos::from((3, 9, 3, 52))),
            rule.to_violation(path.clone(), Sourcepos::from((5, 5, 6, 52))),
            rule.to_violation(path, Sourcepos::from((6, 9, 6, 52))),
        ];
        assert_eq!(actual, expected);
        Ok(())
    }

    // TODO: This should be passed
    // #[test]
    // fn check_errors_with_ol() -> Result<()> {
    //     let text = indoc! {"
    //         * List item
    //            1. Nested list item indented by 3 spaces
    //                * More nested list item indented by 4 spaces
    //         * List item
    //            1. Nested list item indented by 3 spaces
    //                * More nested list item indented by 4 spaces
    //     "}
    //     .to_owned();
    //     let path = Path::new("test.md").to_path_buf();
    //     let arena = Arena::new();
    //     let doc = Document::new(&arena, path.clone(), text)?;
    //     let rule = MD007::default();
    //     let actual = rule.check(&doc)?;
    //     let expected = vec![
    //         rule.to_violation(path.clone(), Sourcepos::from((3, 8, 3, 51))),
    //         rule.to_violation(path, Sourcepos::from((6, 8, 6, 51))),
    //     ];
    //     assert_eq!(actual, expected);
    //     Ok(())
    // }

    #[test]
    fn check_no_errors() -> Result<()> {
        let text = indoc! {"
            * List item
                * Nested list item indented by 4 spaces
                    * More nested list item indented by 4 spaces
            * List Item
                * Nested list item indented by 4 spaces
                    * More nested list item indented by 4 spaces
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text)?;
        let rule = MD007::default();
        let actual = rule.check(&doc)?;
        let expected = vec![];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_no_errors_ol() -> Result<()> {
        let text = indoc! {"
            * List item
               1. Nested list item indented by 3 spaces
            * List Item
               1. Nested list item indented by 3 spaces
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text)?;
        let rule = MD007::default();
        let actual = rule.check(&doc)?;
        let expected = vec![];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_no_errors_with_blockquote() -> Result<()> {
        let text = indoc! {"
            * List
            > * List in blockquote
            >* List in blockquote
            >\t* List in blockquote
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text)?;
        let rule = MD007::default();
        let actual = rule.check(&doc)?;
        let expected = vec![];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_no_errors_with_nested_blockquote() -> Result<()> {
        let text = indoc! {"
            > > * List
            > >     * Nested list
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text)?;
        let rule = MD007::default();
        let actual = rule.check(&doc)?;
        let expected = vec![];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_errors_with_blockquote() -> Result<()> {
        let text = indoc! {"
            > * List
            >    * Nested list indented by 3 spaces
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD007::default();
        let actual = rule.check(&doc)?;
        let expected = vec![rule.to_violation(path, Sourcepos::from((2, 6, 2, 39)))];
        assert_eq!(actual, expected);
        Ok(())
    }

    // `blockquote_indent` returns `None` only for positions that `check` cannot
    // produce, so those arms are exercised directly.
    #[test]
    fn blockquote_indent_measures_from_the_last_marker() {
        let lines = vec![">   * Foo".to_owned()];
        let position = Sourcepos::from((1, 5, 1, 9));
        assert_eq!(MD007::blockquote_indent(&lines, position), Some(2));
    }

    #[test]
    fn blockquote_indent_drops_one_tab_after_the_marker() {
        let lines = vec![">\t* Foo".to_owned()];
        let position = Sourcepos::from((1, 3, 1, 7));
        assert_eq!(MD007::blockquote_indent(&lines, position), Some(0));
    }

    #[test]
    fn blockquote_indent_without_marker() {
        let lines = vec!["  * Foo".to_owned()];
        let position = Sourcepos::from((1, 3, 1, 7));
        assert_eq!(MD007::blockquote_indent(&lines, position), None);
    }

    #[test]
    fn blockquote_indent_beyond_last_line() {
        let lines = vec!["> * Foo".to_owned()];
        let position = Sourcepos::from((2, 3, 2, 7));
        assert_eq!(MD007::blockquote_indent(&lines, position), None);
    }

    #[test]
    fn blockquote_indent_beyond_end_of_line() {
        let lines = vec!["> ".to_owned()];
        let position = Sourcepos::from((1, 9, 1, 9));
        assert_eq!(MD007::blockquote_indent(&lines, position), None);
    }

    #[test]
    fn blockquote_indent_inside_a_multibyte_character() {
        let lines = vec!["\u{3042}> * Foo".to_owned()];
        let position = Sourcepos::from((1, 3, 1, 7));
        assert_eq!(MD007::blockquote_indent(&lines, position), None);
    }
}
