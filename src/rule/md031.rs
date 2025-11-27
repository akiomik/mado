use comrak::nodes::NodeValue;
use miette::Result;

use crate::{Document, violation::Violation};

use super::{Metadata, RuleLike, Tag};

#[derive(Debug, Default, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct MD031;

impl MD031 {
    const METADATA: Metadata = Metadata {
        name: "MD031",
        description: "Fenced code blocks should be surrounded by blank lines",
        tags: &[Tag::Code, Tag::BlankLines],
        aliases: &["blanks-around-fences"],
    };

    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        Self {}
    }
}

impl RuleLike for MD031 {
    #[inline]
    fn metadata(&self) -> &'static Metadata {
        &Self::METADATA
    }

    #[inline]
    fn check(&self, doc: &Document) -> Result<Vec<Violation>> {
        let mut violations = vec![];

        for node in doc.ast.children() {
            if let Some(prev_node) = node.previous_sibling() {
                let prev_position = prev_node.data.borrow().sourcepos;
                let position = node.data.borrow().sourcepos;

                if let NodeValue::CodeBlock(code_block) = &prev_node.data.borrow().value
                    && code_block.fenced
                    && position.start.line == prev_position.end.line + 1
                    && prev_position.end.column != 0
                {
                    let mut fence_position = prev_position;
                    fence_position.start.line = fence_position.end.line;
                    let violation = self.to_violation(doc.path.clone(), fence_position);
                    violations.push(violation);
                }

                if let NodeValue::CodeBlock(code_block) = &node.data.borrow().value
                    && code_block.fenced
                    && position.start.line == prev_position.end.line + 1
                    && prev_position.end.column != 0
                {
                    let mut fence_position = position;
                    fence_position.end.line = fence_position.start.line;
                    let violation = self.to_violation(doc.path.clone(), fence_position);
                    violations.push(violation);
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
    fn check_errors() -> Result<()> {
        let text = "text
```
code
```

text
```
code
```
text

```
code
```
text"
            .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD031::new();
        let actual = rule.check(&doc)?;
        let expected = vec![
            rule.to_violation(path.clone(), Sourcepos::from((2, 1, 2, 3))),
            rule.to_violation(path.clone(), Sourcepos::from((7, 1, 7, 3))),
            rule.to_violation(path.clone(), Sourcepos::from((9, 1, 9, 3))),
            rule.to_violation(path, Sourcepos::from((14, 1, 14, 3))),
        ];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_errors_with_code() -> Result<()> {
        let text = "```
code
```
```
code
```
```
code
```"
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD031::new();
        let actual = rule.check(&doc)?;
        let expected = vec![
            rule.to_violation(path.clone(), Sourcepos::from((3, 1, 3, 3))),
            rule.to_violation(path.clone(), Sourcepos::from((4, 1, 4, 3))),
            rule.to_violation(path.clone(), Sourcepos::from((6, 1, 6, 3))),
            rule.to_violation(path, Sourcepos::from((7, 1, 7, 3))),
        ];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_errors_with_list() -> Result<()> {
        let text = "* list
```
code
```

* list
```
code
```
* list

```
code
```
* list"
            .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD031::new();
        let actual = rule.check(&doc)?;
        let expected = vec![
            rule.to_violation(path.clone(), Sourcepos::from((2, 1, 2, 3))),
            rule.to_violation(path.clone(), Sourcepos::from((7, 1, 7, 3))),
            rule.to_violation(path.clone(), Sourcepos::from((9, 1, 9, 3))),
            rule.to_violation(path, Sourcepos::from((14, 1, 14, 3))),
        ];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_no_errors() -> Result<()> {
        let text = indoc! {"
            text

            ```
            code
            ```

            text

            ```
            code
            ```

            text

            ```
            code
            ```

            text
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text)?;
        let rule = MD031::new();
        let actual = rule.check(&doc)?;
        let expected = vec![];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_noerrors_code() -> Result<()> {
        let text = indoc! {"
            ```
            code
            ```

            ```
            code
            ```

            ```
            code
            ```
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text)?;
        let rule = MD031::new();
        let actual = rule.check(&doc)?;
        let expected = vec![];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_no_errors_indented() -> Result<()> {
        let text = indoc! {"
            Some text
                Code block

                Another code block
            Some more text
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text)?;
        let rule = MD031::new();
        let actual = rule.check(&doc)?;
        let expected = vec![];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_no_errors_with_list() -> Result<()> {
        let text = indoc! {"
            * List

            ```
            Code block
            ```

            ```
            Another code block
            ```

            Some more text
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text)?;
        let rule = MD031::new();
        let actual = rule.check(&doc)?;
        let expected = vec![];
        assert_eq!(actual, expected);
        Ok(())
    }
}
