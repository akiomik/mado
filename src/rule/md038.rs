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
    /// Only a span whose delimiters share a line has one. A span that crosses a
    /// line ending is skipped, and deliberately: nothing available describes its
    /// content. `literal` has already turned the line endings into spaces and
    /// taken the padding off, a width cannot be had from columns that index two
    /// different lines, and reading the lines back fails four ways that were all
    /// found in real documents — the columns shift once a continuation line is
    /// stripped of its indentation, the lines between the delimiters are not part
    /// of any slice of the two ends, a container's `>` or list marker lands
    /// inside the slice, and a span whose ends are both empty rebuilds as a
    /// single space that reads as padding. Reporting under any of those is a
    /// violation its author cannot act on, which is the defect this rule was
    /// fixed for, so the span goes unjudged instead.
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
    /// [`Document::written_position`] undoes the shift, but only for the report:
    /// the columns handed here still describe the unescaped content, which is
    /// the string `literal` came from and so the one to measure against.
    ///
    /// `None` means the columns do not describe a span at all, and the caller
    /// then skips it: a position we cannot measure is not evidence of a
    /// violation.
    fn content(code: &NodeCode, position: Sourcepos) -> Option<Cow<'_, str>> {
        if position.start.line != position.end.line {
            return None;
        }

        let width = (position.end.column + 1)
            .checked_sub(position.start.column)?
            .checked_sub(2 * code.num_backticks)?;

        // `CommonMark` takes a space off each end or nothing at all, so the width
        // tells those two apart on its own. A width it cannot have produced is
        // not a third case to reject: `literal` is then judged as it stands,
        // which reports what a reader can see and nothing else.
        if width == code.literal.len() + 2 {
            return Some(Cow::Owned(format!(" {} ", code.literal)));
        }

        Some(Cow::Borrowed(&code.literal))
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
    ///
    /// The exception is the removal, not the backtick. A space that survives it
    /// is in the output, and being next to a backtick does not make it any less
    /// visible: ``` `` ` a`` ``` renders as `` ` a`` with the space, because the
    /// other end has none for `CommonMark` to pair it with. That is reported,
    /// and the fix is to give it one — ``` `` ` a `` ``` renders as `` `a`` —
    /// rather than to delete a space the span cannot be written without.
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
                let Some(content) = Self::content(code, position) else {
                    continue;
                };

                if Self::is_padded(&content) {
                    // `content` is measured against the columns comrak reports;
                    // only the report is put back on the line as written.
                    let violation =
                        self.to_violation(doc.path.clone(), doc.written_position(position));
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

    // Only the removed pair is invisible. A space against a backtick that
    // `CommonMark` leaves alone is in the output, and reporting it is actionable:
    // the third span here is the first two with the missing space supplied, and
    // it renders without either.
    #[test]
    fn check_errors_with_unbalanced_backtick_content() -> Result<()> {
        let text = indoc! {"
            `` ` embedded``

            ``embedded ` ``

            `` ` embedded ` ``
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD038::new();
        let actual = rule.check(&doc)?;
        let expected = vec![
            rule.to_violation(path.clone(), Sourcepos::from((1, 1, 1, 15))),
            rule.to_violation(path, Sourcepos::from((3, 1, 3, 15))),
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

    // Nothing but spaces is left alone by `CommonMark`, so nothing is removed and
    // every space is as written.
    //
    // NOTE: this sits awkwardly beside the backtick case above. A span of one
    // space is the only way to write a code span that renders as one, so the
    // report cannot be acted on without deleting the span — the shape this rule
    // was fixed for. It is reported anyway, because that is what mado has always
    // done here and narrowing it is a decision of its own, not a bug fix.
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
    // both spans here are measured, not sliced. The report does not survive it,
    // and `Document::written_position` is what puts it back: the backtick the
    // column below names is at column 8, one to the right of where comrak has
    // it because of the single `\|` before it.
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
        let expected = vec![rule.to_violation(path, Sourcepos::from((4, 8, 4, 22)))];
        assert_eq!(actual, expected);
        Ok(())
    }

    // Two escapes shift the span by two, and one written inside the span shifts
    // its end one further than its start. The width the rule measures is taken
    // before any of that is undone, so it still describes the unescaped content.
    #[test]
    fn check_errors_with_escaped_pipe_inside_code_span_in_table() -> Result<()> {
        let text = indoc! {r"
            | a | b |
            | --- | --- |
            | x\|y\|z `` a\|b `` | c |
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD038::new();
        let actual = rule.check(&doc)?;
        let expected = vec![rule.to_violation(path, Sourcepos::from((3, 11, 3, 20)))];
        assert_eq!(actual, expected);
        Ok(())
    }

    // A span that crosses a line ending is not judged, so padding written against
    // one of its delimiters is missed. That is the accepted cost of not reporting
    // the three shapes below, none of which its author could act on. #404 covers
    // the part of this that `literal` can still prove on its own.
    #[test]
    fn check_no_errors_with_padded_multiline_code_span() -> Result<()> {
        let text = indoc! {"
            ` some
            text `
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

    // The content lives on the line between the delimiters, which no slice of
    // the two end lines contains. Both ends are empty here, and a reconstruction
    // from them alone is a single space that reads as padding — on a span whose
    // spaces are the only way to write it.
    #[test]
    fn check_no_errors_with_multiline_code_span_around_content() -> Result<()> {
        let text = indoc! {"
            ``
            `x`
            ``
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

    // A blockquote's `>` sits between the start of the line and the closing
    // delimiter, so it lands inside any slice taken up to that column.
    #[test]
    fn check_no_errors_with_multiline_code_span_in_blockquote() -> Result<()> {
        let text = indoc! {"
            > `text
            >`
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

    // comrak measures an inline against the paragraph's content rather than the
    // file, so once a continuation line has been stripped of its indentation the
    // columns no longer index the line they are supposed to.
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

    // comrak unescapes the paragraph it splits off a table's header row too, so
    // a span written above a table is shifted like one written in it.
    #[test]
    fn check_errors_with_escaped_pipe_in_table_header_preface() -> Result<()> {
        let text = indoc! {r"
            text x\|y `` some text `` here
            | a | b |
            | --- | --- |
            | c | d |
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD038::new();
        let actual = rule.check(&doc)?;
        let expected = vec![rule.to_violation(path, Sourcepos::from((1, 11, 1, 25)))];
        assert_eq!(actual, expected);
        Ok(())
    }
}
