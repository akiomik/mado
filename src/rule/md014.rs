use comrak::nodes::{AstNode, NodeValue};
use miette::Result;

use crate::{violation::Violation, Document};

use super::{
    node::{NodeContext, NodeRule, NodeValueMatcher},
    NewRuleLike, RuleLike, RuleMetadata,
};

#[derive(Default, Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct MD014;

impl MD014 {
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self {}
    }
}

impl RuleLike for MD014 {
    #[inline]
    fn name(&self) -> &'static str {
        "MD014"
    }

    #[inline]
    fn description(&self) -> &'static str {
        "Dollar signs used before commands without showing output"
    }

    #[inline]
    fn tags(&self) -> Vec<&'static str> {
        vec!["code"]
    }

    #[inline]
    fn aliases(&self) -> Vec<&'static str> {
        vec!["commands-show-output"]
    }

    #[inline]
    fn check(&self, doc: &Document) -> Result<Vec<Violation>> {
        let mut violations = vec![];

        for node in doc.ast.descendants() {
            if let NodeValue::CodeBlock(code) = &node.data.borrow().value {
                let mut lines = code.literal.lines();
                if lines.all(|line| line.starts_with("$ ")) {
                    let position = node.data.borrow().sourcepos;
                    let violation = self.to_violation(doc.path.clone(), position);
                    violations.push(violation);
                }
            }
        }

        Ok(violations)
    }
}

impl NewRuleLike for MD014 {
    fn metadata(&self) -> RuleMetadata {
        RuleMetadata {
            name: "MD014",
            description: "Dollar signs used before commands without showing output",
            tags: vec!["code"],
            aliases: vec!["commands-show-output"],
        }
    }
}

impl NodeRule for MD014 {
    fn matcher(&self) -> NodeValueMatcher {
        NodeValueMatcher::new(|node| matches!(node, NodeValue::CodeBlock(_)))
    }

    fn run<'a>(&mut self, ctx: &NodeContext, node: &'a AstNode<'a>) -> Result<Vec<Violation>> {
        let mut violations = vec![];
        if let NodeValue::CodeBlock(code) = &node.data.borrow().value {
            let mut lines = code.literal.lines();
            if lines.all(|line| line.starts_with("$ ")) {
                let position = node.data.borrow().sourcepos;
                let violation = self.to_violation(ctx.path.clone(), position);
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
        let text = "```
$ ls
$ cat foo
$ less bar
```"
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let ast = parse_document(&arena, &text, &Options::default());
        let doc = Document {
            path: path.clone(),
            ast,
            text,
        };
        let rule = MD014::default();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![rule.to_violation(path, Sourcepos::from((1, 1, 5, 3)))];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_errors_with_list() {
        let text = "* List

  ```
  $ ls
  $ cat foo
  $ less bar
  ```"
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let ast = parse_document(&arena, &text, &Options::default());
        let doc = Document {
            path: path.clone(),
            ast,
            text,
        };
        let rule = MD014::default();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![rule.to_violation(path, Sourcepos::from((3, 3, 7, 5)))];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_no_errors_no_dollars() {
        let text = "```
ls
cat foo
less bar
```"
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let ast = parse_document(&arena, &text, &Options::default());
        let doc = Document { path, ast, text };
        let rule = MD014::default();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_no_errors_showing_outputs() {
        let text = "```
$ ls
foo bar
$ cat foo
Hello world
$ cat bar
baz
```"
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let ast = parse_document(&arena, &text, &Options::default());
        let doc = Document { path, ast, text };
        let rule = MD014::default();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_no_errors_no_dollars_with_list() {
        let text = "* List
  ```
  ls
  cat foo
  less bar
  ```"
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let ast = parse_document(&arena, &text, &Options::default());
        let doc = Document { path, ast, text };
        let rule = MD014::default();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_no_errors_showing_outputs_with_list() {
        let text = "* List
  ```
  $ ls
  foo bar
  $ cat foo
  Hello world
  $ cat bar
  baz
  ```"
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let ast = parse_document(&arena, &text, &Options::default());
        let doc = Document { path, ast, text };
        let rule = MD014::default();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![];
        assert_eq!(actual, expected);
    }
}
