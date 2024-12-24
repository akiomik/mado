use std::collections::HashSet;

use comrak::nodes::{AstNode, NodeValue};
use miette::Result;

use crate::{violation::Violation, Document};

use super::{
    helper::inline_text_of,
    node::{NodeContext, NodeRule, NodeValueMatcher},
    NewRuleLike, RuleLike, RuleMetadata,
};

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct State {
    pub contents: HashSet<String>,
}

// TODO: Support allow_different_nesting
#[derive(Default, Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct MD024 {
    state: State,
}

impl MD024 {
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self {
            state: State::default(),
        }
    }
}

impl RuleLike for MD024 {
    #[inline]
    fn name(&self) -> &'static str {
        "MD024"
    }

    #[inline]
    fn description(&self) -> &'static str {
        "Multiple headers with the same content"
    }

    #[inline]
    fn tags(&self) -> Vec<&'static str> {
        vec!["headers"]
    }

    #[inline]
    fn aliases(&self) -> Vec<&'static str> {
        vec!["no-duplicate-header"]
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

impl NewRuleLike for MD024 {
    fn metadata(&self) -> RuleMetadata {
        RuleMetadata {
            name: "MD024",
            description: "Multiple headers with the same content",
            tags: vec!["headers"],
            aliases: vec!["no-duplicate-header"],
        }
    }

    fn reset(&mut self) {
        self.state = State::default();
    }
}

impl NodeRule for MD024 {
    fn matcher(&self) -> NodeValueMatcher {
        NodeValueMatcher::new(|node| matches!(node, NodeValue::Heading(_)))
    }

    fn run<'a>(&mut self, ctx: &NodeContext, node: &'a AstNode<'a>) -> Result<Vec<Violation>> {
        let mut violations = vec![];

        let text = inline_text_of(node);
        if self.state.contents.contains(&text) {
            let position = node.data.borrow().sourcepos;
            let violation = self.to_violation(ctx.path.clone(), position);
            violations.push(violation);
        } else {
            self.state.contents.insert(text.clone());
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
        let text = "# Some text

## Some text"
            .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let ast = parse_document(&arena, &text, &Options::default());
        let doc = Document {
            path: path.clone(),
            ast,
            text,
        };
        let rule = MD024::new();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![rule.to_violation(path.clone(), Sourcepos::from((3, 1, 3, 12)))];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_no_errors() {
        let text = "# Some text

## Some more text"
            .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let ast = parse_document(&arena, &text, &Options::default());
        let doc = Document { path, ast, text };
        let rule = MD024::new();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![];
        assert_eq!(actual, expected);
    }
}
