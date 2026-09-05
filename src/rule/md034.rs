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

            // The literal rather than the line, because what a bare URL is is a
            // question about the text a reader is given: a scan of the line
            // stops at a backslash written into an authority, which no
            // authority can hold, and hands back the piece before it as a URL
            // of its own. `CommonMark` has resolved the escape by the time the
            // literal is built, and the authority there is the one a reader
            // sees. What the literal cannot say is where any of it was written,
            // and that is what `written_column_of` is for.
            for link in finder.links(literal) {
                // NOTE: link.start and link.end start from 0
                let mut position = data.sourcepos;
                position.end.line = position.start.line;
                position.start.column =
                    doc.written_column_of(data.sourcepos, literal, link.start());

                // The URL's last byte rather than the one after it, stepped
                // past once it is on the line: a column is answered for by the
                // byte the literal has at it, and the byte after the URL is not
                // the URL's. A `\|` written there would be one the walk stops
                // at, and the end would follow it past the URL.
                position.end.column =
                    doc.written_column_of(data.sourcepos, literal, link.end() - 1) + 1;

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

    // The column the rule names is the byte after the URL, and here that byte
    // is the backslash of an escape. A column is answered for by the byte the
    // literal has at it, and the byte after the URL is not the URL's: the walk
    // stops at that escape, and the end would follow it past the URL.
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
        let expected = vec![rule.to_violation(path, Sourcepos::from((3, 5, 3, 28)))];
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

    // An escape resolved out of an authority can leave one no URL is written
    // with — an underscore in either of the last two labels is not a domain to
    // GFM, and is not one here — and the literal is where that can be seen. A
    // scan of the line stops at the backslash instead, no authority being able
    // to hold one, and hands back the `http://ex` before it as a URL of its
    // own.
    #[test]
    fn check_no_errors_with_escaped_authority() -> Result<()> {
        let text = "see http://my\\_site.com/ now".to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text)?;
        let rule = MD034::default();
        let actual = rule.check(&doc)?;
        let expected = vec![];
        assert_eq!(actual, expected);
        Ok(())
    }

    // And the piece a scan of the line hands back is a prefix of any URL that
    // shares it, so a second URL on the line is enough to make one of these
    // look like a URL that was written.
    #[test]
    fn check_errors_with_escaped_authority_beside_a_url() -> Result<()> {
        let text = "see http://ex\\_ample.com/ and http://ex.com now".to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD034::default();
        let actual = rule.check(&doc)?;
        let expected = vec![rule.to_violation(path, Sourcepos::from((1, 31, 1, 44)))];
        assert_eq!(actual, expected);
        Ok(())
    }

    // An escape resolved out of an authority that leaves a domain leaves a URL,
    // and GFM autolinks this one whole. The rule reports the whole of it: the
    // literal holds it whole, and the walk puts each end back on the line.
    #[test]
    fn check_errors_with_escaped_authority_that_resolves() -> Result<()> {
        let text = "see http://ex\\-ample.com/ now".to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD034::default();
        let actual = rule.check(&doc)?;
        let expected = vec![rule.to_violation(path, Sourcepos::from((1, 5, 1, 26)))];
        assert_eq!(actual, expected);
        Ok(())
    }

    // An escape written into the path is resolved out of the literal like any
    // other, and the walk puts its two columns back.
    #[test]
    fn check_errors_with_escaped_path() -> Result<()> {
        let text = "see http://www.example.com/foo\\_bar now".to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD034::default();
        let actual = rule.check(&doc)?;
        let expected = vec![rule.to_violation(path, Sourcepos::from((1, 5, 1, 36)))];
        assert_eq!(actual, expected);
        Ok(())
    }
}
