use comrak::nodes::NodeValue;
use miette::Result;

use crate::{Document, violation::Violation};

use super::{Metadata, RuleLike, Tag};

/// Inconsistent indentation for list items at the same level.
///
/// Every item of a list must start at the same column as the list's first
/// item, counted from the start of the content that holds the list. Items of
/// different lists are not compared.
///
/// This differs from mdl, which compares items at the same depth across the
/// whole document, and reports nothing in a blockquote.
#[derive(Default, Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct MD005;

impl MD005 {
    const METADATA: Metadata = Metadata {
        name: "MD005",
        description: "Inconsistent indentation for list items at the same level",
        tags: &[Tag::Bullet, Tag::Ul, Tag::Indentation],
        aliases: &["list-indent"],
    };

    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        Self {}
    }
}

impl RuleLike for MD005 {
    #[inline]
    fn metadata(&self) -> &'static Metadata {
        &Self::METADATA
    }

    #[inline]
    fn check(&self, doc: &Document) -> Result<Vec<Violation>> {
        let mut violations = vec![];

        for list in doc.ast.descendants() {
            if !matches!(list.data.borrow().value, NodeValue::List(_)) {
                continue;
            }

            let mut first_offset = None;
            for item_node in list.children() {
                if let NodeValue::Item(item) = item_node.data.borrow().value
                    && item.marker_offset != *first_offset.get_or_insert(item.marker_offset)
                {
                    let position = item_node.data.borrow().sourcepos;
                    violations.push(self.to_violation(doc.path.clone(), position));
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
            * Item 1
                * Nested item 1
                * Nested item 2
               * A misaligned item
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD005::new();
        let actual = rule.check(&doc)?;
        let expected = vec![rule.to_violation(path, Sourcepos::from((4, 4, 4, 22)))];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_errors_for_empty_item_text() -> Result<()> {
        let text = indoc! {"
            *
                *
                *
               *
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD005::new();
        let actual = rule.check(&doc)?;
        let expected = vec![rule.to_violation(path, Sourcepos::from((4, 4, 4, 4)))];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_no_errors_for_sublists_of_different_lists() -> Result<()> {
        let text = indoc! {"
            * List 1
              * item 1
              * item 2

            Some text

            1. List 2
               1. item 3
               1. item 4
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text)?;
        let rule = MD005::new();
        let actual = rule.check(&doc)?;
        let expected = vec![];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_no_errors_for_list_after_text_in_item() -> Result<()> {
        let text = indoc! {"
            * List 1
              * Item 1
              * Item 2

            1. List 2
               Text in list
               * item 3
               * item 4
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text)?;
        let rule = MD005::new();
        let actual = rule.check(&doc)?;
        let expected = vec![];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_errors_in_blockquote() -> Result<()> {
        let text = indoc! {"
            > * Item 1
            >  * Item 2
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD005::new();
        let actual = rule.check(&doc)?;
        let expected = vec![rule.to_violation(path, Sourcepos::from((2, 4, 2, 11)))];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_no_errors_for_blockquote_alongside_top_level_list() -> Result<()> {
        let text = indoc! {"
            * Item 1
            * Item 2

            > * Item 3
            > * Item 4
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text)?;
        let rule = MD005::new();
        let actual = rule.check(&doc)?;
        let expected = vec![];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_no_errors() -> Result<()> {
        let text = indoc! {"
            * Item 1
                * Nested item 1
                * Nested item 2
                * Nested item 3
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text)?;
        let rule = MD005::new();
        let actual = rule.check(&doc)?;
        let expected = vec![];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_no_errors_for_lists() -> Result<()> {
        let text = indoc! {"
            * List 1
                * item 1
                * item 2

            Some text

            * List 2
                1. item 3
                2. item 4
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text)?;
        let rule = MD005::new();
        let actual = rule.check(&doc)?;
        let expected = vec![];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_no_errors_with_tab_indented_sibling() -> Result<()> {
        let text = indoc! {"
            * a
                * b
            \t* c
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text)?;
        let rule = MD005::new();
        let actual = rule.check(&doc)?;
        let expected = vec![];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_errors_with_tab_indentation() -> Result<()> {
        let text = indoc! {"
            * a
               * b
            \t* c
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD005::new();
        let actual = rule.check(&doc)?;
        let expected = vec![rule.to_violation(path, Sourcepos::from((3, 2, 3, 4)))];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_no_errors_with_blockquote_marker_spacing() -> Result<()> {
        let text = indoc! {"
            > * a
            >* b
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text)?;
        let rule = MD005::new();
        let actual = rule.check(&doc)?;
        let expected = vec![];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_errors_with_tab_after_blockquote_marker() -> Result<()> {
        let text = indoc! {"
            >\t* a
            > * b
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD005::new();
        let actual = rule.check(&doc)?;
        let expected = vec![rule.to_violation(path, Sourcepos::from((2, 3, 2, 5)))];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_no_errors_with_bom() -> Result<()> {
        let text = indoc! {"
            \u{feff}* a
            * b
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text)?;
        let rule = MD005::new();
        let actual = rule.check(&doc)?;
        let expected = vec![];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_no_errors_for_blockquote_after_list() -> Result<()> {
        let text = indoc! {"
            * a
            * b

            >  * c
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text)?;
        let rule = MD005::new();
        let actual = rule.check(&doc)?;
        let expected = vec![];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_no_errors_in_deep_nesting() -> Result<()> {
        let text = indoc! {"
            * a
              * b
                * c
            * d
               * e
                 * f
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text)?;
        let rule = MD005::new();
        let actual = rule.check(&doc)?;
        let expected = vec![];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_no_errors_with_tab_after_marker() -> Result<()> {
        let text = indoc! {"
            *\ta
            \t* b
            * c
                * d
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text)?;
        let rule = MD005::new();
        let actual = rule.check(&doc)?;
        let expected = vec![];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_no_errors_for_sublists_under_parents_of_different_widths() -> Result<()> {
        let text = indoc! {"
            9. a
               * b
            10. c
                * d
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text)?;
        let rule = MD005::new();
        let actual = rule.check(&doc)?;
        let expected = vec![];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_errors_across_blank_line() -> Result<()> {
        let text = indoc! {"
            * a

             * b
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD005::new();
        let actual = rule.check(&doc)?;
        let expected = vec![rule.to_violation(path, Sourcepos::from((3, 2, 3, 4)))];
        assert_eq!(actual, expected);
        Ok(())
    }
}
