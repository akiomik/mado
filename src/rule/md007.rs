use comrak::arena_tree::Node;
use comrak::nodes::{ListType, NodeValue};
use miette::Result;

use crate::{Document, violation::Violation};

use super::{Metadata, RuleLike, Tag};

/// Unordered list indentation.
///
/// A nested bullet passes when its parent item's `padding` plus its own
/// `marker_offset` is `indent`, and a top-level bullet when its `marker_offset`
/// is 0 or `indent`. Both are comrak's: columns from the start of the
/// container's content, with tabs expanded.
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

        for node in doc.ast.descendants() {
            let data = node.data.borrow();
            if let NodeValue::Item(item) = data.value
                && item.list_type == ListType::Bullet
            {
                let maybe_parent_padding =
                    node.parent().and_then(Node::parent).and_then(|parent| {
                        match parent.data.borrow().value {
                            NodeValue::Item(parent_item) => Some(parent_item.padding),
                            _ => None,
                        }
                    });

                let is_misindented = maybe_parent_padding.map_or(
                    item.marker_offset != 0 && item.marker_offset != self.indent,
                    |padding| padding + item.marker_offset != self.indent,
                );

                if is_misindented {
                    let violation = self.to_violation(doc.path.clone(), data.sourcepos);
                    violations.push(violation);
                }
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

    // TODO: This should be passed. See #481.
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

    // Each case is a document, `indent`, and where MD007 reports, as (line, column).
    #[test]
    fn check_cases() -> Result<()> {
        type Case = (&'static str, usize, &'static [(usize, usize)]);
        let cases: &[Case] = &[
            // Nested under a bullet
            ("* a\n    * b\n", 4, &[]),
            ("* a\n   * b\n", 4, &[(2, 4)]),
            ("* a\n     * b\n", 4, &[(2, 6)]),
            ("* a\n  * b\n", 2, &[]),
            ("*   a\n    * b\n", 4, &[]),
            ("* a\n    * b\n        * c\n        * d\n    * e\n", 4, &[]),
            ("* a\n  text\n    * b\n\n      text\n        * c\n", 4, &[]),
            (" * a\n    * b\n    * c\n", 4, &[(1, 2), (2, 5), (3, 5)]),
            (" * a\n     * b\n", 4, &[(1, 2)]),
            ("*\n    * b\n", 4, &[]),
            ("*     code\n    * b\n", 4, &[]),
            // Nested under an ordered item (#481)
            ("1. a\n   * b\n", 4, &[(2, 4)]),
            ("1. a\n    * b\n", 4, &[]),
            ("10. a\n    * b\n", 4, &[]),
            // Tabs
            ("* a\n\t* b\n\t\t* c\n\t* d\n", 4, &[]),
            ("* a\n\t * b\n", 4, &[(2, 3)]),
            ("*\ta\n    * b\n", 4, &[]),
            ("* a\n\t* b\n", 2, &[(2, 2)]),
            // Top level
            (" * a\n", 4, &[(1, 2)]),
            ("  * a\n", 2, &[]),
            (" 1. a\n   * b\n", 2, &[(2, 4)]),
            // Blockquotes
            (
                "> * a\n>     * b\n>         * c\n>         * d\n>     * e\n",
                4,
                &[],
            ),
            (">\t* a\n", 4, &[(1, 3)]),
            ("* a\n  > * b\n  >   * c\n", 4, &[(3, 7)]),
            (">* a\n>    * b\n", 4, &[(2, 6)]),
        ];
        for &(text, indent, expected) in cases {
            let path = Path::new("test.md").to_path_buf();
            let arena = Arena::new();
            let doc = Document::new(&arena, path, text.to_owned())?;
            let actual: Vec<(usize, usize)> = MD007::new(indent)
                .check(&doc)?
                .iter()
                .map(|violation| {
                    let start = violation.position().start;
                    (start.line, start.column)
                })
                .collect();
            assert_eq!(actual, expected, "{text:?} with indent {indent}");
        }
        Ok(())
    }
}
