use comrak::nodes::Sourcepos;
use miette::Result;

use crate::Document;
use crate::violation::Violation;

use super::{Metadata, RuleLike, Tag};

#[derive(Default, Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct MD010;

impl MD010 {
    const METADATA: Metadata = Metadata {
        name: "MD010",
        description: "Hard tabs",
        tags: &[Tag::Whitespace, Tag::HardTab],
        aliases: &["no-hard-tabs"],
    };

    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        Self {}
    }
}

impl RuleLike for MD010 {
    #[inline]
    fn metadata(&self) -> &'static Metadata {
        &Self::METADATA
    }

    #[inline]
    fn check(&self, doc: &Document) -> Result<Vec<Violation>> {
        let mut violations = vec![];
        for (i, line) in doc.lines.iter().enumerate() {
            let lineno = i + 1;
            if let Some(idx) = line.find('\t') {
                let position = Sourcepos::from((lineno, idx + 1, lineno, idx + 1));
                let violation = self.to_violation(doc.path.clone(), position);
                violations.push(violation);
            }
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
            Some text

            	* hard tab character used to indent the list item
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD010::new();
        let actual = rule.check(&doc)?;
        let expected = vec![rule.to_violation(path, Sourcepos::from((3, 1, 3, 1)))];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_no_errors() -> Result<()> {
        let text = indoc! {"
            Some text

                * Spaces used to indent the list item instead
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text)?;
        let rule = MD010::new();
        let actual = rule.check(&doc)?;
        let expected = vec![];
        assert_eq!(actual, expected);
        Ok(())
    }
}
