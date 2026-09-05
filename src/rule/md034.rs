use comrak::nodes::NodeValue;
use linkify::LinkFinder;
use miette::Result;

use crate::{Document, violation::Violation};

use super::{Metadata, RuleLike, Tag};

#[derive(Debug, Default, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct MD034;

impl MD034 {
    const METADATA: Metadata = Metadata {
        name: "MD034",
        description: "Bare URL used",
        tags: &[Tag::Links, Tag::Url],
        aliases: &["no-bare-urls"],
    };

    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        Self {}
    }
}

impl RuleLike for MD034 {
    #[inline]
    fn metadata(&self) -> &'static Metadata {
        &Self::METADATA
    }

    #[inline]
    fn check(&self, doc: &Document) -> Result<Vec<Violation>> {
        let mut violations = vec![];
        let finder = LinkFinder::new();

        for node in doc.ast.descendants() {
            let data = node.data.borrow();
            let NodeValue::Text(literal) = &data.value else {
                continue;
            };

            // A URL inside a link is already linked.
            if let Some(parent) = node.parent()
                && let NodeValue::Link(_) = parent.data.borrow().value
            {
                continue;
            }

            // The line the node was written on rather than its literal, whose
            // offsets stop naming columns as soon as an escape is resolved out
            // of it. The backslash of an escape is on the line and belongs
            // there: GFM reads one written into a URL as the URL's own rather
            // than as an escape, and a URL whose scheme carries one it does not
            // autolink at all.
            let (text, column) = doc.written_text(data.sourcepos, literal);

            for link in finder.links(text) {
                // NOTE: link.start and link.end start from 0, and count off
                //       `column`, which is where the text starts on the line.
                let mut position = data.sourcepos;

                // The offsets are counted off one line, so the span is that
                // line's. comrak ends a text node on another only where the
                // position is one `written_text` cannot read the line for, and
                // the offsets are the start line's there too.
                position.end.line = position.start.line;
                position.start.column = column + link.start();

                // The byte after the URL's last, which is the column reported.
                position.end.column = column + link.end();

                let violation = self.to_violation(doc.path.clone(), position);
                violations.push(violation);
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
        let text = "For more information, see http://www.example.com/.".to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD034::default();
        let actual = rule.check(&doc)?;
        let expected = vec![rule.to_violation(path, Sourcepos::from((1, 27, 1, 50)))];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_no_errors_with_brackets() -> Result<()> {
        let text = "For more information, see <http://www.example.com/>.".to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text)?;
        let rule = MD034::default();
        let actual = rule.check(&doc)?;
        let expected = vec![];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_no_errors_with_link() -> Result<()> {
        let text = "For more information, see [http://www.example.com/](http://www.example.com/)."
            .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text)?;
        let rule = MD034::default();
        let actual = rule.check(&doc)?;
        let expected = vec![];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_no_errors_with_code() -> Result<()> {
        let text = "For more information, see `http://www.example.com/`.".to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text)?;
        let rule = MD034::default();
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
            | x\|y http://www.example.com/ | c |
            | x\|y\|z http://www.example.com/ | c |
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD034::default();
        let actual = rule.check(&doc)?;
        let expected = vec![
            rule.to_violation(path.clone(), Sourcepos::from((3, 8, 3, 31))),
            rule.to_violation(path, Sourcepos::from((4, 11, 4, 34))),
        ];
        assert_eq!(actual, expected);
        Ok(())
    }

    // The rule names the byte after the URL's last, and a URL that runs to the
    // end of its cell puts that past the columns comrak reports for the cell.
    #[test]
    fn check_errors_with_escaped_pipe_at_end_of_table_cell() -> Result<()> {
        let text = indoc! {r"
            | a | b |
            | --- | --- |
            |x\|y http://www.example.com/| c |
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD034::default();
        let actual = rule.check(&doc)?;
        let expected = vec![rule.to_violation(path, Sourcepos::from((3, 7, 3, 30)))];
        assert_eq!(actual, expected);
        Ok(())
    }

    // A backslash directly after a URL is a byte of the line like any other,
    // and the URL scanner reads it as the URL's own rather than stopping at it.
    // The column reported is the byte after the last one that scanner took,
    // which is its own boundary and not GFM's: GFM unescapes the cell first and
    // autolinks on through the `|y`.
    #[test]
    fn check_errors_with_escaped_pipe_after_url_in_table_cell() -> Result<()> {
        let text = indoc! {r"
            | a | b |
            | --- | --- |
            | x http://www.example.com/\|y |
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD034::default();
        let actual = rule.check(&doc)?;
        let expected = vec![rule.to_violation(path, Sourcepos::from((3, 5, 3, 29)))];
        assert_eq!(actual, expected);
        Ok(())
    }

    // A backslash escape is resolved before the text node's literal is built,
    // so the literal is a byte shorter than the line for each one and the URL's
    // offset in it names a column to the left of where it was written. The line
    // is measured instead, and the escape keeps its two columns.
    #[test]
    fn check_errors_with_escaped_punctuation() -> Result<()> {
        let text = indoc! {r"
            x \. y http://www.example.com/ z
            x \. y\. z http://www.example.com/ w
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD034::default();
        let actual = rule.check(&doc)?;
        let expected = vec![
            rule.to_violation(path.clone(), Sourcepos::from((1, 8, 1, 31))),
            rule.to_violation(path, Sourcepos::from((2, 12, 2, 35))),
        ];
        assert_eq!(actual, expected);
        Ok(())
    }

    // Outside a table cell and the paragraph comrak splits off a header row,
    // `\|` is resolved by the inline parser like any other escape, so it costs
    // the literal a byte there rather than shifting the columns comrak reports.
    #[test]
    fn check_errors_with_escaped_pipe_outside_table() -> Result<()> {
        let text = "see x\\|y http://www.example.com/".to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD034::default();
        let actual = rule.check(&doc)?;
        let expected = vec![rule.to_violation(path, Sourcepos::from((1, 10, 1, 33)))];
        assert_eq!(actual, expected);
        Ok(())
    }

    // An escape in the scheme is how a URL is written so that it is not
    // autolinked, and GFM leaves this one alone. The literal has the escape
    // resolved out of it and reads as a bare URL, so measuring against it
    // reported a URL the author had already stopped from becoming one.
    #[test]
    fn check_no_errors_with_escaped_scheme() -> Result<()> {
        let text = "For more information, see http\\://www.example.com/.".to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text)?;
        let rule = MD034::default();
        let actual = rule.check(&doc)?;
        let expected = vec![];
        assert_eq!(actual, expected);
        Ok(())
    }
}
