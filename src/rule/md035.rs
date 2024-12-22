use comrak::nodes::{NodeValue, Sourcepos};
use miette::Result;
use serde::Deserialize;

use crate::{violation::Violation, Document};

use super::RuleLike;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum HorizontalRuleStyle {
    Consistent,
    Custom(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct MD035 {
    style: HorizontalRuleStyle,
}

impl MD035 {
    pub const DEFAULT_STYLE: HorizontalRuleStyle = HorizontalRuleStyle::Consistent;

    #[inline]
    #[must_use]
    pub fn new(style: HorizontalRuleStyle) -> Self {
        Self { style }
    }
}

impl Default for MD035 {
    #[inline]
    fn default() -> Self {
        Self {
            style: HorizontalRuleStyle::Consistent,
        }
    }
}

impl RuleLike for MD035 {
    #[inline]
    fn name(&self) -> &'static str {
        "MD035"
    }

    #[inline]
    fn description(&self) -> &'static str {
        "Horizontal rule style"
    }

    #[inline]
    fn tags(&self) -> Vec<&'static str> {
        vec!["hr"]
    }

    #[inline]
    fn aliases(&self) -> Vec<&'static str> {
        vec!["hr-style"]
    }

    // TODO: Use safe casting
    #[inline]
    #[allow(clippy::cast_possible_wrap)]
    fn check(&self, doc: &Document) -> Result<Vec<Violation>> {
        let mut positions = vec![];
        let mut violations = vec![];
        let mut maybe_hr = None;

        for node in doc.ast.children() {
            if let NodeValue::ThematicBreak = &node.data.borrow().value {
                positions.push(node.data.borrow().sourcepos.start.line);
            }
        }

        for (i, line) in doc.text.lines().enumerate() {
            let lineno = i + 1;
            if positions.contains(&lineno) {
                let is_violated = match (&self.style, maybe_hr) {
                    (HorizontalRuleStyle::Consistent, Some(hr)) => line != hr,
                    (HorizontalRuleStyle::Custom(hr), _) => line != hr,
                    _ => false,
                };

                if is_violated {
                    let position = Sourcepos::from((lineno, 1, lineno, line.len()));
                    let violation = self.to_violation(doc.path.clone(), position);
                    violations.push(violation);
                }

                if maybe_hr.is_none() {
                    maybe_hr = Some(line);
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
    fn check_errors_for_consistent() {
        let text = "---

- - -

***

* * *

****"
            .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let ast = parse_document(&arena, &text, &Options::default());
        let doc = Document {
            path: path.clone(),
            ast,
            text,
        };
        let rule = MD035::default();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![
            rule.to_violation(path.clone(), Sourcepos::from((3, 1, 3, 5))),
            rule.to_violation(path.clone(), Sourcepos::from((5, 1, 5, 3))),
            rule.to_violation(path.clone(), Sourcepos::from((7, 1, 7, 5))),
            rule.to_violation(path, Sourcepos::from((9, 1, 9, 4))),
        ];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_errors_for_custom() {
        let text = "---

- - -

***

* * *

****"
            .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let ast = parse_document(&arena, &text, &Options::default());
        let doc = Document {
            path: path.clone(),
            ast,
            text,
        };
        let rule = MD035::new(HorizontalRuleStyle::Custom("***".to_owned()));
        let actual = rule.check(&doc).unwrap();
        let expected = vec![
            rule.to_violation(path.clone(), Sourcepos::from((1, 1, 1, 3))),
            rule.to_violation(path.clone(), Sourcepos::from((3, 1, 3, 5))),
            rule.to_violation(path.clone(), Sourcepos::from((7, 1, 7, 5))),
            rule.to_violation(path, Sourcepos::from((9, 1, 9, 4))),
        ];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_no_errors_for_consistent() {
        let text = "---

---"
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let ast = parse_document(&arena, &text, &Options::default());
        let doc = Document { path, ast, text };
        let rule = MD035::default();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_no_errors_for_custom() {
        let text = "***

***"
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let ast = parse_document(&arena, &text, &Options::default());
        let doc = Document { path, ast, text };
        let rule = MD035::new(HorizontalRuleStyle::Custom("***".to_owned()));
        let actual = rule.check(&doc).unwrap();
        let expected = vec![];
        assert_eq!(actual, expected);
    }
}