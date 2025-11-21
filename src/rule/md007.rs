use comrak::nodes::{ListType, NodeValue};
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

    // TODO: This should be passed
    // #[test]
    // fn check_no_errors_with_blockquote() -> Result<()> {
    //     let text = indoc! {"
    //         * List
    //         > * List in blockquote
    //         >* List in blockquote
    //     "}
    //     .to_owned();
    //     let path = Path::new("test.md").to_path_buf();
    //     let arena = Arena::new();
    //     let doc = Document::new(&arena, path, text)?;
    //     let rule = MD007::default();
    //     let actual = rule.check(&doc)?;
    //     let expected = vec![];
    //     assert_eq!(actual, expected);
    //     Ok(())
    // }
}
