use comrak::nodes::NodeValue;
use miette::Result;
use serde::{Deserialize, Serialize};

use crate::{Document, violation::Violation};

use super::{Metadata, RuleLike, Tag};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
    const METADATA: Metadata = Metadata {
        name: "MD046",
        description: "Code block style",
        tags: &[Tag::Code],
        aliases: &["code-block-style"],
    };

    pub const DEFAULT_STYLE: CodeBlockStyle = CodeBlockStyle::Fenced;

    #[inline]
    #[must_use]
    pub const fn new(style: CodeBlockStyle) -> Self {
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
    fn metadata(&self) -> &'static Metadata {
        &Self::METADATA
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
                    CodeBlockStyle::Consistent => {
                        maybe_first_fenced.is_some_and(|fenced| code.fenced != fenced)
                    }
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

    use comrak::{Arena, nodes::Sourcepos};
    use indoc::indoc;
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn check_errors_with_fenced() -> Result<()> {
        let text = indoc! {"
            Some text.

                Code block

            Some more text.
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD046::default();
        let actual = rule.check(&doc)?;
        let expected = vec![rule.to_violation(path, Sourcepos::from((3, 5, 4, 0)))];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_errors_with_indented() -> Result<()> {
        let text = indoc! {"
            Some text.

            ```ruby
            Code block
            ```

            Some more text.
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD046::new(CodeBlockStyle::Indented);
        let actual = rule.check(&doc)?;
        let expected = vec![rule.to_violation(path, Sourcepos::from((3, 1, 5, 3)))];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_errors_with_consistent() -> Result<()> {
        let text = indoc! {"
            Some text.

            ```ruby
            Code block
            ```
            Some more text.

                Code block

            Some more more text.
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD046::new(CodeBlockStyle::Consistent);
        let actual = rule.check(&doc)?;
        let expected = vec![rule.to_violation(path, Sourcepos::from((8, 5, 9, 0)))];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_no_errors_with_fenced() -> Result<()> {
        let text = indoc! {"
            Some text.

            ```ruby
            Code block
            ```

            Some more text.
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text)?;
        let rule = MD046::default();
        let actual = rule.check(&doc)?;
        let expected = vec![];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_no_errors_with_indented() -> Result<()> {
        let text = indoc! {"
            Some text.

                Code block

            Some more text.
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text)?;
        let rule = MD046::new(CodeBlockStyle::Indented);
        let actual = rule.check(&doc)?;
        let expected = vec![];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_no_errors_with_consistent_fenced() -> Result<()> {
        let text = indoc! {"
            Some text.

            ```ruby
            Code block
            ```
            Some more text.

            ```ruby
            Code block
            ```

            Some more more text.
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text)?;
        let rule = MD046::new(CodeBlockStyle::Consistent);
        let actual = rule.check(&doc)?;
        let expected = vec![];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_no_errors_with_consistent_indented() -> Result<()> {
        let text = indoc! {"
            Some text.

                Code block

            Some more text.

                Code block

            Some more more text.
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text)?;
        let rule = MD046::new(CodeBlockStyle::Consistent);
        let actual = rule.check(&doc)?;
        let expected = vec![];
        assert_eq!(actual, expected);
        Ok(())
    }
}
