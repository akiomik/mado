use std::collections::HashSet;

use comrak::nodes::NodeValue;
use miette::Result;

use crate::{violation::Violation, Document};

use super::{helper::inline_text_of, Rule};

// TODO: Support allow_different_nesting
#[derive(Default, Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct MD024;

impl MD024 {
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self {}
    }
}

impl Rule for MD024 {
    #[inline]
    fn name(&self) -> String {
        "MD024".to_owned()
    }

    #[inline]
    fn description(&self) -> String {
        "Multiple headers with the same content".to_owned()
    }

    #[inline]
    fn tags(&self) -> Vec<String> {
        vec!["headers".to_owned()]
    }

    #[inline]
    fn aliases(&self) -> Vec<String> {
        vec!["no-duplicate-header".to_owned()]
    }

    #[inline]
    fn check(&self, doc: &Document) -> Result<Vec<Violation>> {
        let mut violations = vec![];
        let mut contents: HashSet<String> = HashSet::new();

        for node in doc.ast.children() {
            if let NodeValue::Heading(_) = node.data.borrow().value {
                let text = inline_text_of(node);
                if contents.contains(&text) {
                    let position = node.data.borrow().sourcepos;
                    let violation = self.to_violation(doc.path.clone(), position);
                    violations.push(violation);
                } else {
                    contents.insert(text.clone());
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

    use super::*;

    #[test]
    fn check_errors() {
        let text = "# Some text

## Some text";
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let ast = parse_document(&arena, text, &Options::default());
        let doc = Document {
            path: path.clone(),
            ast,
            text: text.to_string(),
        };
        let rule = MD024::new();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![rule.to_violation(path.clone(), Sourcepos::from((3, 1, 3, 12)))];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_no_errors() {
        let text = "# Some text

## Some more text";
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let ast = parse_document(&arena, text, &Options::default());
        let doc = Document {
            path,
            ast,
            text: text.to_string(),
        };
        let rule = MD024::new();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![];
        assert_eq!(actual, expected);
    }
}