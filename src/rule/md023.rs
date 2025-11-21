use comrak::nodes::NodeValue;
use miette::Result;

use crate::{Document, violation::Violation};

use super::{Metadata, RuleLike, Tag};

#[derive(Default, Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct MD023;

impl MD023 {
    const METADATA: Metadata = Metadata {
        name: "MD023",
        description: "Headers must start at the beginning of the line",
        tags: &[Tag::Headers, Tag::Spaces],
        aliases: &["header-start-left"],
    };

    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        Self {}
    }
}

impl RuleLike for MD023 {
    #[inline]
    fn metadata(&self) -> &'static Metadata {
        &Self::METADATA
    }

    #[inline]
    fn check(&self, doc: &Document) -> Result<Vec<Violation>> {
        let mut violations = vec![];

        for node in doc.ast.children() {
            if let NodeValue::Heading(_) = node.data.borrow().value {
                let position = node.data.borrow().sourcepos;
                if position.start.column > 1 {
                    let violation = self.to_violation(doc.path.clone(), position);
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
        let text = indoc! {"
            Some text

              # Indented header
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD023::new();
        let actual = rule.check(&doc)?;
        let expected = vec![rule.to_violation(path, Sourcepos::from((3, 3, 3, 19)))];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_no_errors() -> Result<()> {
        let text = indoc! {"
            Some text

            # Header
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text)?;
        let rule = MD023::new();
        let actual = rule.check(&doc)?;
        let expected = vec![];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_no_errors_with_indented_code_block_comment() -> Result<()> {
        let text = indoc! {"
            Some text

               ```
               # Header
               ```
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text)?;
        let rule = MD023::new();
        let actual = rule.check(&doc)?;
        let expected = vec![];
        assert_eq!(actual, expected);
        Ok(())
    }
}
