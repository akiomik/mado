use comrak::nodes::{NodeHeading, NodeValue};
use miette::Result;

use crate::{violation::Violation, Document};

use super::{tag::Tag, Metadata, RuleLike};

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct MD041 {
    level: u8,
}

impl MD041 {
    const METADATA: Metadata = Metadata {
        name: "MD041",
        description: "First line in file should be a top level header",
        tags: &[Tag::Headers],
        aliases: &["first-line-h1"],
    };

    pub const DEFAULT_LEVEL: u8 = 1;

    #[inline]
    #[must_use]
    pub const fn new(level: u8) -> Self {
        Self { level }
    }
}

impl Default for MD041 {
    #[inline]
    fn default() -> Self {
        Self {
            level: Self::DEFAULT_LEVEL,
        }
    }
}

impl RuleLike for MD041 {
    #[inline]
    fn metadata(&self) -> &'static Metadata {
        &Self::METADATA
    }

    #[inline]
    fn check(&self, doc: &Document) -> Result<Vec<Violation>> {
        let mut violations = vec![];

        if let Some(node) = doc.ast.first_child() {
            match node.data.borrow().value {
                NodeValue::Heading(NodeHeading { level, .. }) if level == self.level => {}
                _ => {
                    let position = node.data.borrow().sourcepos;
                    let violation = self.to_violation(doc.path.clone(), position);
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

    use comrak::{nodes::Sourcepos, Arena};
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn check_errors() {
        let text = "This is a file without a header".to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text).unwrap();
        let rule = MD041::default();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![rule.to_violation(path, Sourcepos::from((1, 1, 1, 31)))];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_no_errors() {
        let text = "# File with header

This is a file with a top level header"
            .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text).unwrap();
        let rule = MD041::default();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_no_errors_with_empty_text() {
        let text = String::new();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text).unwrap();
        let rule = MD041::default();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![];
        assert_eq!(actual, expected);
    }
}
