use comrak::nodes::{NodeHeading, NodeValue};
use miette::Result;

use crate::Document;
use crate::violation::Violation;

use super::{Metadata, RuleLike, Tag};

#[derive(Default, Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct MD019;

impl MD019 {
    const METADATA: Metadata = Metadata {
        name: "MD019",
        description: "Multiple spaces after hash on atx style header",
        tags: &[Tag::Headers, Tag::Atx, Tag::Spaces],
        aliases: &["no-multiple-space-atx"],
    };

    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        Self {}
    }
}

impl RuleLike for MD019 {
    #[inline]
    fn metadata(&self) -> &'static Metadata {
        &Self::METADATA
    }

    #[inline]
    fn check(&self, doc: &Document) -> Result<Vec<Violation>> {
        let mut violations = vec![];

        for node in doc.ast.children() {
            if let NodeValue::Heading(NodeHeading {
                setext: false,
                level,
                ..
            }) = &node.data.borrow().value
                && let (Some(first_node), Some(last_node)) = (node.first_child(), node.last_child())
            {
                let heading_position = node.data.borrow().sourcepos;
                let first_position = first_node.data.borrow().sourcepos;
                let last_position = last_node.data.borrow().sourcepos;
                let is_atx = heading_position.end.column == last_position.end.column;

                let expected_offset = (*level as usize) + 1;
                if is_atx
                    && ((heading_position.start.column
                        < first_position.start.column - expected_offset)
                        || (heading_position.end.column
                            > last_position.end.column + expected_offset))
                {
                    let violation = self.to_violation(doc.path.clone(), heading_position);
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
            #  Header 1

            ##  Header 2
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD019::new();
        let actual = rule.check(&doc)?;
        let expected = vec![
            rule.to_violation(path.clone(), Sourcepos::from((1, 1, 1, 11))),
            rule.to_violation(path, Sourcepos::from((3, 1, 3, 12))),
        ];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_no_errors() -> Result<()> {
        let text = indoc! {"
            # Header 1

            ## Header 2
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text)?;
        let rule = MD019::new();
        let actual = rule.check(&doc)?;
        let expected = vec![];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_no_errors_with_atx_closed() -> Result<()> {
        let text = indoc! {"
            #  Header 1  #

            ## Header 2  ##

            ##  Header 3 ##
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text)?;
        let rule = MD019::new();
        let actual = rule.check(&doc)?;
        let expected = vec![];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_no_errors_with_setext() -> Result<()> {
        let text = indoc! {"
              Header 1
            ==========
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text)?;
        let rule = MD019::new();
        let actual = rule.check(&doc)?;
        let expected = vec![];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_no_errors_for_multiple_children() -> Result<()> {
        let text = "# Header with `code` and text".to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text)?;
        let rule = MD019::new();
        let actual = rule.check(&doc)?;
        let expected = vec![];
        assert_eq!(actual, expected);
        Ok(())
    }
}
