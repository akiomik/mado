use comrak::nodes::Sourcepos;
use miette::Result;

use crate::{Document, violation::Violation};

use super::{Metadata, RuleLike, Tag};

#[derive(Debug, Default, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct MD047;

impl MD047 {
    const METADATA: Metadata = Metadata {
        name: "MD047",
        description: "File should end with a single newline character",
        tags: &[Tag::BlankLines],
        aliases: &["single-trailing-newline"],
    };

    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        Self {}
    }
}

impl RuleLike for MD047 {
    #[inline]
    fn metadata(&self) -> &'static Metadata {
        &Self::METADATA
    }

    #[inline]
    fn check(&self, doc: &Document) -> Result<Vec<Violation>> {
        if doc.text.is_empty() || doc.text.ends_with('\n') {
            return Ok(vec![]);
        }

        let lineno = doc.lines.len();
        let end_column = doc.lines.last().unwrap_or(&String::new()).len() + 1;
        let position = Sourcepos::from((lineno, 1, lineno, end_column));
        let violation = self.to_violation(doc.path.clone(), position);

        Ok(vec![violation])
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
        let text = "Some text".to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD047::new();
        let actual = rule.check(&doc)?;
        let expected = vec![rule.to_violation(path, Sourcepos::from((1, 1, 1, 10)))];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_no_errors() -> Result<()> {
        let text = indoc! {"
            Some text

        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text)?;
        let rule = MD047::new();
        let actual = rule.check(&doc)?;
        let expected = vec![];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_errors_with_empty_string() -> Result<()> {
        let text = String::new();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text)?;
        let rule = MD047::new();
        let actual = rule.check(&doc)?;
        let expected = vec![];
        assert_eq!(actual, expected);
        Ok(())
    }
}
