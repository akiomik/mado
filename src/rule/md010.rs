use markdown::unist::Position;
use miette::IntoDiagnostic;
use miette::Result;
use regex::Regex;

use crate::{violation::Violation, Document};

use super::Rule;

#[derive(Default)]
pub struct MD010 {}

impl MD010 {
    pub fn new() -> Self {
        Self {}
    }
}

impl Rule for MD010 {
    #[inline]
    fn name(&self) -> String {
        "MD010".to_string()
    }

    #[inline]
    fn description(&self) -> String {
        "Hard tabs".to_string()
    }

    #[inline]
    fn tags(&self) -> Vec<String> {
        vec!["whitespace".to_string(), "hard_tab".to_string()]
    }

    #[inline]
    fn aliases(&self) -> Vec<String> {
        vec!["no-hard-tabs".to_string()]
    }

    fn check(&self, doc: &Document) -> Result<Vec<Violation>> {
        let re = Regex::new("\t").into_diagnostic()?;

        let mut violations = vec![];
        for (i, line) in doc.text.lines().enumerate() {
            let lineno = i + 1;
            let mut locs = re.capture_locations();
            re.captures_read(&mut locs, line);
            if let Some((start_column, end_column)) = locs.get(0) {
                // TODO: Use correct offset
                let position = Position::new(lineno, start_column, 0, lineno, end_column, 0);
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

    use markdown::{unist::Position, ParseOptions};

    use super::*;

    #[test]
    fn check_errors() {
        let text = "Some text

	* hard tab character used to indent the list item";
        let path = Path::new("test.md").to_path_buf();
        let ast = markdown::to_mdast(text, &ParseOptions::default()).unwrap();
        let doc = Document {
            path: path.clone(),
            ast,
            text: text.to_string(),
        };
        let rule = MD010::new();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![rule.to_violation(path, Position::new(3, 0, 0, 3, 1, 0))];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_no_errors() {
        let text = "Some text

    * Spaces used to indent the list item instead";
        let path = Path::new("test.md").to_path_buf();
        let ast = markdown::to_mdast(text, &ParseOptions::default()).unwrap();
        let doc = Document {
            path,
            ast,
            text: text.to_string(),
        };
        let rule = MD010::new();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![];
        assert_eq!(actual, expected);
    }
}