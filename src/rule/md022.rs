use std::cell::RefCell;

use comrak::nodes::{Ast, AstNode, NodeValue};
use miette::Result;

use crate::{violation::Violation, Document};

use super::{
    node::{NodeContext, NodeRule, NodeValueMatcher},
    NewRuleLike, RuleLike, RuleMetadata,
};

#[derive(Default, Debug, Clone)]
pub struct State {
    pub maybe_prev_node_ref: Option<RefCell<Ast>>,
}

#[derive(Default, Debug, Clone)]
#[non_exhaustive]
pub struct MD022 {
    pub state: State,
}

impl MD022 {
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self {
            state: State::default(),
        }
    }
}

impl RuleLike for MD022 {
    #[inline]
    fn name(&self) -> &'static str {
        "MD022"
    }

    #[inline]
    fn description(&self) -> &'static str {
        "Headers should be surrounded by blank lines"
    }

    #[inline]
    fn tags(&self) -> Vec<&'static str> {
        vec!["headers", "blank_lines"]
    }

    #[inline]
    fn aliases(&self) -> Vec<&'static str> {
        vec!["blanks-around-headers"]
    }

    #[inline]
    fn check(&self, doc: &Document) -> Result<Vec<Violation>> {
        let mut violations = vec![];
        let mut maybe_prev_node: Option<&AstNode> = None;

        for node in doc.ast.children() {
            if let Some(prev_node) = maybe_prev_node {
                let prev_position = prev_node.data.borrow().sourcepos;
                let position = node.data.borrow().sourcepos;

                match (&prev_node.data.borrow().value, &node.data.borrow().value) {
                    (NodeValue::Heading(_), _) => {
                        if position.start.line == prev_position.end.line + 1 {
                            let violation = self.to_violation(doc.path.clone(), prev_position);
                            violations.push(violation);
                        }
                    }
                    (_, NodeValue::Heading(_)) => {
                        // NOTE: Ignore column 0, as the List may end on the next line
                        if position.start.line == prev_position.end.line + 1
                            && prev_position.end.column != 0
                        {
                            let violation = self.to_violation(doc.path.clone(), position);
                            violations.push(violation);
                        }
                    }
                    _ => {}
                }
            }

            maybe_prev_node = Some(node);
        }

        Ok(violations)
    }
}

impl NewRuleLike for MD022 {
    fn metadata(&self) -> RuleMetadata {
        RuleMetadata {
            name: "MD022",
            description: "Headers should be surrounded by blank lines",
            tags: vec!["headers", "blank_lines"],
            aliases: vec!["blanks-around-headers"],
        }
    }

    fn reset(&mut self) {
        self.state = State::default();
    }
}

impl NodeRule for MD022 {
    fn matcher(&self) -> NodeValueMatcher {
        NodeValueMatcher::new(|_| true)
    }

    fn run<'a>(&mut self, ctx: &NodeContext, node: &'a AstNode<'a>) -> Result<Vec<Violation>> {
        let mut violations = vec![];

        if let Some(prev_node_ref) = &self.state.maybe_prev_node_ref {
            let prev_position = prev_node_ref.borrow().sourcepos;
            let position = node.data.borrow().sourcepos;

            match (&prev_node_ref.borrow().value, &node.data.borrow().value) {
                (NodeValue::Heading(_), _) => {
                    if position.start.line == prev_position.end.line + 1 {
                        let violation = self.to_violation(ctx.path.clone(), prev_position);
                        violations.push(violation);
                    }
                }
                (_, NodeValue::Heading(_)) => {
                    // NOTE: Ignore column 0, as the List may end on the next line
                    if position.start.line == prev_position.end.line + 1
                        && prev_position.end.column != 0
                    {
                        let violation = self.to_violation(ctx.path.clone(), position);
                        violations.push(violation);
                    }
                }
                _ => {}
            }
        }

        self.state.maybe_prev_node_ref = Some(node.data.clone());

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
    fn check_errors_for_atx() {
        let text = "# Header 1
Some text

Some more text
## Header 2"
            .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let ast = parse_document(&arena, &text, &Options::default());
        let doc = Document {
            path: path.clone(),
            ast,
            text,
        };
        let rule = MD022::new();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![
            rule.to_violation(path.clone(), Sourcepos::from((1, 1, 1, 10))),
            rule.to_violation(path, Sourcepos::from((5, 1, 5, 11))),
        ];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_errors_for_setext() {
        let text = "Setext style H1
===============
Some text

```
Some code block
```
Setext style H2
---------------"
            .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let ast = parse_document(&arena, &text, &Options::default());
        let doc = Document {
            path: path.clone(),
            ast,
            text,
        };
        let rule = MD022::new();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![
            rule.to_violation(path.clone(), Sourcepos::from((1, 1, 2, 15))),
            rule.to_violation(path, Sourcepos::from((8, 1, 9, 15))),
        ];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_no_errors() {
        let text = "# Header 1

Some text

Some more text

## Header 2"
            .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let ast = parse_document(&arena, &text, &Options::default());
        let doc = Document { path, ast, text };
        let rule = MD022::new();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_no_errors_for_setext() {
        let text = "Setext style H1
===============

Some text

```
Some code block
```

Setext style H2
---------------"
            .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let ast = parse_document(&arena, &text, &Options::default());
        let doc = Document { path, ast, text };
        let rule = MD022::new();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_no_errors_for_list() {
        let text = "# Header 1

- Some list item
- Some more list item

## Header 2"
            .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let ast = parse_document(&arena, &text, &Options::default());
        let doc = Document { path, ast, text };
        let rule = MD022::new();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![];
        assert_eq!(actual, expected);
    }
}
