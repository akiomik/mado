use comrak::nodes::NodeValue;
use miette::Result;

use crate::{violation::Violation, Document};

use super::{tag::Tag, Metadata, RuleLike};

#[derive(Debug, Default, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct MD032;

impl MD032 {
    const METADATA: Metadata = Metadata {
        name: "MD032",
        description: "Lists should be surrounded by blank lines",
        tags: &[Tag::Bullet, Tag::Ul, Tag::Ol, Tag::BlankLines],
        aliases: &["blanks-around-lists"],
    };

    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        Self {}
    }
}

impl RuleLike for MD032 {
    #[inline]
    fn metadata(&self) -> &'static Metadata {
        &Self::METADATA
    }

    #[inline]
    fn check(&self, doc: &Document) -> Result<Vec<Violation>> {
        let mut violations = vec![];

        'node_loop: for node in doc.ast.children() {
            // Check Paragraph in Item
            if let (NodeValue::List(_), Some(item_node)) =
                (&node.data.borrow().value, node.last_child())
            {
                if let Some(block_node) = item_node.last_child() {
                    for inline_node in block_node.descendants() {
                        let inline_position = inline_node.data.borrow().sourcepos;
                        if inline_position.start.column == 1 {
                            // TODO: Improve position
                            let mut bottom_position = inline_position;
                            bottom_position.end.line = bottom_position.start.line;
                            bottom_position.end.column = 0;
                            bottom_position.start.line -= 1;

                            let violation = self.to_violation(doc.path.clone(), bottom_position);
                            violations.push(violation);
                            continue 'node_loop;
                        }
                    }
                }
            }

            if let Some(prev_node) = node.previous_sibling() {
                let position = node.data.borrow().sourcepos;
                let prev_position = prev_node.data.borrow().sourcepos;

                if let NodeValue::List(_) = prev_node.data.borrow().value {
                    if position.start.line == prev_position.end.line + 1
                        && prev_position.end.column != 0
                    {
                        let mut bottom_position = prev_position;
                        bottom_position.start.line = bottom_position.end.line;
                        let violation = self.to_violation(doc.path.clone(), bottom_position);
                        violations.push(violation);
                        continue;
                    }
                }

                if let NodeValue::List(_) = node.data.borrow().value {
                    if position.start.line == prev_position.end.line + 1
                        && prev_position.end.column != 0
                    {
                        let mut top_position = position;
                        if position.end.column == 0 {
                            top_position.end.line = position.start.line + 1;
                        }
                        let violation = self.to_violation(doc.path.clone(), top_position);
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

    use comrak::{nodes::Sourcepos, Arena};
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn check_errors() {
        let text = "text
* list
text
- list
text
1. list
text"
            .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text).unwrap();
        let rule = MD032::new();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![
            rule.to_violation(path.clone(), Sourcepos::from((2, 1, 3, 0))),
            rule.to_violation(path.clone(), Sourcepos::from((4, 1, 5, 0))),
            rule.to_violation(path, Sourcepos::from((6, 1, 7, 0))),
        ];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_errors_with_blank_lines() {
        let text = "text
* list

text
- list
text

1. list
text"
            .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text).unwrap();
        let rule = MD032::new();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![
            rule.to_violation(path.clone(), Sourcepos::from((2, 1, 3, 0))),
            rule.to_violation(path.clone(), Sourcepos::from((5, 1, 6, 0))),
            rule.to_violation(path, Sourcepos::from((8, 1, 9, 0))),
        ];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_errors_with_indented_text_and_blank_lines() {
        let text = "1. list
   text

   text
text"
            .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text).unwrap();
        let rule = MD032::new();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![rule.to_violation(path, Sourcepos::from((4, 1, 5, 0)))];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_errors_nested() {
        let text = "1. list
    * nested list
text"
            .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text).unwrap();
        let rule = MD032::new();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![rule.to_violation(path, Sourcepos::from((2, 1, 3, 0)))];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_errors_nested_with_blank_line() {
        let text = "1. list
   text

   * nested list
text"
            .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text).unwrap();
        let rule = MD032::new();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![rule.to_violation(path, Sourcepos::from((4, 1, 5, 0)))];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_errors_with_code_block() {
        let text = "```
code
```
* list

```
code
```
- list
```
code
```

* list
```
code
```"
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text).unwrap();
        let rule = MD032::new();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![
            // TODO: deduplicate
            rule.to_violation(path.clone(), Sourcepos::from((4, 1, 5, 0))),
            rule.to_violation(path.clone(), Sourcepos::from((9, 1, 9, 6))),
            rule.to_violation(path.clone(), Sourcepos::from((9, 1, 9, 6))),
            rule.to_violation(path, Sourcepos::from((14, 1, 14, 6))),
        ];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_errors_with_code_block_and_blank_line() {
        let text = "1. list
   text

   text
```
code
```"
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text).unwrap();
        let rule = MD032::new();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![rule.to_violation(path, Sourcepos::from((4, 1, 4, 7)))];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_errors_nested_with_code_block() {
        let text = "1. list
   text

   * nested list
```
code
```"
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text).unwrap();
        let rule = MD032::new();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![rule.to_violation(path, Sourcepos::from((4, 1, 4, 16)))];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_no_errors() {
        let text = "text

* list

text

- list

text

1. list

text"
            .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text).unwrap();
        let rule = MD032::new();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_no_errors_with_indented_text() {
        let text = "1. list
   text"
            .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text).unwrap();
        let rule = MD032::new();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_no_errors_nested() {
        let text = "1. list
    * nested list

text"
            .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text).unwrap();
        let rule = MD032::new();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_no_errors_nested_with_blank_line() {
        let text = "1. list
   text

   * nested list

text"
            .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text).unwrap();
        let rule = MD032::new();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_no_errors_with_code_block() {
        let text = "```
code
```

* list

```
code
```

- list

```
code
```

* list

```
code
```"
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text).unwrap();
        let rule = MD032::new();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![];
        assert_eq!(actual, expected);
    }

    // NOTE: mdl triggers a violation
    #[test]
    fn check_no_errors_with_ordered_list_like_paragraph() {
        let text = "* list
  text

text
10. list
20. list"
            .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text).unwrap();
        let rule = MD032::new();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![];
        assert_eq!(actual, expected);
    }
}
