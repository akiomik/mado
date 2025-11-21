use comrak::nodes::NodeValue;
use miette::Result;

use crate::{Document, violation::Violation};

use super::{Metadata, RuleLike, Tag};

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct MD026 {
    punctuation: String,
}

impl MD026 {
    const METADATA: Metadata = Metadata {
        name: "MD026",
        description: "Trailing punctuation in header",
        tags: &[Tag::Headers],
        aliases: &["no-trailing-punctuation"],
    };

    pub const DEFAULT_PUNCTUATION: &str = ".,;:!?";

    #[inline]
    #[must_use]
    pub const fn new(punctuation: String) -> Self {
        Self { punctuation }
    }
}

impl Default for MD026 {
    #[inline]
    fn default() -> Self {
        Self {
            punctuation: Self::DEFAULT_PUNCTUATION.to_owned(),
        }
    }
}

impl RuleLike for MD026 {
    #[inline]
    fn metadata(&self) -> &'static Metadata {
        &Self::METADATA
    }

    #[inline]
    fn check(&self, doc: &Document) -> Result<Vec<Violation>> {
        let mut violations = vec![];

        for node in doc.ast.children() {
            if let NodeValue::Heading(_) = node.data.borrow().value
                && let Some(child) = node.last_child()
                && let NodeValue::Text(text) = &child.data.borrow().value
                && let Some(last_char) = text.chars().last()
                && self.punctuation.contains(last_char)
            {
                let position = node.data.borrow().sourcepos;
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
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn check_errors() -> Result<()> {
        let text = "# This is a header.".to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD026::default();
        let actual = rule.check(&doc)?;
        let expected = vec![rule.to_violation(path, Sourcepos::from((1, 1, 1, 19)))];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_errors_with_link() -> Result<()> {
        let text = "# [This is a header](http://example.com).".to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD026::default();
        let actual = rule.check(&doc)?;
        let expected = vec![rule.to_violation(path, Sourcepos::from((1, 1, 1, 41)))];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_errors_with_code() -> Result<()> {
        let text = "# `This is a header`.".to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD026::default();
        let actual = rule.check(&doc)?;
        let expected = vec![rule.to_violation(path, Sourcepos::from((1, 1, 1, 21)))];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_errors_with_emph() -> Result<()> {
        let text = "# *This is a header*.".to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD026::default();
        let actual = rule.check(&doc)?;
        let expected = vec![rule.to_violation(path, Sourcepos::from((1, 1, 1, 21)))];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_no_errors() -> Result<()> {
        let text = "# This is a header".to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text)?;
        let rule = MD026::default();
        let actual = rule.check(&doc)?;
        let expected = vec![];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_no_errors_with_link() -> Result<()> {
        let text = "# [This is a header.](http://example.com)".to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text)?;
        let rule = MD026::default();
        let actual = rule.check(&doc)?;
        let expected = vec![];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_no_errors_with_code() -> Result<()> {
        let text = "# `This is a header.`".to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text)?;
        let rule = MD026::default();
        let actual = rule.check(&doc)?;
        let expected = vec![];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_no_errors_with_emph() -> Result<()> {
        let text = "# *This is a header.*".to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text)?;
        let rule = MD026::default();
        let actual = rule.check(&doc)?;
        let expected = vec![];
        assert_eq!(actual, expected);
        Ok(())
    }
}
