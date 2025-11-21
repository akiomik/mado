use std::path::PathBuf;

use comrak::nodes::{AstNode, NodeValue, Sourcepos};
use miette::Result;

use crate::{Document, violation::Violation};

use super::{Metadata, RuleLike, Tag};

#[derive(Debug, Default, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct MD028;

impl MD028 {
    const METADATA: Metadata = Metadata {
        name: "MD028",
        description: "Blank line inside blockquote",
        tags: &[Tag::Blockquote, Tag::Whitespace],
        aliases: &["no-blanks-blockquote"],
    };

    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        Self {}
    }

    fn check_recursive<'a>(
        &self,
        root: &'a AstNode<'a>,
        path: &PathBuf,
        violations: &mut Vec<Violation>,
    ) {
        for node in root.children() {
            if let Some(prev_node) = node.previous_sibling()
                && (&prev_node.data.borrow().value, &node.data.borrow().value)
                    == (&NodeValue::BlockQuote, &NodeValue::BlockQuote)
            {
                let prev_position = prev_node.data.borrow().sourcepos;
                let position = node.data.borrow().sourcepos;
                let blank_line_position =
                    Sourcepos::from((prev_position.end.line + 1, 1, position.start.line - 1, 1));
                let violation = self.to_violation(path.clone(), blank_line_position);
                violations.push(violation);
            }

            if let NodeValue::List(_) = node.data.borrow().value {
                for item_node in node.children() {
                    self.check_recursive(item_node, path, violations);
                }
            }
        }
    }
}

impl RuleLike for MD028 {
    #[inline]
    fn metadata(&self) -> &'static Metadata {
        &Self::METADATA
    }

    #[inline]
    fn check(&self, doc: &Document) -> Result<Vec<Violation>> {
        let mut violations = vec![];

        self.check_recursive(doc.ast, &doc.path, &mut violations);

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

            > a quote
            > same quote

            > blank line above this


            > two blank lines above this
             
            > space above this

            * List with embedded blockquote

              > Test
              > Test

              > Test

            * Item 2

              > Test. The blank line below should _not_ trigger MD028 as one blockquote is
              > inside the list, and the other is outside it.

            > Test
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD028::new();
        let actual = rule.check(&doc)?;
        let expected = vec![
            rule.to_violation(path.clone(), Sourcepos::from((5, 1, 5, 1))),
            // NOTE: This ranged result may differ from mdl
            rule.to_violation(path.clone(), Sourcepos::from((7, 1, 8, 1))),
            rule.to_violation(path.clone(), Sourcepos::from((10, 1, 10, 1))),
            rule.to_violation(path, Sourcepos::from((17, 1, 17, 1))),
        ];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_no_errors() -> Result<()> {
        let text = indoc! {"
            Some text

            > a quote
            > same quote
            >
            > blank line above this
            >
            >
            > two blank lines above this
            > 
            > space above this

            * List with embedded blockquote

              > Test
              > Test
              >
              > Test

            * Item 2

              > Test. The blank line below should _not_ trigger MD028 as one blockquote is
              > inside the list, and the other is outside it.

            > Test
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text)?;
        let rule = MD028::new();
        let actual = rule.check(&doc)?;
        let expected = vec![];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_no_errors_with_some_text_in_between() -> Result<()> {
        let text = indoc! {"
            > This is a blockquote.

            And Jimmy also said:

            > This too is a blockquote.
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text)?;
        let rule = MD028::new();
        let actual = rule.check(&doc)?;
        let expected = vec![];
        assert_eq!(actual, expected);
        Ok(())
    }

    // NOTE: This case may differ from mdl
    // #[test]
    // fn check_no_errors_nested() -> Result<()> {
    //     let text = indoc! {"
    //         * List
    //         > This is a blockquote
    //         > which is immediately followed by
    //
    //         > this blockquote. Unfortunately
    //         > In some parsers, these are treated as the same blockquote.
    //     "}
    //     .to_owned();
    //     let path = Path::new("test.md").to_path_buf();
    //     let arena = Arena::new();
    //     let doc = Document::new(&arena, path, text)?;
    //     let rule = MD028::new();
    //     let actual = rule.check(&doc)?;
    //     let expected = vec![];
    //     assert_eq!(actual, expected);
    //     Ok(())
    // }
}
