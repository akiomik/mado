use comrak::nodes::{NodeValue, Sourcepos};
use miette::Result;

use crate::{Document, collection::RangeSet, violation::Violation};

use super::{Metadata, RuleLike, Tag};

#[derive(Default, Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct MD012;

impl MD012 {
    const METADATA: Metadata = Metadata {
        name: "MD012",
        description: "Multiple consecutive blank lines",
        tags: &[Tag::Whitespace, Tag::BlankLines],
        aliases: &["no-multiple-blanks"],
    };

    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        Self {}
    }
}

impl RuleLike for MD012 {
    #[inline]
    fn metadata(&self) -> &'static Metadata {
        &Self::METADATA
    }

    // TODO: Improve codes
    #[inline]
    fn check(&self, doc: &Document) -> Result<Vec<Violation>> {
        let mut violations = vec![];
        let mut maybe_prev_line: Option<&str> = None;
        let mut code_block_ranges = RangeSet::new();

        for node in doc.ast.descendants() {
            if let NodeValue::CodeBlock(_) = node.data.borrow().value {
                let position = node.data.borrow().sourcepos;
                let range = position.start.line..=position.end.line;
                code_block_ranges.insert(range);
            }
        }

        for (i, line) in doc.lines.iter().enumerate() {
            let lineno = i + 1;

            if let Some(prev_line) = maybe_prev_line
                && prev_line.is_empty()
                && line.is_empty()
                && !code_block_ranges.contains(&lineno)
            {
                let position = Sourcepos::from((lineno, 1, lineno, 1));
                let violation = self.to_violation(doc.path.clone(), position);
                violations.push(violation);
            }

            maybe_prev_line = Some(line);
        }

        Ok(violations)
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use comrak::Arena;
    use indoc::indoc;
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn check_errors() -> Result<()> {
        let text = indoc! {"
            Some text here


            Some more text here
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD012::new();
        let actual = rule.check(&doc)?;
        let expected = vec![rule.to_violation(path, Sourcepos::from((3, 1, 3, 1)))];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_errors_with_front_matter() -> Result<()> {
        let text = indoc! {"
            ---
            foo:
            ---


            Some text
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD012::new();
        let actual = rule.check(&doc)?;
        let expected = vec![rule.to_violation(path, Sourcepos::from((5, 1, 5, 1)))];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_no_errors() -> Result<()> {
        let text = indoc! {"
            Some text here

            Some more text here
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text)?;
        let rule = MD012::new();
        let actual = rule.check(&doc)?;
        let expected = vec![];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_no_errors_with_code_block() -> Result<()> {
        let text = indoc! {"
            Some text here

            ```
            This is a code block


            Some code here
            ```

            Some more text here
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text)?;
        let rule = MD012::new();
        let actual = rule.check(&doc)?;
        let expected = vec![];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_no_errors_with_nested_code_block() -> Result<()> {
        let text = indoc! {"
            * List

              ```
              This is a code block


              Some code here
              ```
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text)?;
        let rule = MD012::new();
        let actual = rule.check(&doc)?;
        let expected = vec![];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_no_errors_with_front_matter_and_code_block() -> Result<()> {
        let text = indoc! {"
            ---
            foo:
            bar:
            baz:
            qux:
            ---

            Some text here

            ```
            This is a code block


            Some code here
            ```

            Some more text here
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text)?;
        let rule = MD012::new();
        let actual = rule.check(&doc)?;
        let expected = vec![];
        assert_eq!(actual, expected);
        Ok(())
    }
}
