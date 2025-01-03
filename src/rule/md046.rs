use comrak::nodes::NodeValue;
use miette::Result;
use serde::Deserialize;

use crate::{violation::Violation, Document};

use super::RuleLike;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum CodeBlockStyle {
    Fenced,
    Indented,
    Consistent,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct MD046 {
    style: CodeBlockStyle,
}

impl MD046 {
    pub const DEFAULT_STYLE: CodeBlockStyle = CodeBlockStyle::Fenced;

    #[inline]
    #[must_use]
    pub fn new(style: CodeBlockStyle) -> Self {
        Self { style }
    }
}

impl Default for MD046 {
    #[inline]
    fn default() -> Self {
        Self {
            style: Self::DEFAULT_STYLE,
        }
    }
}

impl RuleLike for MD046 {
    #[inline]
    fn name(&self) -> &'static str {
        "MD046"
    }

    #[inline]
    fn description(&self) -> &'static str {
        "Code block style"
    }

    #[inline]
    fn tags(&self) -> Vec<&'static str> {
        vec!["code"]
    }

    #[inline]
    fn aliases(&self) -> Vec<&'static str> {
        vec!["code-block-style"]
    }

    #[inline]
    fn check(&self, doc: &Document) -> Result<Vec<Violation>> {
        let mut violations = vec![];
        let mut maybe_first_fenced = None;

        for node in doc.ast.descendants() {
            if let NodeValue::CodeBlock(code) = &node.data.borrow().value {
                let is_violated = match self.style {
                    CodeBlockStyle::Fenced => !code.fenced,
                    CodeBlockStyle::Indented => code.fenced,
                    CodeBlockStyle::Consistent => match maybe_first_fenced {
                        Some(fenced) => code.fenced != fenced,
                        None => false,
                    },
                };

                if is_violated {
                    let position = node.data.borrow().sourcepos;
                    let violation = self.to_violation(doc.path.clone(), position);
                    violations.push(violation);
                }

                if maybe_first_fenced.is_none() {
                    maybe_first_fenced = Some(code.fenced);
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
    fn check_errors_with_fenced() {
        let text = "Some text.

    Code block

Some more text."
            .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let ast = parse_document(&arena, &text, &Options::default());
        let doc = Document {
            path: path.clone(),
            ast,
            text,
        };
        let rule = MD046::default();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![rule.to_violation(path, Sourcepos::from((3, 5, 4, 0)))];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_errors_with_indented() {
        let text = "Some text.

```ruby
Code block
```

Some more text."
            .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let ast = parse_document(&arena, &text, &Options::default());
        let doc = Document {
            path: path.clone(),
            ast,
            text,
        };
        let rule = MD046::new(CodeBlockStyle::Indented);
        let actual = rule.check(&doc).unwrap();
        let expected = vec![rule.to_violation(path, Sourcepos::from((3, 1, 5, 3)))];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_errors_with_consistent() {
        let text = "Some text.

```ruby
Code block
```
Some more text.

    Code block

Some more more text."
            .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let ast = parse_document(&arena, &text, &Options::default());
        let doc = Document {
            path: path.clone(),
            ast,
            text,
        };
        let rule = MD046::new(CodeBlockStyle::Consistent);
        let actual = rule.check(&doc).unwrap();
        let expected = vec![rule.to_violation(path, Sourcepos::from((8, 5, 9, 0)))];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_no_errors_with_fenced() {
        let text = "Some text.

```ruby
Code block
```

Some more text."
            .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let ast = parse_document(&arena, &text, &Options::default());
        let doc = Document { path, ast, text };
        let rule = MD046::default();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_no_errors_with_indented() {
        let text = "Some text.

    Code block

Some more text."
            .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let ast = parse_document(&arena, &text, &Options::default());
        let doc = Document { path, ast, text };
        let rule = MD046::new(CodeBlockStyle::Indented);
        let actual = rule.check(&doc).unwrap();
        let expected = vec![];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_no_errors_with_consistent_fenced() {
        let text = "Some text.

```ruby
Code block
```
Some more text.

```ruby
Code block
```

Some more more text."
            .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let ast = parse_document(&arena, &text, &Options::default());
        let doc = Document { path, ast, text };
        let rule = MD046::new(CodeBlockStyle::Consistent);
        let actual = rule.check(&doc).unwrap();
        let expected = vec![];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_no_errors_with_consistent_indented() {
        let text = "Some text.

    Code block

Some more text.

    Code block

Some more more text."
            .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let ast = parse_document(&arena, &text, &Options::default());
        let doc = Document { path, ast, text };
        let rule = MD046::new(CodeBlockStyle::Consistent);
        let actual = rule.check(&doc).unwrap();
        let expected = vec![];
        assert_eq!(actual, expected);
    }
}
