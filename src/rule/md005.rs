use std::path::PathBuf;

use comrak::nodes::{AstNode, NodeValue};
use miette::Result;
use rustc_hash::FxHashMap;

use crate::{Document, violation::Violation};

use super::{Metadata, RuleLike, Tag};

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

    fn check_recursive<'a>(
        &self,
        root: &'a AstNode<'a>,
        path: &PathBuf,
        violations: &mut Vec<Violation>,
        levels: &mut FxHashMap<usize, usize>,
        level: usize,
        content_column: usize,
    ) {
        for node in root.children() {
            if let NodeValue::List(_) = node.data.borrow().value {
                for item_node in node.children() {
                    if let NodeValue::Item(item) = item_node.data.borrow().value {
                        let column = content_column + item.marker_offset;
                        if column != *levels.entry(level).or_insert(column) {
                            let position = item_node.data.borrow().sourcepos;
                            let violation = self.to_violation(path.clone(), position);
                            violations.push(violation);
                        }

                        let item_content_column = column + item.padding;
                        self.check_recursive(
                            item_node,
                            path,
                            violations,
                            levels,
                            level + 1,
                            item_content_column,
                        );
                    }
                }
            } else {
                // Lists below this node get their own baseline; see #486.
                //
                // NOTE: markdownlint reports nothing inside a blockquote here. It
                // measures indentation from the raw line, so `>   * Foo` counts as
                // indent 0 and every quoted item looks equally indented. That is a
                // side effect of how it measures rather than a decision about what
                // MD005 means, so the deviation is deliberate on our side.
                let mut scoped_levels = FxHashMap::default();
                self.check_recursive(node, path, violations, &mut scoped_levels, level, 0);
            }
        }
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
        let mut levels: FxHashMap<usize, usize> = FxHashMap::default();

        self.check_recursive(doc.ast, &doc.path, &mut violations, &mut levels, 0, 0);

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
    fn check_errors_for_lists() -> Result<()> {
        let text = indoc! {"
            * List 1
              * item 1
              * item 2

            Some text

            1. List 2
               1. A misaligned item
               1. More misaligned item
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD005::new();
        let actual = rule.check(&doc)?;
        let expected = vec![
            rule.to_violation(path.clone(), Sourcepos::from((8, 4, 8, 23))),
            rule.to_violation(path, Sourcepos::from((9, 4, 9, 26))),
        ];
        assert_eq!(actual, expected);
        Ok(())
    }

    // NOTE: This test case is not marked as a violation in markdownlint
    #[test]
    fn check_errors_with_test_and_list_in_list() -> Result<()> {
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
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD005::new();
        let actual = rule.check(&doc)?;
        let expected = vec![
            rule.to_violation(path.clone(), Sourcepos::from((7, 4, 7, 11))),
            rule.to_violation(path, Sourcepos::from((8, 4, 8, 11))),
        ];
        assert_eq!(actual, expected);
        Ok(())
    }

    // NOTE: markdownlint reports nothing here, see the comment in `check_recursive`.
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

            text

            * c
             \t* d
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD005::new();
        let actual = rule.check(&doc)?;
        let expected = vec![rule.to_violation(path, Sourcepos::from((7, 3, 7, 5)))];
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
    fn check_no_errors_for_blockquote_with_own_baseline() -> Result<()> {
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
    fn check_errors_in_deep_nesting() -> Result<()> {
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
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD005::new();
        let actual = rule.check(&doc)?;
        let expected = vec![
            rule.to_violation(path.clone(), Sourcepos::from((5, 4, 6, 8))),
            rule.to_violation(path, Sourcepos::from((6, 6, 6, 8))),
        ];
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
}
