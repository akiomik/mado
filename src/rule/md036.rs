use comrak::nodes::NodeValue;
use miette::Result;

use crate::{violation::Violation, Document};

use super::{tag::Tag, Metadata, RuleLike};

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct MD036 {
    punctuation: String,
}

impl MD036 {
    const METADATA: Metadata = Metadata {
        name: "MD036",
        description: "Emphasis used instead of a header",
        tags: &[Tag::Headers, Tag::Emphasis],
        aliases: &["no-emphasis-as-header"],
    };

    pub const DEFAULT_PUNCTUATION: &str = ".,;:!?";

    #[inline]
    #[must_use]
    pub const fn new(punctuation: String) -> Self {
        Self { punctuation }
    }
}

impl Default for MD036 {
    #[inline]
    fn default() -> Self {
        Self {
            punctuation: Self::DEFAULT_PUNCTUATION.to_owned(),
        }
    }
}

impl RuleLike for MD036 {
    #[inline]
    fn metadata(&self) -> &'static Metadata {
        &Self::METADATA
    }

    #[inline]
    fn check(&self, doc: &Document) -> Result<Vec<Violation>> {
        let mut violations = vec![];

        for node in doc.ast.descendants() {
            if node.data.borrow().value == NodeValue::Paragraph {
                if node.children().count() > 1 {
                    continue;
                }

                if let Some(child_node) = node.first_child() {
                    if let NodeValue::Emph | NodeValue::Strong = &child_node.data.borrow().value {
                        let position = node.data.borrow().sourcepos;
                        if position.end.line > position.start.line {
                            continue;
                        }

                        if position.start.column > 1 {
                            continue;
                        }

                        for inline_node in child_node.children() {
                            if let NodeValue::Text(text) = &inline_node.data.borrow().value {
                                if let Some(last_char) = text.chars().last() {
                                    if !self.punctuation.contains(last_char) {
                                        let violation =
                                            self.to_violation(doc.path.clone(), position);
                                        violations.push(violation);
                                    }
                                }
                            }
                        }
                    }
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
        let text = "**My document**

Lorem ipsum dolor sit amet...

_Another section_

Consectetur adipiscing elit, sed do eiusmod."
            .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text).unwrap();
        let rule = MD036::default();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![
            rule.to_violation(path.clone(), Sourcepos::from((1, 1, 1, 15))),
            rule.to_violation(path, Sourcepos::from((5, 1, 5, 17))),
        ];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_errors_in_between_texts() {
        let text = "Some text

**Strong text**

Some more text"
            .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text).unwrap();
        let rule = MD036::default();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![rule.to_violation(path, Sourcepos::from((3, 1, 3, 15)))];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_no_errors() {
        let text = "# My document

Lorem ipsum dolor sit amet...

## Another section

Consectetur adipiscing elit, sed do eiusmod."
            .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text).unwrap();
        let rule = MD036::default();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_no_errors_with_punctuation() {
        let text = "**My document.**

Lorem ipsum dolor sit amet...

_Another section?_

Consectetur adipiscing elit, sed do eiusmod."
            .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text).unwrap();
        let rule = MD036::default();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_no_errors_with_text() {
        let text = "foo **My document**

_Another section_ bar"
            .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text).unwrap();
        let rule = MD036::default();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_no_errors_in_between_texts() {
        let text = "Some text
**Strong text**
Some more text"
            .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text).unwrap();
        let rule = MD036::default();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_no_errors_multiple_lines() {
        let text = "**Multiple lines
text**"
            .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text).unwrap();
        let rule = MD036::default();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_no_errors_with_indent() {
        let text = " **My document**".to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text).unwrap();
        let rule = MD036::default();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_no_errors_with_code() {
        let text = "**`My document`**".to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text).unwrap();
        let rule = MD036::default();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_no_errors_with_link() {
        let text = "**[My document](https://example.com)**".to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text).unwrap();
        let rule = MD036::default();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![];
        assert_eq!(actual, expected);
    }
}
