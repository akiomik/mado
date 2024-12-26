use comrak::nodes::{AstNode, NodeHeading, NodeValue};
use miette::Result;
use serde::Deserialize;

use crate::violation::Violation;
use crate::Document;

use super::{
    node::{NodeContext, NodeRule, NodeValueMatcher},
    NewRuleLike, RuleLike, RuleMetadata,
};

#[derive(Debug, Default, Clone, PartialEq, Eq, Deserialize)]
pub struct State {
    seen_heading_style: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum HeadingStyle {
    Consistent,
    Atx,
    // AtxClosed, // TODO
    Setext,
    SetextWithAtx,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct MD003 {
    style: HeadingStyle,
    state: State,
}

impl MD003 {
    pub const DEFAULT_HEADING_STYLE: HeadingStyle = HeadingStyle::Consistent;

    #[inline]
    #[must_use]
    pub fn new(style: HeadingStyle) -> Self {
        Self {
            style,
            state: State::default(),
        }
    }
}

impl Default for MD003 {
    #[inline]
    fn default() -> Self {
        Self {
            style: Self::DEFAULT_HEADING_STYLE,
            state: State::default(),
        }
    }
}

impl RuleLike for MD003 {
    #[inline]
    fn name(&self) -> &'static str {
        "MD003"
    }

    #[inline]
    fn description(&self) -> &'static str {
        "Header style"
    }

    #[inline]
    fn tags(&self) -> Vec<&'static str> {
        vec!["headers"]
    }

    #[inline]
    fn aliases(&self) -> Vec<&'static str> {
        vec!["header-style"]
    }

    #[inline]
    fn check(&self, doc: &Document) -> Result<Vec<Violation>> {
        let mut violations = vec![];
        let mut maybe_heading_style = None;

        for node in doc.ast.children() {
            if let NodeValue::Heading(NodeHeading { level, setext, .. }) = node.data.borrow().value
            {
                let is_violated = match (&self.style, maybe_heading_style) {
                    (HeadingStyle::Consistent, Some(heading_style)) => setext != heading_style,
                    (HeadingStyle::Atx, _) => setext,
                    (HeadingStyle::Setext, _) => !setext,
                    (HeadingStyle::SetextWithAtx, _) => level < 3 && !setext,
                    _ => false,
                };

                if is_violated {
                    let position = node.data.borrow().sourcepos;
                    let violation = self.to_violation(doc.path.clone(), position);
                    violations.push(violation);
                }

                if maybe_heading_style.is_none() {
                    maybe_heading_style = Some(setext);
                }
            }
        }

        Ok(violations)
    }
}

impl NewRuleLike for MD003 {
    #[inline]
    fn metadata(&self) -> RuleMetadata {
        RuleMetadata {
            name: "MD003",
            description: "Header style",
            tags: vec!["headers"],
            aliases: vec!["header-style"],
        }
    }

    #[inline]
    fn reset(&mut self) {
        self.state = State::default();
    }
}

impl NodeRule for MD003 {
    #[inline]
    fn matcher(&self) -> NodeValueMatcher {
        NodeValueMatcher::new(|node| matches!(node, NodeValue::Heading(_)))
    }

    #[inline]
    fn run<'a>(&mut self, ctx: &NodeContext, node: &'a AstNode<'a>) -> Result<Vec<Violation>> {
        let mut violations = vec![];

        if let NodeValue::Heading(NodeHeading { level, setext, .. }) = node.data.borrow().value {
            let is_violated = match (&self.style, self.state.seen_heading_style) {
                (HeadingStyle::Consistent, Some(heading_style)) => setext != heading_style,
                (HeadingStyle::Atx, _) => setext,
                (HeadingStyle::Setext, _) => !setext,
                (HeadingStyle::SetextWithAtx, _) => level < 3 && !setext,
                _ => false,
            };

            if is_violated {
                let position = node.data.borrow().sourcepos;
                let violation = self.to_violation(ctx.path.clone(), position);
                violations.push(violation);
            }

            if self.state.seen_heading_style.is_none() {
                self.state.seen_heading_style = Some(setext);
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
        let text = "# ATX style H1

## Closed ATX style H2 ##

Setext style H1
==============="
            .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let ast = parse_document(&arena, &text, &Options::default());
        let doc = Document {
            path: path.clone(),
            ast,
            text,
        };
        let rule = MD003::default();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![rule.to_violation(path, Sourcepos::from((5, 1, 6, 15)))];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_errors_for_atx() {
        let text = "# ATX style H1

## Closed ATX style H2 ##

Setext style H1
==============="
            .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let ast = parse_document(&arena, &text, &Options::default());
        let doc = Document {
            path: path.clone(),
            ast,
            text,
        };
        let rule = MD003::new(HeadingStyle::Atx);
        let actual = rule.check(&doc).unwrap();
        let expected = vec![rule.to_violation(path, Sourcepos::from((5, 1, 6, 15)))];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_errors_for_setext() {
        let text = "# ATX style H1

## Closed ATX style H2 ##

Setext style H1
==============="
            .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let ast = parse_document(&arena, &text, &Options::default());
        let doc = Document {
            path: path.clone(),
            ast,
            text,
        };
        let rule = MD003::new(HeadingStyle::Setext);
        let actual = rule.check(&doc).unwrap();
        let expected = vec![
            rule.to_violation(path.clone(), Sourcepos::from((1, 1, 1, 14))),
            rule.to_violation(path.clone(), Sourcepos::from((3, 1, 3, 25))),
        ];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_errors_for_setext_with_atx() {
        let text = "# ATX style H1

## ATX style H2

### ATX style H3"
            .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let ast = parse_document(&arena, &text, &Options::default());
        let doc = Document {
            path: path.clone(),
            ast,
            text,
        };
        let rule = MD003::new(HeadingStyle::SetextWithAtx);
        let actual = rule.check(&doc).unwrap();
        let expected = vec![
            rule.to_violation(path.clone(), Sourcepos::from((1, 1, 1, 14))),
            rule.to_violation(path, Sourcepos::from((3, 1, 3, 15))),
        ];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_no_errors_for_consistent() {
        let text = "# ATX style H1

## ATX style H2"
            .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let ast = parse_document(&arena, &text, &Options::default());
        let doc = Document { path, ast, text };
        let rule = MD003::default();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_no_errors_for_atx() {
        let text = "# ATX style H1

## ATX style H2"
            .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let ast = parse_document(&arena, &text, &Options::default());
        let doc = Document { path, ast, text };
        let rule = MD003::new(HeadingStyle::Atx);
        let actual = rule.check(&doc).unwrap();
        let expected = vec![];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_no_errors_for_setext() {
        let text = "Setext style H1
===============

Setext style H2
---------------"
            .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let ast = parse_document(&arena, &text, &Options::default());
        let doc = Document { path, ast, text };
        let rule = MD003::new(HeadingStyle::Setext);
        let actual = rule.check(&doc).unwrap();
        let expected = vec![];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_no_errors_for_setext_with_atx() {
        let text = "Setext style H1
===============

Setext style H2
---------------

### ATX style H3"
            .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let ast = parse_document(&arena, &text, &Options::default());
        let doc = Document { path, ast, text };
        let rule = MD003::new(HeadingStyle::SetextWithAtx);
        let actual = rule.check(&doc).unwrap();
        let expected = vec![];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_no_errors_with_front_matter() {
        let text = r#"---
author: "John Smith"
---

# Header 1"#
            .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let mut options = Options::default();
        options.extension.front_matter_delimiter = Some("---".to_owned());
        let ast = parse_document(&arena, &text, &options);
        let doc = Document { path, ast, text };
        let rule = MD003::new(HeadingStyle::Consistent);
        let actual = rule.check(&doc).unwrap();
        let expected = vec![];
        assert_eq!(actual, expected);
    }
}
