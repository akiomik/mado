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

    // TODO: Use safe casting
    #[inline]
    #[allow(clippy::cast_possible_wrap)]
    fn check(&self, doc: &Document) -> Result<Vec<Violation>> {
        static RE: LazyLock<Regex> = LazyLock::new(|| {
            #[allow(clippy::unwrap_used)]
            Regex::new(r"(?:\s\*\s.+\*)|(?:\s\*\*\s.+\*\*)|(?:\s_\s.+_)|(?:\s__\s.+__)|(?:\*.+\s\*\s)|(?:\*\*.+\s\*\*\s)|(?:_.+\s_\s)|(?:__.+\s__\s)").unwrap()
        });

        let mut violations = vec![];

        for node in doc.ast.descendants() {
            if let NodeValue::Text(text) = &node.data.borrow().value
                && let Some(m) = RE.find(text)
            {
                let mut position = node.data.borrow().sourcepos;

                if m.as_str().starts_with(' ') {
                    // When a start marker matches
                    position.end = position.start.column_add(m.end() as isize - 1);
                    position.start = position.start.column_add(m.start() as isize + 1);
                } else {
                    // When an end marker matches
                    position.end = position.start.column_add(m.end() as isize - 2);
                    position.start = position.start.column_add(m.start() as isize);
                }

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
}
