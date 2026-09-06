use comrak::nodes::{AstNode, NodeValue, Sourcepos};
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

    /// Whether `node` is a link written as nothing but the URL itself.
    ///
    /// A bare URL and an autolink reach the tree as the same node: comrak
    /// gives both a link holding one text child, and the URL is the text
    /// either way. What tells them apart is how much of the line the link
    /// covers. `<http://example.com>` and `[text](http://example.com)` are
    /// written with something around the text and so span more of it than
    /// their text does, and comrak measures the text inside the wrapper. A
    /// bare URL has no wrapper to measure, and comrak says so by giving the
    /// text the link's own position.
    fn is_bare(node: &AstNode<'_>, position: Sourcepos) -> bool {
        node.first_child().is_some_and(|text| {
            let data = text.data.borrow();
            matches!(data.value, NodeValue::Text(_)) && data.sourcepos == position
        })
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

        for node in doc.autolink_ast.descendants() {
            let data = node.data.borrow();
            if !matches!(data.value, NodeValue::Link(_)) || !Self::is_bare(node, data.sourcepos) {
                continue;
            }

            // #405's correction, a position from inside a table cell being
            // measured against the unescaped cell rather than against the line.
            let mut position = doc.written_position(data.sourcepos);

            // The byte after the URL's last, which is the column reported, and
            // asked for as that rather than as the last byte's column stepped
            // past: `written_position` answers for a column by the character
            // the line has at it, and the byte after the URL is not the URL's.
            position.end.column += 1;

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

    // A link with nothing in it has no text to measure against, and nothing
    // written bare either.
    #[test]
    fn check_no_errors_with_empty_link() -> Result<()> {
        let text = "For more information, see [](http://www.example.com/).".to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text)?;
        let rule = MD034::default();
        let actual = rule.check(&doc)?;
        let expected = vec![];
        assert_eq!(actual, expected);
        Ok(())
    }

    // Nor is a link whose text is not text at all: an autolink's child is the
    // URL and nothing else, and emphasis is something else.
    #[test]
    fn check_no_errors_with_emphasized_link_text() -> Result<()> {
        let text = "For more information, see [*x*](http://www.example.com/).".to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text)?;
        let rule = MD034::default();
        let actual = rule.check(&doc)?;
        let expected = vec![];
        assert_eq!(actual, expected);
        Ok(())
    }

    // An escape written into the scheme is how a URL is spelled so that it is
    // not autolinked: the parser reads the backslash before it can reach the
    // `://`, and what a reader is handed is the URL as text. Nothing is linked,
    // and the rule is about text a reader is handed a link to.
    #[test]
    fn check_no_errors_with_escaped_scheme() -> Result<()> {
        let text = r"For more information, see http\://www.example.com/.".to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text)?;
        let rule = MD034::default();
        let actual = rule.check(&doc)?;
        let expected = vec![];
        assert_eq!(actual, expected);
        Ok(())
    }

    // A domain is segments separated by periods and GFM asks for at least one,
    // so a host with no period in it is not one it autolinks, however much of a
    // URL it is to a scanner.
    #[test]
    fn check_no_errors_with_domain_without_period() -> Result<()> {
        let text = "For more information, see http://localhost/x.".to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text)?;
        let rule = MD034::default();
        let actual = rule.check(&doc)?;
        let expected = vec![];
        assert_eq!(actual, expected);
        Ok(())
    }

    // A scheme is not what makes a bare URL: GFM autolinks a `www.` host
    // without one, and a reader is handed the same link either way.
    #[test]
    fn check_errors_with_www() -> Result<()> {
        let text = "For more information, see www.example.com.".to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD034::default();
        let actual = rule.check(&doc)?;
        let expected = vec![rule.to_violation(path, Sourcepos::from((1, 27, 1, 42)))];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_errors_with_email() -> Result<()> {
        let text = "For more information, mail foo@example.com.".to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD034::default();
        let actual = rule.check(&doc)?;
        let expected = vec![rule.to_violation(path, Sourcepos::from((1, 28, 1, 43)))];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_no_errors_with_email_in_brackets() -> Result<()> {
        let text = "For more information, mail <foo@example.com>.".to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text)?;
        let rule = MD034::default();
        let actual = rule.check(&doc)?;
        let expected = vec![];
        assert_eq!(actual, expected);
        Ok(())
    }

    // GFM autolinks nothing inside square brackets, a shortcut link being what
    // this could be, so there is no link here for a reader to be handed.
    #[test]
    fn check_no_errors_with_url_in_square_brackets() -> Result<()> {
        let text = "For more information, see [http://www.example.com/].".to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text)?;
        let rule = MD034::default();
        let actual = rule.check(&doc)?;
        let expected = vec![];
        assert_eq!(actual, expected);
        Ok(())
    }

    // GFM autolinks `http://`, `https://` and `ftp://` and no other scheme, so
    // a URL written with one of the rest is text a reader is handed as text.
    #[test]
    fn check_no_errors_with_scheme_gfm_does_not_autolink() -> Result<()> {
        let text = indoc! {"
            see ftps://www.example.com/ now

            see file:///tmp/x now

            see ssh://www.example.com/ now
        "}
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

    // `ftp://` is one of the three, and is reported.
    #[test]
    fn check_errors_with_ftp() -> Result<()> {
        let text = "For more information, see ftp://www.example.com/.".to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD034::default();
        let actual = rule.check(&doc)?;
        let expected = vec![rule.to_violation(path, Sourcepos::from((1, 27, 1, 49)))];
        assert_eq!(actual, expected);
        Ok(())
    }

    // GFM starts an email autolink at the `mailto:` rather than at the address,
    // and the whole of what it links is what is reported.
    #[test]
    fn check_errors_with_mailto_email() -> Result<()> {
        let text = "For more information, mail mailto:foo@example.com.".to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD034::default();
        let actual = rule.check(&doc)?;
        let expected = vec![rule.to_violation(path, Sourcepos::from((1, 28, 1, 50)))];
        assert_eq!(actual, expected);
        Ok(())
    }

    // comrak refuses an autolink inside brackets and counts its way out of them
    // on any `]`, so the URL after the `]` of a nested pair is inside link text
    // and autolinked both — and the autolink runs to the next space, taking the
    // `](y)` that would have closed the link with it. What is reported is the
    // link GFM makes, whether or not the document has another URL elsewhere.
    #[test]
    fn check_errors_with_bare_url_after_nested_brackets() -> Result<()> {
        let text = "see [a [b] http://x.example.com/](y) now".to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD034::default();
        let actual = rule.check(&doc)?;
        let expected = vec![rule.to_violation(path, Sourcepos::from((1, 12, 1, 37)))];
        assert_eq!(actual, expected);
        Ok(())
    }

    // An image's alt text is square brackets too, and GFM autolinks nothing
    // inside them: the alt text of the rendered image is the URL as text, and
    // no reader is handed a link to it.
    #[test]
    fn check_no_errors_with_url_in_image_alt_text() -> Result<()> {
        let text = "For more information, see ![x http://www.example.com/](y.png).".to_owned();
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
    // them. `written_position` puts those bytes back, and the link is reported
    // at the columns it was written at.
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

    // A cell is unescaped before its inlines are parsed, so the `|` the author
    // escaped is a byte of the URL by the time GFM autolinks one: the link is
    // `http://www.example.com/|y`, and the column after it is the one after the
    // `y`. Both ends are put back on the line, where the escape is two columns.
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
        let expected = vec![rule.to_violation(path, Sourcepos::from((3, 5, 3, 31)))];
        assert_eq!(actual, expected);
        Ok(())
    }

    // An escape written before the URL is resolved out of the text node's
    // literal, but not out of the line comrak measures the link against, so the
    // two columns it was written with are both still there.
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

    // GFM reads a domain past the backslash of an escape and counts the byte it
    // guards, so this one holds an underscore in the last two labels, which is
    // not a domain. Nothing here is autolinked, and nothing is reported.
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

    // And a line carrying one of those beside a URL GFM does autolink is
    // reported for the second alone.
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

    // A hyphen is a domain character where an underscore is not, so the same
    // escape leaves a domain here. GFM autolinks the URL whole — the backslash
    // is linked along with the rest of it — and the whole of it is reported.
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

    // GFM trims a trailing `_` off an autolink, and it is the written text it
    // trims: the `_` here is the byte an escape guards, so the link ends on the
    // backslash before it and the `_` is text. The column after the link is the
    // one the `_` is at, which is the second of the escape's two.
    #[test]
    fn check_errors_with_escape_at_end_of_url() -> Result<()> {
        let text = "see http://www.example.com/foo\\_ now".to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD034::default();
        let actual = rule.check(&doc)?;
        let expected = vec![rule.to_violation(path, Sourcepos::from((1, 5, 1, 32)))];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_errors_with_multibyte_at_end_of_url() -> Result<()> {
        let text = "see http://www.example.com/f\u{e9}\u{e9} now".to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD034::default();
        let actual = rule.check(&doc)?;
        let expected = vec![rule.to_violation(path, Sourcepos::from((1, 5, 1, 33)))];
        assert_eq!(actual, expected);
        Ok(())
    }

    // An escaped backslash before the URL is two columns of the line and one
    // byte of the literal, and the link is measured against the line.
    #[test]
    fn check_errors_with_escape_at_start_of_node() -> Result<()> {
        let text = "\\\\.x http://www.example.com/ y".to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD034::default();
        let actual = rule.check(&doc)?;
        let expected = vec![rule.to_violation(path, Sourcepos::from((1, 6, 1, 29)))];
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
