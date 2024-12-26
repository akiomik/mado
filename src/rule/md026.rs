use comrak::nodes::{AstNode, NodeValue};
use miette::Result;

use crate::{violation::Violation, Document};

use super::{
    node::{NodeContext, NodeRule, NodeValueMatcher},
    NewRuleLike, RuleLike, RuleMetadata,
};

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct MD026 {
    punctuation: String,
}

impl MD026 {
    pub const DEFAULT_PUNCTUATION: &str = ".,;:!?";

    #[inline]
    #[must_use]
    pub fn new(punctuation: String) -> Self {
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
    fn name(&self) -> &'static str {
        "MD026"
    }

    #[inline]
    fn description(&self) -> &'static str {
        "Trailing punctuation in header"
    }

    #[inline]
    fn tags(&self) -> Vec<&'static str> {
        vec!["headers"]
    }

    #[inline]
    fn aliases(&self) -> Vec<&'static str> {
        vec!["no-trailing-punctuation"]
    }

    #[inline]
    fn check(&self, doc: &Document) -> Result<Vec<Violation>> {
        let mut violations = vec![];

        for node in doc.ast.children() {
            if let NodeValue::Heading(_) = node.data.borrow().value {
                if let Some(child) = node.last_child() {
                    if let NodeValue::Text(text) = &child.data.borrow().value {
                        if let Some(last_char) = text.chars().last() {
                            if self.punctuation.contains(last_char) {
                                let position = node.data.borrow().sourcepos;
                                let violation = self.to_violation(doc.path.clone(), position);
                                violations.push(violation);
                            }
                        }
                    }
                }
            }
        }

        Ok(violations)
    }
}

impl NewRuleLike for MD026 {
    #[inline]
    fn metadata(&self) -> RuleMetadata {
        RuleMetadata {
            name: "MD026",
            description: "Trailing punctuation in header",
            tags: vec!["headers"],
            aliases: vec!["no-trailing-punctuation"],
        }
    }

    #[inline]
    fn reset(&mut self) {}
}

impl NodeRule for MD026 {
    #[inline]
    fn matcher(&self) -> super::node::NodeValueMatcher {
        NodeValueMatcher::new(|node| matches!(node, NodeValue::Heading(_)))
    }

    #[inline]
    fn run<'a>(&mut self, ctx: &NodeContext, node: &'a AstNode<'a>) -> Result<Vec<Violation>> {
        let mut violations = vec![];

        if let Some(child) = node.last_child() {
            if let NodeValue::Text(text) = &child.data.borrow().value {
                if let Some(last_char) = text.chars().last() {
                    if self.punctuation.contains(last_char) {
                        let position = node.data.borrow().sourcepos;
                        let violation = self.to_violation(ctx.path.clone(), position);
                        violations.push(violation);
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

    use comrak::{nodes::Sourcepos, parse_document, Arena, Options};
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn check_errors() {
        let text = "# This is a header.".to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let ast = parse_document(&arena, &text, &Options::default());
        let doc = Document {
            path: path.clone(),
            ast,
            text,
        };
        let rule = MD026::default();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![rule.to_violation(path.clone(), Sourcepos::from((1, 1, 1, 19)))];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_errors_with_link() {
        let text = "# [This is a header](http://example.com).".to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let ast = parse_document(&arena, &text, &Options::default());
        let doc = Document {
            path: path.clone(),
            ast,
            text,
        };
        let rule = MD026::default();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![rule.to_violation(path.clone(), Sourcepos::from((1, 1, 1, 41)))];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_errors_with_code() {
        let text = "# `This is a header`.".to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let ast = parse_document(&arena, &text, &Options::default());
        let doc = Document {
            path: path.clone(),
            ast,
            text,
        };
        let rule = MD026::default();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![rule.to_violation(path.clone(), Sourcepos::from((1, 1, 1, 21)))];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_errors_with_emph() {
        let text = "# *This is a header*.".to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let ast = parse_document(&arena, &text, &Options::default());
        let doc = Document {
            path: path.clone(),
            ast,
            text,
        };
        let rule = MD026::default();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![rule.to_violation(path.clone(), Sourcepos::from((1, 1, 1, 21)))];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_no_errors() {
        let text = "# This is a header".to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let ast = parse_document(&arena, &text, &Options::default());
        let doc = Document { path, ast, text };
        let rule = MD026::default();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_no_errors_with_link() {
        let text = "# [This is a header.](http://example.com)".to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let ast = parse_document(&arena, &text, &Options::default());
        let doc = Document { path, ast, text };
        let rule = MD026::default();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_no_errors_with_code() {
        let text = "# `This is a header.`".to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let ast = parse_document(&arena, &text, &Options::default());
        let doc = Document { path, ast, text };
        let rule = MD026::default();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_no_errors_with_emph() {
        let text = "# *This is a header.*".to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let ast = parse_document(&arena, &text, &Options::default());
        let doc = Document { path, ast, text };
        let rule = MD026::default();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![];
        assert_eq!(actual, expected);
    }
}
