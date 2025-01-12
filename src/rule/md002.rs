use comrak::nodes::NodeValue;
use miette::Result;

use crate::violation::Violation;
use crate::Document;

use super::{Metadata, RuleLike};

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct MD002 {
    pub level: u8,
}

impl MD002 {
    const METADATA: Metadata = Metadata {
        name: "MD002",
        description: "First header should be a top level header",
        tags: &["headers"],
        aliases: &["first-header-h1"],
    };

    pub const DEFAULT_LEVEL: u8 = 1;

    #[inline]
    #[must_use]
    pub fn new(level: u8) -> Self {
        Self { level }
    }
}

impl Default for MD002 {
    #[inline]
    fn default() -> Self {
        Self {
            level: Self::DEFAULT_LEVEL,
        }
    }
}

impl RuleLike for MD002 {
    #[inline]
    fn metadata(&self) -> Metadata {
        Self::METADATA
    }

    #[inline]
    fn check(&self, doc: &Document) -> Result<Vec<Violation>> {
        for node in doc.ast.children() {
            if let NodeValue::Heading(heading) = node.data.borrow().value {
                if heading.level != self.level {
                    let position = node.data.borrow().sourcepos;
                    let violation = self.to_violation(doc.path.clone(), position);

                    return Ok(vec![violation]);
                }

                break;
            }
        }

        Ok(vec![])
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
        let text = "## This isn't a H1 header

### Another header"
            .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text).unwrap();
        let rule = MD002::default();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![rule.to_violation(path, Sourcepos::from((1, 1, 1, 25)))];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_errors_with_level() {
        let text = "# Start with a H1 header

## Then use a H2 for subsections"
            .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text).unwrap();
        let rule = MD002::new(2);
        let actual = rule.check(&doc).unwrap();
        let expected = vec![rule.to_violation(path, Sourcepos::from((1, 1, 1, 24)))];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_no_errors() {
        let text = "# Start with a H1 header

## Then use a H2 for subsections"
            .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text).unwrap();
        let rule = MD002::default();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_no_errors_with_level() {
        let text = "## This isn't a H1 header

### Another header"
            .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text).unwrap();
        let rule = MD002::new(2);
        let actual = rule.check(&doc).unwrap();
        let expected = vec![];
        assert_eq!(actual, expected);
    }
}
