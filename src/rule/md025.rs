use comrak::nodes::NodeValue;
use miette::Result;

use crate::{Document, violation::Violation};

use super::{Metadata, RuleLike, Tag};

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct MD025 {
    level: u8,
}

impl MD025 {
    const METADATA: Metadata = Metadata {
        name: "MD025",
        description: "Multiple top level headers in the same document",
        tags: &[Tag::Headers],
        aliases: &["single-h1"],
    };

    pub const DEFAULT_LEVEL: u8 = 1;

    #[inline]
    #[must_use]
    pub const fn new(level: u8) -> Self {
        Self { level }
    }
}

impl Default for MD025 {
    #[inline]
    fn default() -> Self {
        Self {
            level: Self::DEFAULT_LEVEL,
        }
    }
}

impl RuleLike for MD025 {
    #[inline]
    fn metadata(&self) -> &'static Metadata {
        &Self::METADATA
    }

    #[inline]
    fn check(&self, doc: &Document) -> Result<Vec<Violation>> {
        let mut violations = vec![];
        let mut seen_top_level_header = false;

        for node in doc.ast.children() {
            if let NodeValue::Heading(heading) = node.data.borrow().value
                && heading.level == self.level
            {
                if seen_top_level_header {
                    let position = node.data.borrow().sourcepos;
                    let violation = self.to_violation(doc.path.clone(), position);
                    violations.push(violation);
                } else {
                    seen_top_level_header = true;
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
            # Top level header

            # Another top level header
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD025::default();
        let actual = rule.check(&doc)?;
        let expected = vec![rule.to_violation(path, Sourcepos::from((3, 1, 3, 26)))];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_errors_with_level() -> Result<()> {
        let text = indoc! {"
            ## Top level header

            ## Another top level header
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD025::new(2);
        let actual = rule.check(&doc)?;
        let expected = vec![rule.to_violation(path, Sourcepos::from((3, 1, 3, 27)))];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_no_errors() -> Result<()> {
        let text = indoc! {"
            # Title

            ## Header

            ## Another header
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text)?;
        let rule = MD025::default();
        let actual = rule.check(&doc)?;
        let expected = vec![];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_no_errors_with_level() -> Result<()> {
        let text = indoc! {"
            ## Title

            ### Header

            ### Another header
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text)?;
        let rule = MD025::new(2);
        let actual = rule.check(&doc)?;
        let expected = vec![];
        assert_eq!(actual, expected);
        Ok(())
    }
}
