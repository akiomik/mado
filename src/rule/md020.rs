use comrak::nodes::{NodeHeading, NodeValue};
use miette::Result;

use crate::Document;
use crate::violation::Violation;

use super::{Metadata, RuleLike, Tag};

#[derive(Default, Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct MD020;

impl MD020 {
    const METADATA: Metadata = Metadata {
        name: "MD020",
        description: "No space inside hashes on closed atx style header",
        tags: &[Tag::Headers, Tag::AtxClosed, Tag::Spaces],
        aliases: &["no-missing-space-closed-atx"],
    };

    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        Self {}
    }
}

impl RuleLike for MD020 {
    #[inline]
    fn metadata(&self) -> &'static Metadata {
        &Self::METADATA
    }

    #[inline]
    fn check(&self, doc: &Document) -> Result<Vec<Violation>> {
        let mut violations = vec![];

        for node in doc.ast.children() {
            if node.data.borrow().value == NodeValue::Paragraph {
                for child_node in node.children() {
                    if let NodeValue::Text(text) = &child_node.data.borrow().value {
                        let position = node.data.borrow().sourcepos;
                        if position.start.column == 1
                            && text.starts_with('#')
                            && text.ends_with('#')
                        {
                            let violation = self.to_violation(doc.path.clone(), position);
                            violations.push(violation);
                        }
                    }
                }
            }

            if let NodeValue::Heading(NodeHeading { setext: false, .. }) = node.data.borrow().value
                && let Some(child_node) = node.last_child()
                && let NodeValue::Text(text) = &child_node.data.borrow().value
                && text.ends_with('#')
            {
                let position = node.data.borrow().sourcepos;
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

    use comrak::{Arena, nodes::Sourcepos};
    use indoc::indoc;
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn check_errors() -> Result<()> {
        let text = indoc! {"
            #Header 1#

            ## Header 2##

            ##Header 3 ##
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD020::default();
        let actual = rule.check(&doc)?;
        let expected = vec![
            rule.to_violation(path.clone(), Sourcepos::from((1, 1, 1, 10))),
            rule.to_violation(path.clone(), Sourcepos::from((3, 1, 3, 13))),
            rule.to_violation(path, Sourcepos::from((5, 1, 5, 13))),
        ];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_no_errors() -> Result<()> {
        let text = indoc! {"
            # Header 1 #

            ## Header 2 ##
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text)?;
        let rule = MD020::default();
        let actual = rule.check(&doc)?;
        let expected = vec![];
        assert_eq!(actual, expected);
        Ok(())
    }

    // TODO: Support escaped hash
    // #[test]
    // fn check_no_errors_with_escaped_hash() -> Result<()> {
    //     let text = indoc! {"
    //         #Header 1\\#
    //
    //         \\##Header 2##
    //
    //         ## Header 3\\##
    //
    //         \\## Header 4##
    //
    //         ##Header 5 \\##
    //
    //         \\##Header 6 ##
    //     "}
    //     .to_owned();
    //     let path = Path::new("test.md").to_path_buf();
    //     let arena = Arena::new();
    //     let doc = Document::new(&arena, path, text)?;
    //     let rule = MD020::default();
    //     let actual = rule.check(&doc)?;
    //     let expected = vec![];
    //     assert_eq!(actual, expected);
    //     Ok(())
    // }

    #[test]
    fn check_no_errors_with_atx() -> Result<()> {
        let text = indoc! {"
            #Header 1

            ## Header 2

            ##Header 3
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text)?;
        let rule = MD020::default();
        let actual = rule.check(&doc)?;
        let expected = vec![];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_no_errors_with_issue_number() -> Result<()> {
        let text = indoc! {"
            # Header 1 #

            See [#4649](https://example.com) and [#4979](https://example.com) for details.
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text)?;
        let rule = MD020::default();
        let actual = rule.check(&doc)?;
        let expected = vec![];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_no_errors_with_list() -> Result<()> {
        let text = "* #Header1#".to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text)?;
        let rule = MD020::default();
        let actual = rule.check(&doc)?;
        let expected = vec![];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_no_errors_with_code_block_comment() -> Result<()> {
        let text = indoc! {"
            ```
            #Header#
            ```
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text)?;
        let rule = MD020::new();
        let actual = rule.check(&doc)?;
        let expected = vec![];
        assert_eq!(actual, expected);
        Ok(())
    }
}
