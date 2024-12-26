use comrak::nodes::{AstNode, ListType, NodeList, NodeValue};
use miette::Result;
use serde::Deserialize;

use crate::violation::Violation;
use crate::Document;

use super::{
    node::{NodeContext, NodeRule, NodeValueMatcher},
    NewRuleLike, RuleLike, RuleMetadata,
};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum ListStyle {
    Consistent,
    Asterisk,
    Plus,
    Dash,
    // Sublist, // TODO
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Deserialize)]
pub struct State {
    seen_list_char: Option<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct MD004 {
    style: ListStyle,
    state: State,
}

impl MD004 {
    pub const DEFAULT_LIST_STYLE: ListStyle = ListStyle::Consistent;

    #[inline]
    #[must_use]
    pub fn new(style: ListStyle) -> Self {
        Self {
            style,
            state: State::default(),
        }
    }
}

impl Default for MD004 {
    #[inline]
    fn default() -> Self {
        Self {
            style: Self::DEFAULT_LIST_STYLE,
            state: State::default(),
        }
    }
}

impl RuleLike for MD004 {
    #[inline]
    fn name(&self) -> &'static str {
        "MD004"
    }

    #[inline]
    fn description(&self) -> &'static str {
        "Unordered list style"
    }

    #[inline]
    fn tags(&self) -> Vec<&'static str> {
        vec!["bullet", "ul"]
    }

    #[inline]
    fn aliases(&self) -> Vec<&'static str> {
        vec!["ul-style"]
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

impl NewRuleLike for MD004 {
    #[inline]
    fn metadata(&self) -> RuleMetadata {
        RuleMetadata {
            name: "MD004",
            description: "Unordered list style",
            tags: vec!["bullet", "ul"],
            aliases: vec!["ul-style"],
        }
    }

    #[inline]
    fn reset(&mut self) {
        self.state = State::default();
    }
}

impl NodeRule for MD004 {
    #[inline]
    fn matcher(&self) -> NodeValueMatcher {
        NodeValueMatcher::new(|node| {
            matches!(
                node,
                NodeValue::Item(NodeList { list_type, .. }) if *list_type == ListType::Bullet
            )
        })
    }

    #[inline]
    fn run<'a>(&mut self, ctx: &NodeContext, node: &'a AstNode<'a>) -> Result<Vec<Violation>> {
        let mut violations = vec![];

        if let NodeValue::Item(NodeList { bullet_char, .. }) = node.data.borrow().value {
            let is_violated = match (&self.style, self.state.seen_list_char) {
                (ListStyle::Consistent, Some(list_char)) => bullet_char != list_char,
                (ListStyle::Asterisk, _) => bullet_char != b'*',
                (ListStyle::Plus, _) => bullet_char != b'+',
                (ListStyle::Dash, _) => bullet_char != b'-',
                _ => false,
            };

            if is_violated {
                let position = node.data.borrow().sourcepos;
                let violation = self.to_violation(ctx.path.clone(), position);
                violations.push(violation);
            }

            if self.state.seen_list_char.is_none() {
                self.state.seen_list_char = Some(bullet_char);
            }
        }

        Ok(violations)
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use comrak::{nodes::Sourcepos, parse_document, Arena, Options};
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
        let ast = parse_document(&arena, &text, &Options::default());
        let doc = Document {
            path: path.clone(),
            ast,
            text,
        };
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
        let ast = parse_document(&arena, &text, &Options::default());
        let doc = Document {
            path: path.clone(),
            ast,
            text,
        };
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
        let ast = parse_document(&arena, &text, &Options::default());
        let doc = Document {
            path: path.clone(),
            ast,
            text,
        };
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
        let ast = parse_document(&arena, &text, &Options::default());
        let doc = Document {
            path: path.clone(),
            ast,
            text,
        };
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
        let ast = parse_document(&arena, &text, &Options::default());
        let doc = Document { path, ast, text };
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
        let ast = parse_document(&arena, &text, &Options::default());
        let doc = Document { path, ast, text };
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
        let ast = parse_document(&arena, &text, &Options::default());
        let doc = Document { path, ast, text };
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
        let ast = parse_document(&arena, &text, &Options::default());
        let doc = Document { path, ast, text };
        let rule = MD004::new(ListStyle::Dash);
        let actual = rule.check(&doc).unwrap();
        let expected = vec![];
        assert_eq!(actual, expected);
    }

    // NOTE: This test case is marked as a violation in markdownlint
    #[test]
    fn check_errors_with_blockquote() {
        let text = ">- Item 1
>- Item 2
>- Item 3"
            .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let ast = parse_document(&arena, &text, &Options::default());
        let doc = Document {
            path: path.clone(),
            ast,
            text,
        };
        let rule = MD004::default();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![];
        assert_eq!(actual, expected);
    }
}
