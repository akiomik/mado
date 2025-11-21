use comrak::nodes::NodeValue;
use miette::Result;

use crate::{Document, violation::Violation};

use super::{Metadata, RuleLike, Tag};

#[derive(Default, Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct MD022;

impl MD022 {
    const METADATA: Metadata = Metadata {
        name: "MD022",
        description: "Headers should be surrounded by blank lines",
        tags: &[Tag::Headers, Tag::BlankLines],
        aliases: &["blanks-around-headers"],
    };

    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        Self {}
    }
}

impl RuleLike for MD022 {
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

                match (&prev_node.data.borrow().value, &node.data.borrow().value) {
                    (NodeValue::Heading(_), _) => {
                        if position.start.line == prev_position.end.line + 1 {
                            let violation = self.to_violation(doc.path.clone(), prev_position);
                            violations.push(violation);
                        }
                    }
                    (_, NodeValue::Heading(_)) => {
                        // NOTE: Ignore column 0, as the List may end on the next line
                        if position.start.line == prev_position.end.line + 1
                            && prev_position.end.column != 0
                        {
                            let violation = self.to_violation(doc.path.clone(), position);
                            violations.push(violation);
                        }
                    }
                    _ => {}
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
    fn check_errors_for_atx() -> Result<()> {
        let text = indoc! {"
            # Header 1
            Some text

            Some more text
            ## Header 2
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD022::new();
        let actual = rule.check(&doc)?;
        let expected = vec![
            rule.to_violation(path.clone(), Sourcepos::from((1, 1, 1, 10))),
            rule.to_violation(path, Sourcepos::from((5, 1, 5, 11))),
        ];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_errors_for_setext() -> Result<()> {
        let text = indoc! {"
            Setext style H1
            ===============
            Some text

            ```
            Some code block
            ```
            Setext style H2
            ---------------
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD022::new();
        let actual = rule.check(&doc)?;
        let expected = vec![
            rule.to_violation(path.clone(), Sourcepos::from((1, 1, 2, 15))),
            rule.to_violation(path, Sourcepos::from((8, 1, 9, 15))),
        ];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_no_errors() -> Result<()> {
        let text = indoc! {"
            # Header 1

            Some text

            Some more text

            ## Header 2
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text)?;
        let rule = MD022::new();
        let actual = rule.check(&doc)?;
        let expected = vec![];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_no_errors_for_setext() -> Result<()> {
        let text = indoc! {"
            Setext style H1
            ===============

            Some text

            ```
            Some code block
            ```

            Setext style H2
            ---------------
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text)?;
        let rule = MD022::new();
        let actual = rule.check(&doc)?;
        let expected = vec![];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_no_errors_for_list() -> Result<()> {
        let text = indoc! {"
            # Header 1

            - Some list item
            - Some more list item

            ## Header 2
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text)?;
        let rule = MD022::new();
        let actual = rule.check(&doc)?;
        let expected = vec![];
        assert_eq!(actual, expected);
        Ok(())
    }
}
