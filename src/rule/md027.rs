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

    /// Where line `lineno` carries more than one space or tab after a blockquote
    /// marker the quote owns, from what follows the spaces to the end of the line.
    /// One space after a marker belongs to it, per `CommonMark`.
    ///
    /// The line is read rather than the inlines on it, since an inline that opened
    /// on an earlier line leaves the first inline of this one beginning wherever
    /// that inline closed. Its markers are read off it too, the same nesting being
    /// written at different columns from one line to the next, as `>>` and `> >`
    /// are. `offset` skips to the marker comrak measured on the line the quote
    /// starts at, the one line a list item's marker can precede.
    ///
    /// The quote owns the innermost `own_markers` of the `markers` quoting the
    /// line. The spaces before the outermost it owns are the indentation of
    /// whatever holds that marker rather than any quote's, and go unmeasured; the
    /// ones after it are the quote's. A line carrying fewer markers than it is
    /// quoted by is measured for as many as it carries, and one carrying none is
    /// left alone.
    ///
    /// What this reads as a prefix and `CommonMark` does not is #455, what it
    /// cannot tell from a list item's indentation is #456, and the tab it counts
    /// as one space here and as its own width above is #458.
    ///
    /// `None` for a line `lines` does not hold, and for one the `offset` is not
    /// within. The caller reports nothing for either.
    fn indented_content_positions(
        lines: &[String],
        lineno: usize,
        markers: usize,
        own_markers: usize,
        offset: usize,
    ) -> Option<Vec<Sourcepos>> {
        let line = lines.get(lineno.checked_sub(1)?)?;
        let mut rest = line.get(offset..)?;
        let mut prefix_len = offset;
        let mut prefix_width = Self::expanded_width(line.get(..offset)?, 0);
        let mut positions = vec![];
        let content_position = |column| Sourcepos::from((lineno, column, lineno, line.len()));

        for marker in 0..markers {
            let at_marker = if marker == 0 {
                rest.trim_start_matches(' ')
            } else {
                rest.trim_start_matches([' ', '\t'])
            };
            let spaces = rest.len() - at_marker.len();
            let owned = marker + own_markers > markers;

            // A marker takes one space of its own and three of indentation, per
            // `CommonMark`. Past that the `>` is text, and the spaces are the
            // previous marker's content rather than indentation. Only a gap this
            // quote answers for is held to it: one a list item indents is the
            // item's, and as wide as the item is.
            let width = Self::expanded_width(rest.get(..spaces)?, prefix_width);
            let marker_here = at_marker.starts_with('>') && !(owned && width > 4);

            if !marker_here {
                if owned && spaces > 1 && !at_marker.is_empty() {
                    positions.push(content_position(prefix_len + spaces + 1));
                }

                return Some(positions);
            }

            if owned && spaces > 1 {
                positions.push(content_position(prefix_len + spaces + 1));
            }

            rest = at_marker.get(1..)?;
            prefix_len += spaces + 1;
            prefix_width += width + 1;
        }

        let content = rest.trim_start_matches([' ', '\t']);
        let spaces = rest.len() - content.len();

        if spaces > 1 && !content.is_empty() {
            positions.push(content_position(prefix_len + spaces + 1));
        }

        Some(positions)
    }

    /// How wide `text` is, starting `from` columns into the line, with each tab
    /// taking the columns up to the next stop of four as `CommonMark` expands them.
    /// `from` is a width rather than a column: the first column of a line is 0 of
    /// them.
    fn expanded_width(text: &str, from: usize) -> usize {
        text.chars().fold(0, |width, character| {
            width
                + if character == '\t' {
                    // The columns up to the next stop, which is every fourth and
                    // so is what the low two bits count off.
                    4 - ((from + width) & 3)
                } else {
                    1
                }
        })
    }

    /// Appends what line `lineno` carries after the markers `prefix` describes:
    /// how many quote the line, how many of those the quote owns, and the byte to
    /// start reading them at.
    fn measure(
        &self,
        doc: &Document,
        lineno: usize,
        prefix: (usize, usize, usize),
        violations: &mut Vec<Violation>,
    ) {
        let (markers, own_markers, offset) = prefix;
        let positions =
            Self::indented_content_positions(&doc.lines, lineno, markers, own_markers, offset);

        for position in positions.into_iter().flatten() {
            violations.push(self.to_violation(doc.path.clone(), position));
        }
    }

    /// How many blockquotes `node`, itself a blockquote, is quoted by, itself
    /// included.
    fn block_quote_depth<'a>(node: &'a AstNode<'a>) -> usize {
        node.ancestors()
            .filter(|ancestor| ancestor.data.borrow().value == NodeValue::BlockQuote)
            .count()
    }

    /// How many blockquotes `node`, itself a blockquote, is quoted by with nothing
    /// but another blockquote in between, itself included.
    fn nested_block_quotes<'a>(node: &'a AstNode<'a>) -> usize {
        node.ancestors()
            .take_while(|ancestor| ancestor.data.borrow().value == NodeValue::BlockQuote)
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
            if node.data.borrow().value == NodeValue::BlockQuote {
                let block_quote_position = node.data.borrow().sourcepos;
                let depth = Self::block_quote_depth(node);
                let nested = Self::nested_block_quotes(node);

                // The quote's own marker on the line it starts at, where comrak has
                // already said which column it is at and a list item's marker can
                // precede it. Every line after is read from its start, for the
                // markers it carries of the ones quoting it, which is the only way
                // to read a line the same nesting is written differently on.
                let prefix = |lineno| {
                    if lineno == block_quote_position.start.line {
                        (1, 1, block_quote_position.start.column.saturating_sub(1))
                    } else {
                        (depth, nested, 0)
                    }
                };

                // Every block comrak gives the quote a node for, not only the
                // first: a blank quoted line ends one and starts another, and the
                // indentation of what follows is the quote's to answer for too.
                // A link reference definition is left no node and so unmeasured,
                // which is #461.
                for child_node in node.children() {
                    let child_position = child_node.data.borrow().sourcepos;

                    match &child_node.data.borrow().value {
                        NodeValue::Paragraph => {
                            for lineno in child_position.start.line..=child_position.end.line {
                                self.measure(doc, lineno, prefix(lineno), &mut violations);
                            }
                        }
                        NodeValue::List(_) => {
                            // TODO: Support multi-line errors
                            for item_node in child_node.children() {
                                let lineno = item_node.data.borrow().sourcepos.start.line;
                                self.measure(doc, lineno, prefix(lineno), &mut violations);
                            }
                        }
                        _ => {
                            // TODO: Support multi-line errors
                            let lineno = child_position.start.line;
                            self.measure(doc, lineno, prefix(lineno), &mut violations);
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

    // NOTE: A line of whitespace `CommonMark` does not end a line with is
    // content, and the content of this one begins two spaces after the marker.
    #[test]
    fn check_errors_paragraph_with_unicode_whitespace() -> Result<()> {
        let text = indoc! {"
            >  \u{a0}
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD027::new();
        let actual = rule.check(&doc)?;
        let expected = vec![rule.to_violation(path, Sourcepos::from((1, 4, 1, 5)))];
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
    fn check_errors_paragraph_in_list_item_with_nested_block_quote() -> Result<()> {
        let text = indoc! {"
            > -   >  Indented text
            >     Not indented
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD027::new();
        let actual = rule.check(&doc)?;
        let expected = vec![rule.to_violation(path, Sourcepos::from((1, 10, 1, 22)))];
        assert_eq!(actual, expected);
        Ok(())
    }

    // NOTE: The `>` on line 2 is five columns in, past the four a marker takes,
    // so it is text and the spaces before it are the outer marker's content.
    #[test]
    fn check_errors_paragraph_with_over_indented_marker() -> Result<()> {
        let text = indoc! {"
            > > Quoted text
            >     >  More text
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD027::new();
        let actual = rule.check(&doc)?;
        let expected = vec![rule.to_violation(path, Sourcepos::from((2, 7, 2, 18)))];
        assert_eq!(actual, expected);
        Ok(())
    }

    // NOTE: The tab on line 2 takes the two columns up to the next stop, which
    // leaves the `>` after it within a marker's reach.
    #[test]
    fn check_errors_paragraph_with_tab_between_markers() -> Result<()> {
        let text = indoc! {"
            > > Quoted text
            > \t>  More text
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD027::new();
        let actual = rule.check(&doc)?;
        let expected = vec![
            rule.to_violation(path.clone(), Sourcepos::from((2, 4, 2, 15))),
            rule.to_violation(path, Sourcepos::from((2, 7, 2, 15))),
        ];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_errors_later_paragraph() -> Result<()> {
        let text = indoc! {"
            > Quoted text
            >
            >  More quoted text
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD027::new();
        let actual = rule.check(&doc)?;
        let expected = vec![rule.to_violation(path, Sourcepos::from((3, 4, 3, 19)))];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_errors_later_list() -> Result<()> {
        let text = indoc! {"
            > Quoted text
            >
            >  * Item
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD027::new();
        let actual = rule.check(&doc)?;
        let expected = vec![rule.to_violation(path, Sourcepos::from((3, 4, 3, 9)))];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_errors_later_code_block() -> Result<()> {
        let text = indoc! {"
            > Quoted text
            >
            >  ```
            >  foo
            >  ```
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD027::new();
        let actual = rule.check(&doc)?;
        let expected = vec![rule.to_violation(path, Sourcepos::from((3, 4, 3, 6)))];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_errors_later_block_quote() -> Result<()> {
        let text = indoc! {"
            > Quoted text
            >
            >  >  More quoted text
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD027::new();
        let actual = rule.check(&doc)?;
        let expected = vec![
            rule.to_violation(path.clone(), Sourcepos::from((3, 4, 3, 22))),
            rule.to_violation(path, Sourcepos::from((3, 7, 3, 22))),
        ];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_errors_later_block_with_indented_block_quote() -> Result<()> {
        let text = indoc! {"
               > Quoted text
               >
            >  # Heading
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD027::new();
        let actual = rule.check(&doc)?;
        let expected = vec![rule.to_violation(path, Sourcepos::from((3, 4, 3, 12)))];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_errors_later_block_in_nested_block_quote() -> Result<()> {
        let text = indoc! {"
            > > Quoted text
            > >
            >  > # Heading
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD027::new();
        let actual = rule.check(&doc)?;
        let expected = vec![rule.to_violation(path, Sourcepos::from((3, 4, 3, 14)))];
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
            rule.to_violation(path, Sourcepos::from((1, 4, 1, 6))),
            // TODO: This results are expected
            // rule.to_violation(path.clone(), Sourcepos::from((2, 4, 2, 6))),
            // rule.to_violation(path.clone(), Sourcepos::from((4, 4, 4, 6))),
            // rule.to_violation(path, Sourcepos::from((5, 4, 5, 6))),
        ];
        assert_eq!(actual, expected);
        Ok(())
    }

    // NOTE: The four spaces after the marker's own are what make this a code
    // block, so the report cannot be acted on. #459 is whether to make it.
    #[test]
    fn check_errors_indented_code_block() -> Result<()> {
        let text = indoc! {"
            >      code block
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD027::new();
        let actual = rule.check(&doc)?;
        let expected = vec![rule.to_violation(path, Sourcepos::from((1, 8, 1, 17)))];
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
            rule.to_violation(path, Sourcepos::from((1, 4, 1, 8))),
            // TODO: This results are expected
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
            rule.to_violation(path.clone(), Sourcepos::from((1, 4, 1, 58))),
            rule.to_violation(path.clone(), Sourcepos::from((1, 7, 1, 58))),
            rule.to_violation(path.clone(), Sourcepos::from((1, 10, 1, 58))),
            rule.to_violation(path.clone(), Sourcepos::from((3, 4, 3, 55))),
            rule.to_violation(path.clone(), Sourcepos::from((3, 7, 3, 55))),
            rule.to_violation(path, Sourcepos::from((3, 10, 3, 55))),
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
    fn check_errors_with_nested_block_quotes4() -> Result<()> {
        let text = indoc! {"
            > > Quoted text
            >  > More quoted text
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD027::new();
        let actual = rule.check(&doc)?;
        let expected = vec![rule.to_violation(path, Sourcepos::from((2, 4, 2, 21)))];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_errors_with_nested_block_quotes5() -> Result<()> {
        let text = indoc! {"
            > > Quoted text
            >   More quoted text
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD027::new();
        let actual = rule.check(&doc)?;
        let expected = vec![rule.to_violation(path, Sourcepos::from((2, 5, 2, 20)))];
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

    // NOTE: A tab is four columns of indentation, more than `CommonMark` allows
    // before a marker, so the `>` on line 2 is text.
    #[test]
    fn check_no_errors_paragraph_with_tab_indented_continuation() -> Result<()> {
        let text = indoc! {"
            > Quoted text
            \t>  More text
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
    fn check_no_errors_later_paragraph() -> Result<()> {
        let text = indoc! {"
            > Quoted text
            >
            > More quoted text
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

    // NOTE: A continuation line may carry three spaces of indentation before the
    // marker, and one space follows the marker on every line here.
    #[test]
    fn check_no_errors_later_block_with_indented_marker() -> Result<()> {
        let text = indoc! {"
            > Quoted text
            >
               > # Heading
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
