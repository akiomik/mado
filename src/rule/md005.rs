use std::{collections::HashMap, path::PathBuf};

use comrak::nodes::{AstNode, NodeValue, Sourcepos};
use miette::Result;

use crate::{violation::Violation, Document};

use super::Rule;

#[derive(Default, Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct MD005;

impl MD005 {
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self {}
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

impl Rule for MD005 {
    #[inline]
    fn name(&self) -> String {
        "MD005".to_owned()
    }

    #[inline]
    fn description(&self) -> String {
        "Inconsistent indentation for list items at the same level".to_owned()
    }

    #[inline]
    fn tags(&self) -> Vec<String> {
        vec![
            "bullet".to_owned(),
            "ul".to_owned(),
            "indentation".to_owned(),
        ]
    }

    #[inline]
    fn aliases(&self) -> Vec<String> {
        vec!["list-indent".to_owned()]
    }

    #[inline]
    fn check(&self, doc: &Document) -> Result<Vec<Violation>> {
        let mut violations = vec![];
        let mut levels: HashMap<usize, Sourcepos> = HashMap::new();

        self.check_recursive(doc.ast, &doc.path, &mut violations, &mut levels, 0);

        Ok(violations)
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use comrak::{parse_document, Arena, Options};

    use super::*;

    #[test]
    fn check_errors() {
        let text = "* Item 1
    * Nested item 1
    * Nested item 2
   * A misaligned item";
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let ast = parse_document(&arena, text, &Options::default());
        let doc = Document {
            path: path.clone(),
            ast,
            text: text.to_string(),
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
   *";
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let ast = parse_document(&arena, text, &Options::default());
        let doc = Document {
            path: path.clone(),
            ast,
            text: text.to_string(),
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
   1. More misaligned item";
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let ast = parse_document(&arena, text, &Options::default());
        let doc = Document {
            path: path.clone(),
            ast,
            text: text.to_string(),
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
   * item 4";
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let ast = parse_document(&arena, text, &Options::default());
        let doc = Document {
            path: path.clone(),
            ast,
            text: text.to_string(),
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
    * Nested item 3";
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let ast = parse_document(&arena, text, &Options::default());
        let doc = Document {
            path,
            ast,
            text: text.to_string(),
        };
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
    2. item 4";
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let ast = parse_document(&arena, text, &Options::default());
        let doc = Document {
            path,
            ast,
            text: text.to_string(),
        };
        let rule = MD005::new();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![];
        assert_eq!(actual, expected);
    }
}
