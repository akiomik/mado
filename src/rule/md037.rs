use std::sync::LazyLock;

use comrak::nodes::NodeValue;
use miette::Result;
use regex::Regex;

use crate::{Document, violation::Violation};

use super::{Metadata, RuleLike, Tag};

#[derive(Debug, Default, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct MD037;

impl MD037 {
    const METADATA: Metadata = Metadata {
        name: "MD037",
        description: "Spaces inside emphasis markers",
        tags: &[Tag::Whitespace, Tag::Emphasis],
        aliases: &["no-space-in-emphasis"],
    };

    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        Self {}
    }
}

impl RuleLike for MD037 {
    #[inline]
    fn metadata(&self) -> &'static Metadata {
        &Self::METADATA
    }

    #[inline]
    fn check(&self, doc: &Document) -> Result<Vec<Violation>> {
        static RE: LazyLock<Regex> = LazyLock::new(|| {
            #[allow(clippy::unwrap_used)]
            Regex::new(r"(?:\s\*\s.+\*)|(?:\s\*\*\s.+\*\*)|(?:\s_\s.+_)|(?:\s__\s.+__)|(?:\*.+\s\*\s)|(?:\*\*.+\s\*\*\s)|(?:_.+\s_\s)|(?:__.+\s__\s)").unwrap()
        });

        let mut violations = vec![];

        for node in doc.ast.descendants() {
            let data = node.data.borrow();
            let NodeValue::Text(literal) = &data.value else {
                continue;
            };

            // The line the node was written on rather than its literal, whose
            // offsets stop naming columns as soon as an escape is resolved out
            // of it, and with the escapes on that line masked out: a marker the
            // author escaped is not one, and this regex looks for markers in
            // places that have nothing before them for it to check.
            let (text, column) = doc.written_text_without_escapes(data.sourcepos, literal);

            let Some(m) = RE.find(&text) else {
                continue;
            };

            let mut position = data.sourcepos;

            // The offsets are counted off one line, so the span is that line's.
            // comrak ends a text node on another only where the position is one
            // `written_text_without_escapes` cannot read the line for, and the
            // offsets are the start line's there too.
            position.end.line = position.start.line;

            // NOTE: m.start and m.end start from 0, and count off `column`,
            //       which is where the text starts on the line.
            //
            // The alternatives that begin at a start marker begin at the
            // whitespace before it, and the ones that begin at an end marker
            // begin at the marker, so what the match begins with says which
            // matched. `\s` is any whitespace and not the space alone, so it is
            // asked about as such — a tab answered for the end-marker
            // arithmetic and put the report on the tab rather than on the
            // marker — and the marker is that character's width along, which is
            // two bytes for a no-break space and three for an ideographic one.
            if let Some(space) = m
                .as_str()
                .chars()
                .next()
                .filter(|char| char.is_whitespace())
            {
                // When a start marker matches
                position.start.column = column + m.start() + space.len_utf8();
                position.end.column = column + m.end() - 1;
            } else {
                // When an end marker matches
                position.start.column = column + m.start();
                position.end.column = column + m.end() - 2;
            }

            let violation = self.to_violation(doc.path.clone(), position);
            violations.push(violation);
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
            Here is some ** bold ** text.

            Here is some * italic * text.

            Here is some more __ bold __ text.

            Here is some more _ italic _ text.
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD037::new();
        let actual = rule.check(&doc)?;
        let expected = vec![
            rule.to_violation(path.clone(), Sourcepos::from((1, 14, 1, 23))),
            rule.to_violation(path.clone(), Sourcepos::from((3, 14, 3, 23))),
            rule.to_violation(path.clone(), Sourcepos::from((5, 19, 5, 28))),
            rule.to_violation(path, Sourcepos::from((7, 19, 7, 28))),
        ];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_errors_with_space() -> Result<()> {
        let text = indoc! {"
            Here is some **bold ** text.

            Here is some * italic* text.

            Here is some more __bold __ text.

            Here is some more _ italic_ text.
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD037::new();
        let actual = rule.check(&doc)?;
        let expected = vec![
            rule.to_violation(path.clone(), Sourcepos::from((1, 14, 1, 22))),
            rule.to_violation(path.clone(), Sourcepos::from((3, 14, 3, 22))),
            rule.to_violation(path.clone(), Sourcepos::from((5, 19, 5, 27))),
            rule.to_violation(path, Sourcepos::from((7, 19, 7, 27))),
        ];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_no_errors() -> Result<()> {
        let text = indoc! {"
            Here is some **bold** text.

            Here is some *italic* text.

            Here is some more __bold__ text.

            Here is some more _italic_ text.
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text)?;
        let rule = MD037::new();
        let actual = rule.check(&doc)?;
        let expected = vec![];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_no_errors_nested() -> Result<()> {
        let text = indoc! {"
            Here is ** some **bold** text ** .

            Here is * some *italic* text * .

            Here is some __ more __bold__ text __ .

            Here is some _ more _italic_ text _ .
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text)?;
        let rule = MD037::new();
        let actual = rule.check(&doc)?;
        let expected = vec![];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_no_errors_with_emoji() -> Result<()> {
        let text = "This is an emoji :white_check_mark:".to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text)?;
        let rule = MD037::new();
        let actual = rule.check(&doc)?;
        let expected = vec![];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_no_errors_start_marker() -> Result<()> {
        let text = indoc! {"
            Here is some **bold **text.

            Here is some *italic *text.

            Here is some more __bold __text.

            Here is some more _italic _text.
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text)?;
        let rule = MD037::new();
        let actual = rule.check(&doc)?;
        let expected = vec![];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_no_errors_end_marker() -> Result<()> {
        let text = indoc! {"
            Here is some** bold** text.

            Here is some* italic* text.

            Here is some more__ bold__ text.

            Here is some more_ italic_ text.
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text)?;
        let rule = MD037::new();
        let actual = rule.check(&doc)?;
        let expected = vec![];
        assert_eq!(actual, expected);
        Ok(())
    }

    // comrak unescapes a table cell before parsing its inlines, so the columns
    // it reports from inside one are short a byte for every `\|` written before
    // them. The rule adds its own offsets on top of a start that has already
    // been shifted, and both are put back together.
    #[test]
    fn check_errors_with_escaped_pipe_in_table() -> Result<()> {
        let text = indoc! {r"
            | a | b |
            | --- | --- |
            | x\|y ** b ** | c |
            | x\|y\|z ** b ** | c |
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD037::new();
        let actual = rule.check(&doc)?;
        let expected = vec![
            rule.to_violation(path.clone(), Sourcepos::from((3, 8, 3, 14))),
            rule.to_violation(path, Sourcepos::from((4, 11, 4, 17))),
        ];
        assert_eq!(actual, expected);
        Ok(())
    }

    // A backslash escape is resolved before the text node's literal is built,
    // so the literal is a byte shorter than the line for each one and the
    // marker's offset in it names a column to the left of where it was written.
    // The line is measured instead, and the escape keeps its two columns.
    #[test]
    fn check_errors_with_escaped_punctuation() -> Result<()> {
        let text = indoc! {r"
            x \. y ** b ** z

            | a | b |
            | --- | --- |
            | x \. y ** b ** | c |
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD037::new();
        let actual = rule.check(&doc)?;
        let expected = vec![
            rule.to_violation(path.clone(), Sourcepos::from((1, 8, 1, 14))),
            rule.to_violation(path, Sourcepos::from((5, 10, 5, 16))),
        ];
        assert_eq!(actual, expected);
        Ok(())
    }

    // Outside a table cell and the paragraph comrak splits off a header row,
    // `\|` is resolved by the inline parser like any other escape, so it costs
    // the literal a byte there rather than shifting the columns comrak reports.
    #[test]
    fn check_errors_with_escaped_pipe_outside_table() -> Result<()> {
        let text = "see x\\|y\\|z ** b ** w".to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD037::new();
        let actual = rule.check(&doc)?;
        let expected = vec![rule.to_violation(path, Sourcepos::from((1, 13, 1, 19)))];
        assert_eq!(actual, expected);
        Ok(())
    }

    // An escaped marker reaches the literal as a bare one, so a match could
    // begin at a marker that is not one and name a column that holds neither of
    // the markers on the line. The line keeps the backslash that guards it, and
    // the match begins at the emphasis that is really there.
    #[test]
    fn check_errors_with_escaped_marker() -> Result<()> {
        let text = "x \\* y ** b ** z".to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD037::new();
        let actual = rule.check(&doc)?;
        let expected = vec![rule.to_violation(path, Sourcepos::from((1, 8, 1, 14)))];
        assert_eq!(actual, expected);
        Ok(())
    }

    // Emphasis an author escaped is text, and a pair of escaped markers is not
    // emphasis with spaces inside it. Against the literal both escapes were
    // gone and the pair read as a violation the document does not have.
    //
    // The escapes are masked out of the search rather than guarded against
    // inside it, because the regex looks for a marker in four places that have
    // nothing before them to check: the closing marker of each start-marker
    // alternative, and the opening marker of each end-marker one. A backslash
    // left on the line is also a byte for `.+` to match, so the search reaches
    // further along the line than the literal ever let it.
    #[test]
    fn check_no_errors_with_escaped_markers() -> Result<()> {
        let text = indoc! {r"
            x \* y \* z

            a b \_ * \*

            a \* b * c

            a * b \*
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text)?;
        let rule = MD037::new();
        let actual = rule.check(&doc)?;
        let expected = vec![];
        assert_eq!(actual, expected);
        Ok(())
    }

    // The regex anchors a start marker to `\s`, which is a tab as much as a
    // space, and the report belongs on the marker either way.
    #[test]
    fn check_errors_with_tab_before_marker() -> Result<()> {
        let text = "x\t** b ** y".to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD037::new();
        let actual = rule.check(&doc)?;
        let expected = vec![rule.to_violation(path, Sourcepos::from((1, 3, 1, 9)))];
        assert_eq!(actual, expected);
        Ok(())
    }

    // `\s` is whatever is whitespace and not what is one byte of it, so the
    // marker is that character's width along rather than one: a no-break space
    // is two bytes, and a column one past its first is inside it and not a
    // column of the line at all.
    #[test]
    fn check_errors_with_multibyte_space_before_marker() -> Result<()> {
        let text = "x\u{a0}** b ** y".to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD037::new();
        let actual = rule.check(&doc)?;
        let expected = vec![rule.to_violation(path, Sourcepos::from((1, 4, 1, 10)))];
        assert_eq!(actual, expected);
        Ok(())
    }
}
