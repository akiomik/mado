use comrak::arena_tree::Node;
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

    // See #481.
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

    #[test]
    fn check_no_errors_with_siblings() -> Result<()> {
        let text = indoc! {"
            * List item
                * Nested list item
                    * More nested list item
                    * More nested list item
                        * Most nested list item
                        * Most nested list item
                    * More nested list item
                * Nested list item
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
    fn check_no_errors_with_paragraphs() -> Result<()> {
        let text = indoc! {"
            * List item
              continued
                * Nested list item
                    * More nested list item
                    * More nested list item

                      Another paragraph
                        * Most nested list item
                        * Most nested list item
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
    fn check_errors_with_siblings() -> Result<()> {
        let text = indoc! {"
            * List item
                * Nested list item
                    * More nested list item
                     * More nested list item indented by 5 spaces
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD007::default();
        let actual = rule.check(&doc)?;
        let expected = vec![rule.to_violation(path, Sourcepos::from((4, 10, 4, 53)))];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_no_errors_with_wide_padding() -> Result<()> {
        let text = indoc! {"
            *   List item
                * Nested list item
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
    fn check_no_errors_with_tab_after_marker() -> Result<()> {
        let text = "*\tList item\n    * Nested list item\n".to_owned();
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
    fn check_no_errors_top_level_at_indent() -> Result<()> {
        let text =
            "  * List item indented by 2 spaces\n  * List item indented by 2 spaces\n".to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text)?;
        let rule = MD007::new(2);
        let actual = rule.check(&doc)?;
        let expected = vec![];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_errors_with_blockquote_in_item() -> Result<()> {
        let text = indoc! {"
            * List item
              > * List in blockquote
              >   * Nested list indented by 2 spaces
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD007::default();
        let actual = rule.check(&doc)?;
        let expected = vec![rule.to_violation(path, Sourcepos::from((3, 7, 3, 40)))];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_no_errors_with_tabs() -> Result<()> {
        let text = "* List item\n\t* Nested list item\n\t\t* More nested list item\n\t* Nested list item\n".to_owned();
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
    fn check_errors_with_tabs() -> Result<()> {
        let text = "* List item\n\t * Nested list item indented by 5 columns\n".to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD007::default();
        let actual = rule.check(&doc)?;
        let expected = vec![rule.to_violation(path, Sourcepos::from((2, 3, 2, 42)))];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_no_errors_with_siblings_in_blockquote() -> Result<()> {
        let text = indoc! {"
            > * List item
            >     * Nested list item
            >         * More nested list item
            >         * More nested list item
            >     * Nested list item
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

    // `>\t` leaves two columns of indentation, as `>   ` does.
    #[test]
    fn check_errors_with_tab_in_blockquote() -> Result<()> {
        let text = ">\t* List\n".to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD007::default();
        let actual = rule.check(&doc)?;
        let expected = vec![rule.to_violation(path, Sourcepos::from((1, 3, 1, 8)))];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_errors_top_level_after_another_list() -> Result<()> {
        let text = " 1. List item\n   * List item indented by 3 spaces\n".to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD007::new(2);
        let actual = rule.check(&doc)?;
        let expected = vec![rule.to_violation(path, Sourcepos::from((2, 4, 2, 35)))];
        assert_eq!(actual, expected);
        Ok(())
    }

    // See #481.
    #[test]
    fn check_errors_under_ordered_item() -> Result<()> {
        let text = "1. List item\n   * Nested list item indented by 3 spaces\n".to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD007::default();
        let actual = rule.check(&doc)?;
        let expected = vec![rule.to_violation(path, Sourcepos::from((2, 4, 2, 42)))];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_errors_in_misindented_list() -> Result<()> {
        let text = " * List item\n    * Nested list item\n    * Nested list item\n".to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD007::default();
        let actual = rule.check(&doc)?;
        let expected = vec![
            rule.to_violation(path.clone(), Sourcepos::from((1, 2, 3, 22))),
            rule.to_violation(path.clone(), Sourcepos::from((2, 5, 2, 22))),
            rule.to_violation(path, Sourcepos::from((3, 5, 3, 22))),
        ];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_errors_with_tabs_and_indent_2() -> Result<()> {
        let text = "* List item\n\t* Nested list item\n".to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD007::new(2);
        let actual = rule.check(&doc)?;
        let expected = vec![rule.to_violation(path, Sourcepos::from((2, 2, 2, 19)))];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_errors_under_misindented_sibling() -> Result<()> {
        let text = "* List item\n * List item\n    * Nested list item\n".to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD007::default();
        let actual = rule.check(&doc)?;
        let expected = vec![
            rule.to_violation(path.clone(), Sourcepos::from((2, 2, 3, 22))),
            rule.to_violation(path, Sourcepos::from((3, 5, 3, 22))),
        ];
        assert_eq!(actual, expected);
        Ok(())
    }

    // Baseline for #481, which is expected to change how parents are measured.
    #[test]
    fn check_no_errors_with_unusual_parents() -> Result<()> {
        let text = indoc! {"
            *
                * Nested under an empty item

            Paragraph

            *     Indented code in an item
                * Nested list item

            Paragraph

            1. List item
                * Nested under an ordered item

            Paragraph

            10. List item
                * Nested under an ordered item
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
}
