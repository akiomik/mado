use std::{collections::HashMap, path::PathBuf};

use comrak::nodes::{AstNode, NodeValue, Sourcepos};
use miette::Result;

use crate::{violation::Violation, Document};

use super::{
    node::{NodeContext, NodeRule, NodeValueMatcher},
    NewRuleLike, RuleLike, RuleMetadata,
};

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct State {
    pub levels: HashMap<usize, Sourcepos>,
}

#[derive(Default, Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct MD005 {
    state: State,
}

impl MD005 {
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self {
            state: State::default(),
        }
    }

    fn check_recursive<'a>(
        &self,
        root: &'a AstNode<'a>,
        path: &PathBuf,
        violations: &mut Vec<Violation>,
        levels: &mut HashMap<usize, Sourcepos>,
        level: usize,
    ) {
        for node in root.children() {
            if let NodeValue::List(_) = node.data.borrow().value {
                for item_node in node.children() {
                    if let NodeValue::Item(_) = item_node.data.borrow().value {
                        let position = item_node.data.borrow().sourcepos;
                        match levels.get(&level) {
                            Some(expected_position) => {
                                if position.start.column != expected_position.start.column {
                                    let violation = self.to_violation(path.clone(), position);
                                    violations.push(violation);
                                }
                            }
                            None => {
                                levels.insert(level, position);
                            }
                        }

                        self.check_recursive(item_node, path, violations, levels, level + 1);
                    }
                }
            }
        }
    }
}

impl RuleLike for MD005 {
    #[inline]
    fn name(&self) -> &'static str {
        "MD005"
    }

    #[inline]
    fn description(&self) -> &'static str {
        "Inconsistent indentation for list items at the same level"
    }

    #[inline]
    fn tags(&self) -> Vec<&'static str> {
        vec!["bullet", "ul", "indentation"]
    }

    #[inline]
    fn aliases(&self) -> Vec<&'static str> {
        vec!["list-indent"]
    }

    #[inline]
    fn check(&self, doc: &Document) -> Result<Vec<Violation>> {
        let mut violations = vec![];
        let mut levels: HashMap<usize, Sourcepos> = HashMap::new();

        self.check_recursive(doc.ast, &doc.path, &mut violations, &mut levels, 0);

        Ok(violations)
    }
}

impl NewRuleLike for MD005 {
    fn metadata(&self) -> RuleMetadata {
        RuleMetadata {
            name: "MD005",
            description: "Inconsistent indentation for list items at the same level",
            tags: vec!["bullet", "ul", "indentation"],
            aliases: vec!["list-indent"],
        }
    }
}

impl NodeRule for MD005 {
    fn matcher(&self) -> NodeValueMatcher {
        NodeValueMatcher::new(|node| matches!(node, NodeValue::Item(_)))
    }

    fn run<'a>(&mut self, ctx: &NodeContext, node: &'a AstNode<'a>) -> Result<Vec<Violation>> {
        let mut violations = vec![];

        if let Some(level) = ctx.list_level {
            let position = node.data.borrow().sourcepos;
            match self.state.levels.get(&level) {
                Some(expected_position) => {
                    if position.start.column != expected_position.start.column {
                        let violation = self.to_violation(ctx.path.clone(), position);
                        violations.push(violation);
                    }
                }
                None => {
                    self.state.levels.insert(level, position);
                }
            }
        }

        Ok(violations)
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use comrak::{parse_document, Arena, Options};
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn check_errors() {
        let text = "* Item 1
    * Nested item 1
    * Nested item 2
   * A misaligned item"
            .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let ast = parse_document(&arena, &text, &Options::default());
        let doc = Document {
            path: path.clone(),
            ast,
            text,
        };
        let rule = MD005::new();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![rule.to_violation(path, Sourcepos::from((4, 4, 4, 22)))];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_errors_for_empty_item_text() {
        let text = "*
    *
    *
   *"
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let ast = parse_document(&arena, &text, &Options::default());
        let doc = Document {
            path: path.clone(),
            ast,
            text,
        };
        let rule = MD005::new();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![rule.to_violation(path, Sourcepos::from((4, 4, 4, 4)))];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_errors_for_lists() {
        let text = "* List 1
  * item 1
  * item 2

Some text

1. List 2
   1. A misaligned item
   1. More misaligned item"
            .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let ast = parse_document(&arena, &text, &Options::default());
        let doc = Document {
            path: path.clone(),
            ast,
            text,
        };
        let rule = MD005::new();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![
            rule.to_violation(path.clone(), Sourcepos::from((8, 4, 8, 23))),
            rule.to_violation(path, Sourcepos::from((9, 4, 9, 26))),
        ];
        assert_eq!(actual, expected);
    }

    // NOTE: This test case is not marked as a violation in markdownlint
    #[test]
    fn check_errors_with_test_and_list_in_list() {
        let text = "* List 1
  * Item 1
  * Item 2

1. List 2
   Text in list
   * item 3
   * item 4"
            .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let ast = parse_document(&arena, &text, &Options::default());
        let doc = Document {
            path: path.clone(),
            ast,
            text,
        };
        let rule = MD005::new();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![
            rule.to_violation(path.clone(), Sourcepos::from((7, 4, 7, 11))),
            rule.to_violation(path, Sourcepos::from((8, 4, 8, 11))),
        ];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_no_errors() {
        let text = "* Item 1
    * Nested item 1
    * Nested item 2
    * Nested item 3"
            .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let ast = parse_document(&arena, &text, &Options::default());
        let doc = Document { path, ast, text };
        let rule = MD005::new();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_no_errors_for_lists() {
        let text = "* List 1
    * item 1
    * item 2

Some text

* List 2
    1. item 3
    2. item 4"
            .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let ast = parse_document(&arena, &text, &Options::default());
        let doc = Document { path, ast, text };
        let rule = MD005::new();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![];
        assert_eq!(actual, expected);
    }
}
