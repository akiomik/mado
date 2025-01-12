use comrak::nodes::{NodeHeading, NodeValue};
use miette::Result;

use crate::violation::Violation;
use crate::Document;

use super::RuleLike;

#[derive(Default, Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct MD021;

impl MD021 {
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self {}
    }
}

impl RuleLike for MD021 {
    #[inline]
    fn name(&self) -> &'static str {
        "MD021"
    }

    #[inline]
    fn description(&self) -> &'static str {
        "Multiple spaces inside hashes on closed atx style header"
    }

    #[inline]
    fn tags(&self) -> &'static [&'static str] {
        &["headers", "atx_closed", "spaces"]
    }

    #[inline]
    fn aliases(&self) -> &'static [&'static str] {
        &["no-multiple-space-closed-atx"]
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
            {
                if let (Some(first_node), Some(last_node)) = (node.first_child(), node.last_child())
                {
                    let heading_position = node.data.borrow().sourcepos;
                    let first_position = first_node.data.borrow().sourcepos;
                    let last_position = last_node.data.borrow().sourcepos;
                    let is_atx_closed = heading_position.end.column > last_position.end.column;

                    let expected_offset = (*level as usize) + 1;
                    if is_atx_closed
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
        }

        Ok(violations)
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use comrak::{nodes::Sourcepos, Arena};
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn check_errors() {
        let text = "#  Header 1  #

## Header 2  ##

##  Header 3 ##"
            .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text).unwrap();
        let rule = MD021::new();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![
            rule.to_violation(path.clone(), Sourcepos::from((1, 1, 1, 14))),
            rule.to_violation(path.clone(), Sourcepos::from((3, 1, 3, 15))),
            rule.to_violation(path, Sourcepos::from((5, 1, 5, 15))),
        ];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_no_errors() {
        let text = "# Header 1 #

## Header 2 ##"
            .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text).unwrap();
        let rule = MD021::new();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_no_errors_with_escaped_hash() {
        let text = "#  Header 1  \\#

\\##  Header 2  ##

## Header 3  \\##

\\## Header 4  ##

##  Header 5 \\##

\\##  Header 6 ##"
            .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text).unwrap();
        let rule = MD021::default();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_no_errors_with_atx() {
        let text = "#  Header 1

##  Header 2"
            .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text).unwrap();
        let rule = MD021::new();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_no_errors_with_setext() {
        let text = "  Header 1
========"
            .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text).unwrap();
        let rule = MD021::new();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_no_errors_for_multiple_children() {
        let text = "# Header with `code` and text #".to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text).unwrap();
        let rule = MD021::new();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![];
        assert_eq!(actual, expected);
    }
}
