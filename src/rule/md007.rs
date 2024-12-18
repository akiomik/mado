use comrak::nodes::NodeValue;
use miette::Result;

use crate::{violation::Violation, Document};

use super::RuleLike;

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct MD007 {
    indent: usize,
}

impl MD007 {
    #[inline]
    #[must_use]
    pub fn new(indent: usize) -> Self {
        Self { indent }
    }
}

impl Default for MD007 {
    #[inline]
    fn default() -> Self {
        Self { indent: 4 }
    }
}

impl RuleLike for MD007 {
    #[inline]
    fn name(&self) -> String {
        "MD007".to_owned()
    }

    #[inline]
    fn description(&self) -> String {
        "Unordered list indentation".to_owned()
    }

    #[inline]
    fn tags(&self) -> Vec<String> {
        vec![
            "bullet".to_owned(),
            "ul".to_owned(),
            "indentation".to_owned(),
        ]
    }

    #[inline]
    fn aliases(&self) -> Vec<String> {
        vec!["ul-indent".to_owned()]
    }

    #[inline]
    fn check(&self, doc: &Document) -> Result<Vec<Violation>> {
        let mut violations = vec![];

        for node in doc.ast.descendants() {
            if let NodeValue::Item(_) = node.data.borrow().value {
                // TODO: Calculate based on nested levels, not modulo
                let position = node.data.borrow().sourcepos;
                let indent = position.start.column - 1;
                if indent % self.indent != 0 {
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

    use comrak::{nodes::Sourcepos, parse_document, Arena, Options};
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn check_errors() {
        let text = "* List item
   * Nested list item indented by 3 spaces"
            .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let ast = parse_document(&arena, &text, &Options::default());
        let doc = Document {
            path: path.clone(),
            ast,
            text,
        };
        let rule = MD007::default();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![rule.to_violation(path.clone(), Sourcepos::from((2, 4, 2, 42)))];
        assert_eq!(actual, expected);
    }

    // TODO: This test case should pass
    // #[test]
    // fn check_errors_for_multiple_indentation() {
    //     let text = "* List item
    // * Nested list item indented by 4 spaces".to_owned();
    //     let path = Path::new("test.md").to_path_buf();
    //     let arena = Arena::new();
    //     let ast = parse_document(&arena, &text, &Options::default());
    //     let doc = Document {
    //         path: path.clone(),
    //         ast,
    //         text,
    //     };
    //     let rule = MD007::new(2);
    //     let actual = rule.check(&doc).unwrap();
    //     let expected = vec![rule.to_violation(path, Sourcepos::from((2, 5, 2, 43)))];
    //     assert_eq!(actual, expected);
    // }

    #[test]
    fn check_no_errors() {
        let text = "* List item
    * Nested list item indented by 4 spaces"
            .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let ast = parse_document(&arena, &text, &Options::default());
        let doc = Document { path, ast, text };
        let rule = MD007::default();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![];
        assert_eq!(actual, expected);
    }
}
