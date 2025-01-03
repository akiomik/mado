use comrak::nodes::NodeValue;
use miette::Result;

use crate::{violation::Violation, Document};

use super::RuleLike;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct MD038;

impl MD038 {
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self {}
    }
}

impl RuleLike for MD038 {
    #[inline]
    fn name(&self) -> &'static str {
        "MD038"
    }

    #[inline]
    fn description(&self) -> &'static str {
        "Spaces inside code span elements"
    }

    #[inline]
    fn tags(&self) -> Vec<&'static str> {
        vec!["whitespace", "code"]
    }

    #[inline]
    fn aliases(&self) -> Vec<&'static str> {
        vec!["no-space-in-code"]
    }

    #[inline]
    fn check(&self, doc: &Document) -> Result<Vec<Violation>> {
        let mut violations = vec![];

        for node in doc.ast.descendants() {
            if let NodeValue::Code(code) = &node.data.borrow().value {
                let position = node.data.borrow().sourcepos;
                let content_len = position.end.column - position.start.column + 1;
                if code.literal.trim() != code.literal || code.literal.len() != content_len {
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
        let text = "` some text `

`some text `

` some text`"
            .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let ast = parse_document(&arena, &text, &Options::default());
        let doc = Document {
            path: path.clone(),
            ast,
            text,
        };
        let rule = MD038::new();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![
            rule.to_violation(path.clone(), Sourcepos::from((1, 2, 1, 12))),
            rule.to_violation(path.clone(), Sourcepos::from((3, 2, 3, 11))),
            rule.to_violation(path, Sourcepos::from((5, 2, 5, 11))),
        ];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_no_errors() {
        let text = "`some text`".to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let ast = parse_document(&arena, &text, &Options::default());
        let doc = Document { path, ast, text };
        let rule = MD038::new();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![];
        assert_eq!(actual, expected);
    }
}
