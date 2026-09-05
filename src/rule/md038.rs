extern crate alloc;

use alloc::borrow::Cow;

use comrak::nodes::{NodeCode, NodeValue, Sourcepos};
use miette::Result;

use crate::{Document, violation::Violation};

use super::{Metadata, RuleLike, Tag};

#[derive(Debug, Default, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct MD038;

impl MD038 {
    const METADATA: Metadata = Metadata {
        name: "MD038",
        description: "Spaces inside code span elements",
        tags: &[Tag::Whitespace, Tag::Code],
        aliases: &["no-space-in-code"],
    };

    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        Self {}
    }

    /// The text between a code span's delimiters, as it was written, before
    /// `CommonMark` removed any padding from it.
    ///
    /// `code.literal` is that text after the removal, so on its own it cannot
    /// say whether anything was taken off. What says so is how wide the span is:
    /// `sourcepos` covers the delimiters, and comrak carries `num_backticks`, so
    /// the content is `2 * num_backticks` bytes narrower than the span. Against
    /// `literal` that width is either equal — nothing was removed — or two
    /// bytes longer, which is the one space `CommonMark` takes off each end.
    ///
    /// Widths are used rather than the line itself because `sourcepos` is not
    /// always an index into it. comrak unescapes a table cell before parsing its
    /// inlines, so inside one every `\|` earlier in the cell shifts the columns
    /// of everything after it by a byte. A width survives that — both ends shift
    /// together — where a slice of `doc.lines` would read the wrong text.
    ///
    /// `None` means the width and `literal` disagree by an amount `CommonMark`
    /// has no rule for, and the caller then skips the span: a span we cannot
    /// account for is not evidence of a violation.
    fn same_line_content(code: &NodeCode, position: Sourcepos) -> Option<Cow<'_, str>> {
        let width = (position.end.column + 1)
            .checked_sub(position.start.column)?
            .checked_sub(2 * code.num_backticks)?;

        match width.checked_sub(code.literal.len())? {
            0 => Some(Cow::Borrowed(&code.literal)),
            2 => Some(Cow::Owned(format!(" {} ", code.literal))),
            _ => None,
        }
    }

    /// The same text for a span whose delimiters sit on different lines.
    ///
    /// A width cannot be had here — the two columns index different lines — and
    /// `literal` has already turned each line ending into a space, so neither
    /// says what was written against the delimiters. The lines do, when they can
    /// be trusted, and a table cell cannot reach this far: a row ends at its line
    /// ending, so a span that crosses one is never inside a cell.
    ///
    /// Trust has to be earned rather than assumed. comrak measures an inline
    /// against the paragraph's content, not against the file, and the two part
    /// company once a continuation line has been stripped of its indentation. In
    /// the GitLab corpus under `scripts/benchmarks/data`,
    /// `doc/migrate_ci_to_ce/README.md:134` lands two columns past its own
    /// closing delimiter. So the delimiters are looked for where `sourcepos`
    /// claims they are, and `None` says they were not there.
    ///
    /// Only the two ends are read back. What lies between them cannot make the
    /// content all spaces, because a blank line would have ended the paragraph
    /// rather than the code span, so the joined ends stand in for the whole.
    fn multi_line_content(
        lines: &[String],
        position: Sourcepos,
        num_backticks: usize,
    ) -> Option<String> {
        let delimiter = "`".repeat(num_backticks);
        let opened = lines
            .get(position.start.line.checked_sub(1)?)?
            .get(position.start.column.checked_sub(1)?..)?
            .strip_prefix(&delimiter)?;
        let closed = lines
            .get(position.end.line.checked_sub(1)?)?
            .get(..position.end.column)?
            .strip_suffix(&delimiter)?;

        // The line ending between them is a space, and so is every one the lines
        // in between contribute.
        Some(format!("{opened} {closed}"))
    }

    /// The content as a reader sees it, or `None` when `CommonMark` leaves it
    /// alone.
    ///
    /// `CommonMark` removes one space from each end of a code span that both
    /// begins and ends with one, unless it is nothing but spaces. Those two
    /// spaces are the only ones that can be there out of necessity rather than
    /// carelessness, so which spaces were removed is what this rule turns on.
    fn strip_padding(content: &str) -> Option<&str> {
        // "Entirely of spaces" is about the space character alone, so a tab
        // between two of them is content and the pair around it does come off.
        if content.bytes().all(|byte| byte == b' ') {
            return None;
        }

        content.strip_prefix(' ')?.strip_suffix(' ')
    }

    /// Whether a code span carries a space this rule should report.
    ///
    /// A pair of spaces `CommonMark` removes is not padding when it is the only
    /// way to write the span: a span that starts or ends with a backtick has to
    /// be padded at *both* ends, because the removal happens only when both are
    /// there. ``` `` ` `` ``` is how `CommonMark` spells a literal backtick, and
    /// reporting it would leave no way to author one at all.
    fn is_padded(content: &str) -> bool {
        let stripped = Self::strip_padding(content);
        let visible = stripped.unwrap_or(content);

        // A space still standing after the removal is padding whatever it sits
        // next to; a removed pair is padding unless it shields a backtick.
        visible.starts_with(char::is_whitespace)
            || visible.ends_with(char::is_whitespace)
            || stripped.is_some_and(|inner| !(inner.starts_with('`') || inner.ends_with('`')))
    }
}

impl RuleLike for MD038 {
    #[inline]
    fn metadata(&self) -> &'static Metadata {
        &Self::METADATA
    }

    #[inline]
    fn check(&self, doc: &Document) -> Result<Vec<Violation>> {
        let mut violations = vec![];

        for node in doc.ast.descendants() {
            if let NodeValue::Code(code) = &node.data.borrow().value {
                let position = node.data.borrow().sourcepos;
                let maybe_content = if position.start.line == position.end.line {
                    Self::same_line_content(code, position)
                } else {
                    Self::multi_line_content(&doc.lines, position, code.num_backticks)
                        .map(Cow::Owned)
                };
                let Some(content) = maybe_content else {
                    continue;
                };

                if Self::is_padded(&content) {
                    let violation = self.to_violation(doc.path.clone(), position);
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
            ` some text `

            `some text `

            ` some text`
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD038::new();
        let actual = rule.check(&doc)?;
        let expected = vec![
            rule.to_violation(path.clone(), Sourcepos::from((1, 1, 1, 13))),
            rule.to_violation(path.clone(), Sourcepos::from((3, 1, 3, 12))),
            rule.to_violation(path, Sourcepos::from((5, 1, 5, 12))),
        ];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_errors_with_multiple_backticks() -> Result<()> {
        let text = indoc! {"
            ``  some text  ``

            ```some text ```
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD038::new();
        let actual = rule.check(&doc)?;
        let expected = vec![
            rule.to_violation(path.clone(), Sourcepos::from((1, 1, 1, 17))),
            rule.to_violation(path, Sourcepos::from((3, 1, 3, 16))),
        ];
        assert_eq!(actual, expected);
        Ok(())
    }

    // The removed pair shields the backtick, but the space it leaves behind is
    // padding all the same.
    #[test]
    fn check_errors_with_padded_backtick_content() -> Result<()> {
        let text = indoc! {"
            ``  ` ``

            `` `  ``
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD038::new();
        let actual = rule.check(&doc)?;
        let expected = vec![
            rule.to_violation(path.clone(), Sourcepos::from((1, 1, 1, 8))),
            rule.to_violation(path, Sourcepos::from((3, 1, 3, 8))),
        ];
        assert_eq!(actual, expected);
        Ok(())
    }

    // Nothing but spaces is left alone by `CommonMark`, so every one of them is
    // visible padding: the span renders as a space and reads as an empty one.
    // Reporting it is deliberate — a span with nothing else in it is a space the
    // author can see and remove.
    #[test]
    fn check_errors_with_only_spaces() -> Result<()> {
        let text = "` `".to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD038::new();
        let actual = rule.check(&doc)?;
        let expected = vec![rule.to_violation(path, Sourcepos::from((1, 1, 1, 3)))];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_no_errors() -> Result<()> {
        let text = "`some text`".to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text)?;
        let rule = MD038::new();
        let actual = rule.check(&doc)?;
        let expected = vec![];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_no_errors_with_multiple_backticks() -> Result<()> {
        let text = indoc! {"
            ``some text``

            ``some `text` here``

            ```some text```
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text)?;
        let rule = MD038::new();
        let actual = rule.check(&doc)?;
        let expected = vec![];
        assert_eq!(actual, expected);
        Ok(())
    }

    // The spaces are the only way to write a span that starts or ends with a
    // backtick, so they are not padding and must not be reported.
    #[test]
    fn check_no_errors_with_backtick_content() -> Result<()> {
        let text = indoc! {"
            `` ` ``

            `` `some text ``

            `` some text` ``
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text)?;
        let rule = MD038::new();
        let actual = rule.check(&doc)?;
        let expected = vec![];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_no_errors_with_multiline_code_span() -> Result<()> {
        let text = indoc! {"
            `some
            text`
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text)?;
        let rule = MD038::new();
        let actual = rule.check(&doc)?;
        let expected = vec![];
        assert_eq!(actual, expected);
        Ok(())
    }

    // A multibyte prefix moves the span's columns without changing its width, so
    // it has to be reported at the columns the multibyte text puts it at.
    #[test]
    fn check_errors_with_multibyte_prefix() -> Result<()> {
        let text = "\u{3042}\u{3044} ` pad ` \u{3046}\u{3048}".to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD038::new();
        let actual = rule.check(&doc)?;
        let expected = vec![rule.to_violation(path, Sourcepos::from((1, 8, 1, 14)))];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_no_errors_with_multibyte_prefix() -> Result<()> {
        let text = "\u{3042}\u{3044} ``some text`` \u{3046}\u{3048}".to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text)?;
        let rule = MD038::new();
        let actual = rule.check(&doc)?;
        let expected = vec![];
        assert_eq!(actual, expected);
        Ok(())
    }

    // comrak unescapes a table cell before parsing its inlines, so `sourcepos`
    // no longer indexes the line the cell was written on. Widths survive that;
    // both spans here are measured, not sliced.
    #[test]
    fn check_errors_with_escaped_pipe_in_table() -> Result<()> {
        let text = indoc! {r"
            | a | b |
            | --- | --- |
            | x\|y\|z ``some text`` | c |
            | x\|y `` some text `` | c |
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD038::new();
        let actual = rule.check(&doc)?;
        let expected = vec![rule.to_violation(path, Sourcepos::from((4, 7, 4, 21)))];
        assert_eq!(actual, expected);
        Ok(())
    }

    // comrak measures an inline against the paragraph's content rather than the
    // file, so once a continuation line has been stripped of its indentation the
    // columns no longer index the line. The delimiters are not where `sourcepos`
    // says they are, and the span goes unchecked rather than being read out of
    // the wrong place — a miss, never a report the author cannot act on.
    #[test]
    fn check_no_errors_with_multiline_code_span_after_indentation() -> Result<()> {
        let text = indoc! {"
            lead in
              indented continuation with ` pad
            text ` end
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text)?;
        let rule = MD038::new();
        let actual = rule.check(&doc)?;
        let expected = vec![];
        assert_eq!(actual, expected);
        Ok(())
    }

    // A line ending inside a code span becomes a space, so padding written
    // against a delimiter is as invisible here as it is on one line.
    #[test]
    fn check_errors_with_multiline_code_span() -> Result<()> {
        let text = indoc! {"
            ` some
            text `
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD038::new();
        let actual = rule.check(&doc)?;
        let expected = vec![rule.to_violation(path, Sourcepos::from((1, 1, 2, 6)))];
        assert_eq!(actual, expected);
        Ok(())
    }
}
