use comrak::nodes::{AstNode, NodeValue, Sourcepos};
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
    /// when more than one space or tab separates it from the innermost of the
    /// `depth` blockquote markers that follow byte `offset`. One of those spaces
    /// belongs to the marker, per `CommonMark`, and the rest is what this rule
    /// reports.
    ///
    /// The line is read rather than the inlines on it, since an inline that
    /// opened on an earlier line leaves the first inline of this one beginning
    /// wherever that inline closed.
    ///
    /// The markers are counted off the line itself rather than taken from the
    /// column the quote started at, which is a column the same nesting can be
    /// written at differently from one line to the next, as `>>` and `> >` are.
    /// `offset` is for the line the quote starts at, where what precedes the
    /// marker is a list item's own, and comrak has already said which column the
    /// marker is at.
    ///
    /// `None` for a line with nothing to report, and for one that does not carry
    /// `depth` markers after `offset`: a lazy continuation line carries none, and
    /// its text is not a prefix however much like one it reads.
    fn indented_content_position(
        lines: &[String],
        lineno: usize,
        depth: usize,
        offset: usize,
    ) -> Option<Sourcepos> {
        let line = lines.get(lineno.checked_sub(1)?)?;
        let mut after_markers = line.get(offset..)?;
        let mut prefix_len = offset;

        for _ in 0..depth {
            let at_marker = after_markers.trim_start_matches([' ', '\t']);
            let spaces = after_markers.len() - at_marker.len();
            after_markers = at_marker.strip_prefix('>')?;
            prefix_len += spaces + 1;
        }

        let content = after_markers.trim_start_matches([' ', '\t']);
        let spaces = after_markers.len() - content.len();

        if spaces < 2 || content.trim().is_empty() {
            return None;
        }

        let column = prefix_len + spaces + 1;
        Some(Sourcepos::from((lineno, column, lineno, line.len())))
    }

    /// How many blockquotes `node`, itself a blockquote, is quoted by, itself
    /// included.
    fn block_quote_depth<'a>(node: &'a AstNode<'a>) -> usize {
        node.ancestors()
            .filter(|ancestor| ancestor.data.borrow().value == NodeValue::BlockQuote)
            .count()
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
                        let block_quote_position = node.data.borrow().sourcepos;
                        let depth = Self::block_quote_depth(node);
                        let paragraph_position = child_node.data.borrow().sourcepos;
                        for lineno in paragraph_position.start.line..=paragraph_position.end.line {
                            // The quote's own marker on the line it starts at, and
                            // the whole prefix on every line after, where nothing
                            // but the quote's markers can precede the content.
                            let (markers, offset) = if lineno == block_quote_position.start.line {
                                (1, block_quote_position.start.column.saturating_sub(1))
                            } else {
                                (depth, 0)
                            };

                            if let Some(position) =
                                Self::indented_content_position(&doc.lines, lineno, markers, offset)
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

            > **bold
            >  span**
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD027::new();
        let actual = rule.check(&doc)?;
        let expected = vec![
            rule.to_violation(path.clone(), Sourcepos::from((2, 4, 2, 20))),
            rule.to_violation(path, Sourcepos::from((5, 4, 5, 9))),
        ];
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
    fn check_errors_paragraph_in_list_item() -> Result<()> {
        let text = indoc! {"
            - >  Indented text
              >  More indented
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD027::new();
        let actual = rule.check(&doc)?;
        let expected = vec![
            rule.to_violation(path.clone(), Sourcepos::from((1, 6, 1, 18))),
            rule.to_violation(path, Sourcepos::from((2, 6, 2, 18))),
        ];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_errors_paragraph_with_tab() -> Result<()> {
        let text = indoc! {"
            > \tIndented text
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD027::new();
        let actual = rule.check(&doc)?;
        let expected = vec![rule.to_violation(path, Sourcepos::from((1, 4, 1, 16)))];
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
    fn check_errors_with_nested_block_quotes3() -> Result<()> {
        let text = indoc! {"
            >> Original
            > >  Reply
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD027::new();
        let actual = rule.check(&doc)?;
        let expected = vec![rule.to_violation(path, Sourcepos::from((2, 6, 2, 10)))];
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

    // NOTE: Line 2 is a lazy continuation line. It carries no marker, and the
    // `>` in its text is not one however much the column it sits at looks like
    // the quote's.
    #[test]
    fn check_no_errors_paragraph_with_lazy_continuation() -> Result<()> {
        let text = indoc! {"
            > > Quoted text
            ab>  more text
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
