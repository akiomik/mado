use std::collections::HashSet;

use comrak::nodes::NodeValue;
use miette::Result;

use crate::{violation::Violation, Document};

use super::{helper::inline_text_of, tag::Tag, Metadata, RuleLike};

// TODO: Support allow_different_nesting
#[derive(Default, Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct MD024;

impl MD024 {
    const METADATA: Metadata = Metadata {
        name: "MD024",
        description: "Multiple headers with the same content",
        tags: &[Tag::Headers],
        aliases: &["no-duplicate-header"],
    };

    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        Self {}
    }
}

impl RuleLike for MD024 {
    #[inline]
    fn metadata(&self) -> &'static Metadata {
        &Self::METADATA
    }

    #[inline]
    fn check(&self, doc: &Document) -> Result<Vec<Violation>> {
        let mut violations = vec![];
        let mut contents: HashSet<String> = HashSet::new();

        for node in doc.ast.children() {
            if let NodeValue::Heading(_) = node.data.borrow().value {
                let text = inline_text_of(node);
                if contents.contains(&text) {
                    let position = node.data.borrow().sourcepos;
                    let violation = self.to_violation(doc.path.clone(), position);
                    violations.push(violation);
                } else {
                    contents.insert(text.clone());
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
        let text = "# Some text

## Some text"
            .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text).unwrap();
        let rule = MD024::new();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![rule.to_violation(path, Sourcepos::from((3, 1, 3, 12)))];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_no_errors() {
        let text = "# Some text

## Some more text"
            .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text).unwrap();
        let rule = MD024::new();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![];
        assert_eq!(actual, expected);
    }
}
