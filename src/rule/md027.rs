use comrak::nodes::{AstNode, NodeValue};
use miette::Result;

use crate::{violation::Violation, Document};

use super::{
    node::{NodeContext, NodeRule, NodeValueMatcher},
    NewRuleLike, RuleLike, RuleMetadata,
};

#[derive(Debug, Default, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct MD027;

impl MD027 {
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self {}
    }
}

impl RuleLike for MD027 {
    #[inline]
    fn name(&self) -> &'static str {
        "MD027"
    }

    #[inline]
    fn description(&self) -> &'static str {
        "Multiple spaces after blockquote symbol"
    }

    #[inline]
    fn tags(&self) -> Vec<&'static str> {
        vec!["blockquote", "whitespace", "indentation"]
    }

    #[inline]
    fn aliases(&self) -> Vec<&'static str> {
        vec!["no-multiple-space-blockquote"]
    }

    #[inline]
    fn check(&self, doc: &Document) -> Result<Vec<Violation>> {
        let mut violations = vec![];

        for node in doc.ast.descendants() {
            if let NodeValue::BlockQuote = node.data.borrow().value {
                if let Some(child_node) = node.first_child() {
                    let block_quote_position = node.data.borrow().sourcepos;
                    let child_position = child_node.data.borrow().sourcepos;

                    if child_position.start.column > block_quote_position.start.column + 2 {
                        let violation = self.to_violation(doc.path.clone(), child_position);
                        violations.push(violation);
                    }
                }
            }
        }

        Ok(violations)
    }
}

impl NewRuleLike for MD027 {
    #[inline]
    fn metadata(&self) -> RuleMetadata {
        RuleMetadata {
            name: "MD027",
            description: "Multiple spaces after blockquote symbol",
            tags: vec!["blockquote", "whitespace", "indentation"],
            aliases: vec!["no-multiple-space-blockquote"],
        }
    }

    #[inline]
    fn reset(&mut self) {}
}

impl NodeRule for MD027 {
    #[inline]
    fn matcher(&self) -> NodeValueMatcher {
        NodeValueMatcher::new(|node| matches!(node, NodeValue::BlockQuote))
    }

    #[inline]
    fn run<'a>(&mut self, ctx: &NodeContext, node: &'a AstNode<'a>) -> Result<Vec<Violation>> {
        let mut violations = vec![];

        if let Some(child_node) = node.first_child() {
            let block_quote_position = node.data.borrow().sourcepos;
            let child_position = child_node.data.borrow().sourcepos;

            if child_position.start.column > block_quote_position.start.column + 2 {
                let violation = self.to_violation(ctx.path.clone(), child_position);
                violations.push(violation);
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
        let text = ">  This is a block quote with bad indentation
>  there should only be one."
            .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let ast = parse_document(&arena, &text, &Options::default());
        let doc = Document {
            path: path.clone(),
            ast,
            text,
        };
        let rule = MD027::new();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![
            // TODO: Support multiple errors
            rule.to_violation(path.clone(), Sourcepos::from((1, 4, 2, 28))),
            // rule.to_violation(path.clone(), Sourcepos::from((1, 4, 1, 45))),
            // rule.to_violation(path, Sourcepos::from((2, 4, 2, 28))),
        ];
        assert_eq!(actual, expected);
    }

    // NOTE: This case is not an error in markdownlint
    #[test]
    fn check_errors_with_nested_block_quotes() {
        let text = ">>>  This is multiple blockquote with bad indentation.".to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let ast = parse_document(&arena, &text, &Options::default());
        let doc = Document {
            path: path.clone(),
            ast,
            text,
        };
        let rule = MD027::new();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![rule.to_violation(path.clone(), Sourcepos::from((1, 6, 1, 54)))];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_errors_with_nested_block_quotes2() {
        let text = ">  >  >  This is multiple blockquote with bad indentation.".to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let ast = parse_document(&arena, &text, &Options::default());
        let doc = Document {
            path: path.clone(),
            ast,
            text,
        };
        let rule = MD027::new();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![
            rule.to_violation(path.clone(), Sourcepos::from((1, 4, 1, 58))),
            rule.to_violation(path.clone(), Sourcepos::from((1, 7, 1, 58))),
            rule.to_violation(path, Sourcepos::from((1, 10, 1, 58))),
        ];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_no_errors() {
        let text = "> This is a blockquote with correct
> indentation."
            .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let ast = parse_document(&arena, &text, &Options::default());
        let doc = Document { path, ast, text };
        let rule = MD027::new();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_no_errors_with_nested_block_quotes() {
        let text = ">>> This is multiple blockquote with correct indentation.".to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let ast = parse_document(&arena, &text, &Options::default());
        let doc = Document { path, ast, text };
        let rule = MD027::new();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_no_errors_with_nested_block_quotes2() {
        let text = ">>> This is multiple blockquote with bad
    indentation."
            .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let ast = parse_document(&arena, &text, &Options::default());
        let doc = Document {
            path: path.clone(),
            ast,
            text,
        };
        let rule = MD027::new();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_no_errors_with_nested_block_quotes3() {
        let text = "> > > This is multiple blockquote with correct indentation.".to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let ast = parse_document(&arena, &text, &Options::default());
        let doc = Document { path, ast, text };
        let rule = MD027::new();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![];
        assert_eq!(actual, expected);
    }
}
