use core::cell::RefCell;

use comrak::nodes::{Ast, AstNode, NodeValue};
use miette::Result;

use crate::{violation::Violation, Document};

use super::{
    node::{NodeContext, NodeValueMatcher},
    Rule, RuleLike, RuleMetadata,
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

        for node in doc.ast.children() {
            if let Some(prev_node) = node.previous_sibling() {
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
        }

        Ok(violations)
    }
}

impl<'a> Rule<&NodeContext, &'a AstNode<'a>, NodeValueMatcher> for MD022 {
    #[inline]
    fn metadata(&self) -> RuleMetadata {
        RuleMetadata {
            name: "MD022",
            description: "Headers should be surrounded by blank lines",
            tags: vec!["headers", "blank_lines"],
            aliases: vec!["blanks-around-headers"],
        }
    }

    #[inline]
    fn matcher(&self) -> NodeValueMatcher {
        NodeValueMatcher::new(|_| true)
    }

    #[inline]
    fn run(&mut self, ctx: &NodeContext, node: &'a AstNode<'a>) -> Result<Vec<Violation>> {
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

    #[inline]
    fn reset(&mut self) {
        self.state = State::default();
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use comrak::{nodes::Sourcepos, Arena};
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
        let doc = Document::new(&arena, path.clone(), text).unwrap();
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
        let doc = Document::new(&arena, path.clone(), text).unwrap();
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
        let doc = Document::new(&arena, path, text).unwrap();
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
        let doc = Document::new(&arena, path, text).unwrap();
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
        let doc = Document::new(&arena, path, text).unwrap();
        let rule = MD022::new();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![];
        assert_eq!(actual, expected);
    }
}
