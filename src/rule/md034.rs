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

    // TODO: Use safe casting
    #[inline]
    #[allow(clippy::cast_possible_wrap)]
    fn check(&self, doc: &Document) -> Result<Vec<Violation>> {
        let mut violations = vec![];
        let finder = LinkFinder::new();

        for node in doc.ast.descendants() {
            if let NodeValue::Text(text) = &node.data.borrow().value {
                for link in finder.links(text) {
                    if let Some(parent) = node.parent()
                        && let NodeValue::Link(_) = parent.data.borrow().value
                    {
                        continue;
                    }

                    // NOTE: link.start and link.end start from 0
                    let mut position = node.data.borrow().sourcepos;
                    position.end = position.start.column_add(link.end() as isize);
                    position.start = position.start.column_add(link.start() as isize);

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
}
