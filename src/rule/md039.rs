use comrak::nodes::NodeValue;
use miette::Result;

use crate::{Document, violation::Violation};

use super::{Metadata, RuleLike, Tag};

#[derive(Debug, Default, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct MD039;

impl MD039 {
    const METADATA: Metadata = Metadata {
        name: "MD039",
        description: "Spaces inside link text",
        tags: &[Tag::Whitespace, Tag::Links],
        aliases: &["no-space-in-links"],
    };

    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        Self {}
    }
}

impl RuleLike for MD039 {
    #[inline]
    fn metadata(&self) -> &'static Metadata {
        &Self::METADATA
    }

    #[inline]
    fn check(&self, doc: &Document) -> Result<Vec<Violation>> {
        let mut violations = vec![];

        for node in doc.ast.descendants() {
            if let NodeValue::Link(_) = node.data.borrow().value {
                if let Some(text_node) = node.first_child()
                    && let NodeValue::Text(text) = &text_node.data.borrow().value
                    && text.trim_start() != text
                {
                    let position = text_node.data.borrow().sourcepos;
                    let violation =
                        self.to_violation(doc.path.clone(), doc.written_position(position));
                    violations.push(violation);
                    continue;
                }

                if let Some(text_node) = node.last_child()
                    && let NodeValue::Text(text) = &text_node.data.borrow().value
                    && text.trim_end() != text
                {
                    let position = text_node.data.borrow().sourcepos;
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
            [ a link ](http://www.example.com/)
            [a link ](http://www.example.com/)
            [ a link](http://www.example.com/)
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD039::new();
        let actual = rule.check(&doc)?;
        let expected = vec![
            rule.to_violation(path.clone(), Sourcepos::from((1, 2, 1, 9))),
            rule.to_violation(path.clone(), Sourcepos::from((2, 2, 2, 8))),
            rule.to_violation(path, Sourcepos::from((3, 2, 3, 8))),
        ];
        assert_eq!(actual, expected);
        Ok(())
    }

    // NOTE: These results may differ from mdl
    #[test]
    fn check_errors_code() -> Result<()> {
        let text = indoc! {"
            [ a `link` ](http://www.example.com/)
            [ `link` ](http://www.example.com)
            [`link` ](http://www.example.com)
            [ `link`](http://www.example.com)
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD039::new();
        let actual = rule.check(&doc)?;
        let expected = vec![
            rule.to_violation(path.clone(), Sourcepos::from((1, 2, 1, 4))),
            rule.to_violation(path.clone(), Sourcepos::from((2, 2, 2, 2))),
            rule.to_violation(path.clone(), Sourcepos::from((3, 8, 3, 8))),
            rule.to_violation(path, Sourcepos::from((4, 2, 4, 2))),
        ];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_no_errors() -> Result<()> {
        let text = "[a link](http://www.example.com/)".to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text)?;
        let rule = MD039::new();
        let actual = rule.check(&doc)?;
        let expected = vec![];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_no_errors_url() -> Result<()> {
        let text = "[a link]( http://www.example.com/ )".to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text)?;
        let rule = MD039::new();
        let actual = rule.check(&doc)?;
        let expected = vec![];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_no_errors_bracket() -> Result<()> {
        let text = "< http://www.example.com/ >".to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text)?;
        let rule = MD039::new();
        let actual = rule.check(&doc)?;
        let expected = vec![];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_no_errors_code() -> Result<()> {
        let text = indoc! {"
            [a `link`](http://www.example.com)
            [`link`](http://www.example.com)
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text)?;
        let rule = MD039::new();
        let actual = rule.check(&doc)?;
        let expected = vec![];
        assert_eq!(actual, expected);
        Ok(())
    }

    // comrak unescapes a table cell before parsing its inlines, so the columns
    // it reports from inside one are short a byte for every `\|` written before
    // them. The rule reports the link's text, which starts at column 9 here.
    #[test]
    fn check_errors_with_escaped_pipe_in_table() -> Result<()> {
        let text = indoc! {r"
            | a | b |
            | --- | --- |
            | x\|y [ t ](http://www.example.com/) | c |
            | x\|y\|z [ t ](http://www.example.com/) | c |
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD039::new();
        let actual = rule.check(&doc)?;
        let expected = vec![
            rule.to_violation(path.clone(), Sourcepos::from((3, 9, 3, 11))),
            rule.to_violation(path, Sourcepos::from((4, 12, 4, 14))),
        ];
        assert_eq!(actual, expected);
        Ok(())
    }
}
