use comrak::nodes::{ListType, NodeList, NodeValue};
use miette::Result;
use serde::{Deserialize, Serialize};

use crate::violation::Violation;
use crate::Document;

use super::{tag::Tag, Metadata, RuleLike};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum ListStyle {
    Consistent,
    Asterisk,
    Plus,
    Dash,
    // Sublist, // TODO
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct MD004 {
    style: ListStyle,
}

impl MD004 {
    const METADATA: Metadata = Metadata {
        name: "MD004",
        description: "Unordered list style",
        tags: &[Tag::Bullet, Tag::Ul],
        aliases: &["ul-style"],
    };

    pub const DEFAULT_LIST_STYLE: ListStyle = ListStyle::Consistent;

    #[inline]
    #[must_use]
    pub const fn new(style: ListStyle) -> Self {
        Self { style }
    }
}

impl Default for MD004 {
    #[inline]
    fn default() -> Self {
        Self {
            style: Self::DEFAULT_LIST_STYLE,
        }
    }
}

impl RuleLike for MD004 {
    #[inline]
    fn metadata(&self) -> &'static Metadata {
        &Self::METADATA
    }

    #[inline]
    fn check(&self, doc: &Document) -> Result<Vec<Violation>> {
        let mut violations = vec![];
        let mut maybe_list_char = None;

        for node in doc.ast.descendants() {
            if let NodeValue::Item(NodeList {
                list_type: ListType::Bullet,
                bullet_char,
                ..
            }) = node.data.borrow().value
            {
                let is_violated = match (&self.style, maybe_list_char) {
                    (ListStyle::Consistent, Some(list_char)) => bullet_char != list_char,
                    (ListStyle::Asterisk, _) => bullet_char != b'*',
                    (ListStyle::Plus, _) => bullet_char != b'+',
                    (ListStyle::Dash, _) => bullet_char != b'-',
                    _ => false,
                };

                if is_violated {
                    let position = node.data.borrow().sourcepos;
                    let violation = self.to_violation(doc.path.clone(), position);
                    violations.push(violation);
                }

                if maybe_list_char.is_none() {
                    maybe_list_char = Some(bullet_char);
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
    fn check_errors_for_consistent() {
        let text = "* Item 1
+ Item 2
- Item 3"
            .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text).unwrap();
        let rule = MD004::default();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![
            rule.to_violation(path.clone(), Sourcepos::from((2, 1, 2, 8))),
            rule.to_violation(path, Sourcepos::from((3, 1, 3, 8))),
        ];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_errors_for_asterisk() {
        let text = "* Item 1
+ Item 2
- Item 3"
            .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text).unwrap();
        let rule = MD004::new(ListStyle::Asterisk);
        let actual = rule.check(&doc).unwrap();
        let expected = vec![
            rule.to_violation(path.clone(), Sourcepos::from((2, 1, 2, 8))),
            rule.to_violation(path, Sourcepos::from((3, 1, 3, 8))),
        ];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_errors_for_plus() {
        let text = "* Item 1
+ Item 2
- Item 3"
            .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text).unwrap();
        let rule = MD004::new(ListStyle::Plus);
        let actual = rule.check(&doc).unwrap();
        let expected = vec![
            rule.to_violation(path.clone(), Sourcepos::from((1, 1, 1, 8))),
            rule.to_violation(path, Sourcepos::from((3, 1, 3, 8))),
        ];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_errors_for_dash() {
        let text = "* Item 1
+ Item 2
- Item 3"
            .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text).unwrap();
        let rule = MD004::new(ListStyle::Dash);
        let actual = rule.check(&doc).unwrap();
        let expected = vec![
            rule.to_violation(path.clone(), Sourcepos::from((1, 1, 1, 8))),
            rule.to_violation(path, Sourcepos::from((2, 1, 2, 8))),
        ];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_no_errors_for_consistent() {
        let text = "* Item 1
* Item 2
* Item 3"
            .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text).unwrap();
        let rule = MD004::default();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_no_errors_for_asterisk() {
        let text = "* Item 1
* Item 2
* Item 3"
            .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text).unwrap();
        let rule = MD004::new(ListStyle::Asterisk);
        let actual = rule.check(&doc).unwrap();
        let expected = vec![];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_no_errors_for_plus() {
        let text = "+ Item 1
+ Item 2
+ Item 3"
            .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text).unwrap();
        let rule = MD004::new(ListStyle::Plus);
        let actual = rule.check(&doc).unwrap();
        let expected = vec![];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_no_errors_for_dash() {
        let text = "- Item 1
- Item 2
- Item 3"
            .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text).unwrap();
        let rule = MD004::new(ListStyle::Dash);
        let actual = rule.check(&doc).unwrap();
        let expected = vec![];
        assert_eq!(actual, expected);
    }

    // NOTE: This test case is marked as a violation in markdownlint
    #[test]
    fn check_no_errors_with_blockquote() {
        let text = ">- Item 1
>- Item 2
>- Item 3"
            .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text).unwrap();
        let rule = MD004::default();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![];
        assert_eq!(actual, expected);
    }
}
