use comrak::nodes::{NodeValue, Sourcepos};
use miette::Result;

use crate::{Document, violation::Violation};

use super::{Metadata, RuleLike, Tag};

#[derive(Debug, Default, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct MD027;

impl MD027 {
    const METADATA: Metadata = Metadata {
        name: "MD027",
        description: "Multiple spaces after blockquote symbol",
        tags: &[Tag::Blockquote, Tag::Whitespace, Tag::Indentation],
        aliases: &["no-multiple-space-blockquote"],
    };

    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        Self {}
    }

    /// The content of line `lineno` from where it begins to the end of the line,
    /// when more than one space separates it from the blockquote marker at
    /// `marker_column`. One space after the marker belongs to the prefix, per
    /// `CommonMark`, and the rest is what this rule reports.
    ///
    /// The line is read rather than the inlines on it, since an inline that
    /// opened on an earlier line leaves the first inline of this one beginning
    /// wherever that inline closed.
    ///
    /// `None` for a line with nothing to report, a line the marker cannot be
    /// read at included: a lazy continuation line carries no marker, and a
    /// nested quote's markers move between lines that spell the nesting with
    /// different widths.
    fn indented_content_position(
        lines: &[String],
        lineno: usize,
        marker_column: usize,
    ) -> Option<Sourcepos> {
        let line = lines.get(lineno.checked_sub(1)?)?;
        let after_marker = line
            .get(marker_column.checked_sub(1)?..)?
            .strip_prefix('>')?;
        let content = after_marker.trim_start_matches(' ');
        let spaces = after_marker.len() - content.len();

        if spaces < 2 || content.trim().is_empty() {
            return None;
        }

        let column = marker_column + spaces + 1;
        Some(Sourcepos::from((lineno, column, lineno, line.len())))
    }
}

impl RuleLike for MD027 {
    #[inline]
    fn metadata(&self) -> &'static Metadata {
        &Self::METADATA
    }

    #[inline]
    fn check(&self, doc: &Document) -> Result<Vec<Violation>> {
        let mut violations = vec![];

        for node in doc.ast.descendants() {
            if node.data.borrow().value == NodeValue::BlockQuote
                && let Some(child_node) = node.first_child()
            {
                match &child_node.data.borrow().value {
                    NodeValue::Paragraph => {
                        let marker_column = node.data.borrow().sourcepos.start.column;
                        let paragraph_position = child_node.data.borrow().sourcepos;
                        for lineno in paragraph_position.start.line..=paragraph_position.end.line {
                            if let Some(position) =
                                Self::indented_content_position(&doc.lines, lineno, marker_column)
                            {
                                let violation = self.to_violation(doc.path.clone(), position);
                                violations.push(violation);
                            }
                        }
                    }
                    NodeValue::List(_) => {
                        for item_node in child_node.children() {
                            let block_quote_position = node.data.borrow().sourcepos;
                            let item_position = item_node.data.borrow().sourcepos;
                            let expected_column = block_quote_position.start.column + 2;

                            if item_position.start.column > expected_column {
                                let violation = self.to_violation(doc.path.clone(), item_position);
                                violations.push(violation);
                            }
                        }
                    }
                    _ => {
                        // TODO: Support multi-line errors
                        let parent_position = node.data.borrow().sourcepos;
                        let child_position = child_node.data.borrow().sourcepos;
                        if child_position.start.column > parent_position.start.column + 2 {
                            let violation = self.to_violation(doc.path.clone(), child_position);
                            violations.push(violation);
                        }
                    }
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
    fn check_errors_paragraph() -> Result<()> {
        let text = indoc! {"
            >  Indented text
            >  More indented
            > Not indented
            >  *Emph* and text
            >  **Strong** and text
            >  `code` and text
            >  [link](https://example.com) and text
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD027::new();
        let actual = rule.check(&doc)?;
        let expected = vec![
            rule.to_violation(path.clone(), Sourcepos::from((1, 4, 1, 16))),
            rule.to_violation(path.clone(), Sourcepos::from((2, 4, 2, 16))),
            rule.to_violation(path.clone(), Sourcepos::from((4, 4, 4, 18))),
            rule.to_violation(path.clone(), Sourcepos::from((5, 4, 5, 22))),
            rule.to_violation(path.clone(), Sourcepos::from((6, 4, 6, 18))),
            rule.to_violation(path, Sourcepos::from((7, 4, 7, 39))),
        ];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_errors_paragraph_with_multiple_line_inline() -> Result<()> {
        let text = indoc! {"
            > **bold
            >  span.** tail here
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD027::new();
        let actual = rule.check(&doc)?;
        let expected = vec![rule.to_violation(path, Sourcepos::from((2, 4, 2, 20)))];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_errors_paragraph_with_indented_block_quote() -> Result<()> {
        let text = indoc! {"
            Text

              >  Indented text
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD027::new();
        let actual = rule.check(&doc)?;
        let expected = vec![rule.to_violation(path, Sourcepos::from((3, 6, 3, 18)))];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_errors_list() -> Result<()> {
        let text = indoc! {"
            >  * foo
            > * bar
            >   * baz
            >  * quz
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD027::new();
        let actual = rule.check(&doc)?;
        let expected = vec![
            rule.to_violation(path.clone(), Sourcepos::from((1, 4, 1, 8))),
            rule.to_violation(path, Sourcepos::from((4, 4, 4, 8))),
        ];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_errors_code_block() -> Result<()> {
        let text = indoc! {"
            >  ```
            >  foo
            > bar
            >  baz
            >  ```
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD027::new();
        let actual = rule.check(&doc)?;
        let expected = vec![
            rule.to_violation(path, Sourcepos::from((1, 4, 5, 6))),
            // TODO: This results are expected
            // rule.to_violation(path.clone(), Sourcepos::from((1, 4, 1, 6))),
            // rule.to_violation(path.clone(), Sourcepos::from((2, 4, 2, 6))),
            // rule.to_violation(path.clone(), Sourcepos::from((4, 4, 4, 6))),
            // rule.to_violation(path, Sourcepos::from((5, 4, 5, 6))),
        ];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_errors_html_block_single_line() -> Result<()> {
        let text = indoc! {"
            >  <div>text</div>
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD027::new();
        let actual = rule.check(&doc)?;
        let expected = vec![rule.to_violation(path, Sourcepos::from((1, 4, 1, 18)))];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_errors_html_block_multiple_lines() -> Result<()> {
        let text = indoc! {"
            >  <div>
            > <p>some text</p>
            >   </div>
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD027::new();
        let actual = rule.check(&doc)?;
        let expected = vec![
            rule.to_violation(path, Sourcepos::from((1, 4, 3, 10))),
            // TODO: This results are expected
            // rule.to_violation(path.clone(), Sourcepos::from((1, 4, 1, 8))),
            // rule.to_violation(path, Sourcepos::from((3, 4, 3, 10))),
        ];
        assert_eq!(actual, expected);
        Ok(())
    }

    // NOTE: This case is not an error in markdownlint
    #[test]
    fn check_errors_with_nested_block_quotes() -> Result<()> {
        let text = indoc! {"
            >>>  This is multiple blockquotes with bad indentation.
            >>> More multiple blockquotes with good indentation.
            >>>  More multiple blockquotes with bad indentation.
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD027::new();
        let actual = rule.check(&doc)?;
        let expected = vec![
            rule.to_violation(path.clone(), Sourcepos::from((1, 6, 1, 55))),
            rule.to_violation(path, Sourcepos::from((3, 6, 3, 52))),
        ];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_errors_with_nested_block_quotes2() -> Result<()> {
        let text = indoc! {"
            >  >  >  This is multiple blockquote with bad indentation.
            > > > More multiple blockquote with good indentation.
            >  >  >  More multiple blockquote with bad indentation.
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD027::new();
        let actual = rule.check(&doc)?;
        let expected = vec![
            rule.to_violation(path.clone(), Sourcepos::from((1, 4, 3, 55))),
            rule.to_violation(path.clone(), Sourcepos::from((1, 7, 3, 55))),
            rule.to_violation(path.clone(), Sourcepos::from((1, 10, 1, 58))),
            rule.to_violation(path, Sourcepos::from((3, 10, 3, 55))),
            // TODO: This results are expected
            // rule.to_violation(path.clone(), Sourcepos::from((1, 4, 1, 58))),
            // rule.to_violation(path.clone(), Sourcepos::from((1, 7, 1, 58))),
            // rule.to_violation(path.clone(), Sourcepos::from((1, 10, 1, 58))),
            // rule.to_violation(path.clone(), Sourcepos::from((3, 4, 3, 55))),
            // rule.to_violation(path.clone(), Sourcepos::from((3, 7, 3, 55))),
            // rule.to_violation(path, Sourcepos::from((3, 10, 3, 55))),
        ];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_no_errors_paragraph() -> Result<()> {
        let text = indoc! {"
            > Text
            > More text
            > *Emph* and text
            > **Strong** and text
            > `code` and text
            > [link](https://example.com) and text
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text)?;
        let rule = MD027::new();
        let actual = rule.check(&doc)?;
        let expected = vec![];
        assert_eq!(actual, expected);
        Ok(())
    }

    // NOTE: One space follows every marker here. The inline that spans both
    // lines leaves the first inline of the second line beginning where it
    // closed, which is not where the line's content begins.
    #[test]
    fn check_no_errors_paragraph_with_multiple_line_inline() -> Result<()> {
        let text = indoc! {"
            > **bold
            > span.** tail here

            > _em
            > span._ tail here

            > [link
            > text](https://example.com) tail here

            > `code
            > span` tail here
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text)?;
        let rule = MD027::new();
        let actual = rule.check(&doc)?;
        let expected = vec![];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_no_errors_list() -> Result<()> {
        let text = indoc! {"
            > * foo
            > * bar
            >   * baz
            > * quz
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text)?;
        let rule = MD027::new();
        let actual = rule.check(&doc)?;
        let expected = vec![];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_no_errors_code_block() -> Result<()> {
        let text = indoc! {"
            > ```
            > foo
            > bar
            > baz
            > ```
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text)?;
        let rule = MD027::new();
        let actual = rule.check(&doc)?;
        let expected = vec![];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_no_errors_html_block() -> Result<()> {
        let text = indoc! {"
            > <div>
            > <p>some text</p>
            > </div>
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text)?;
        let rule = MD027::new();
        let actual = rule.check(&doc)?;
        let expected = vec![];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_no_errors_with_nested_block_quotes() -> Result<()> {
        let text = indoc! {"
            >>> This is multiple blockquotes with good indentation.
            >>> More multiple blockquotes with good indentation.
            >>> More multiple blockquotes with good indentation.
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text)?;
        let rule = MD027::new();
        let actual = rule.check(&doc)?;
        let expected = vec![];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_no_errors_with_nested_block_quotes2() -> Result<()> {
        let text = indoc! {"
            > > > This is multiple blockquote with good indentation.
            > > > More multiple blockquote with good indentation.
            > > > More multiple blockquote with good indentation.
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text)?;
        let rule = MD027::new();
        let actual = rule.check(&doc)?;
        let expected = vec![];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_no_errors_with_nested_block_quotes3() -> Result<()> {
        let text = indoc! {"
            >>> This is multiple blockquote with good indentation.
                More multiple blockquote with good indentation.
                More multiple blockquote with good indentation.
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text)?;
        let rule = MD027::new();
        let actual = rule.check(&doc)?;
        let expected = vec![];
        assert_eq!(actual, expected);
        Ok(())
    }
}
