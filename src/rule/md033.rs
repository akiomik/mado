use std::path::Path;

use comrak::nodes::{NodeValue, Sourcepos};
use miette::Result;
use scraper::Html;

use crate::{violation::Violation, Document};

use super::RuleLike;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct MD033 {
    allowed_elements: Vec<String>,
}

impl MD033 {
    #[inline]
    #[must_use]
    pub fn new(allowed_elements: &[String]) -> Self {
        Self {
            allowed_elements: allowed_elements.to_vec(),
        }
    }

    fn check_html(
        &self,
        html: &str,
        path: &Path,
        position: &Sourcepos,
        violations: &mut Vec<Violation>,
    ) {
        // NOTE: `scraper::Html` automatically creates `<html>` as the root element
        let fragment = Html::parse_fragment(html);
        for element in fragment.root_element().child_elements() {
            let is_allowed = self
                .allowed_elements
                .contains(&element.value().name().to_owned());

            if !is_allowed {
                let violation = self.to_violation(path.to_path_buf(), *position);
                violations.push(violation);
            }
        }
    }
}

impl RuleLike for MD033 {
    #[inline]
    fn name(&self) -> String {
        "MD033".to_owned()
    }

    #[inline]
    fn description(&self) -> String {
        "Inline HTML".to_owned()
    }

    #[inline]
    fn tags(&self) -> Vec<String> {
        vec!["html".to_owned()]
    }

    #[inline]
    fn aliases(&self) -> Vec<String> {
        vec!["no-inline-html".to_owned()]
    }

    #[inline]
    fn check(&self, doc: &Document) -> Result<Vec<Violation>> {
        let mut violations = vec![];

        for node in doc.ast.descendants() {
            let position = node.data.borrow().sourcepos;

            match &node.data.borrow().value {
                NodeValue::HtmlInline(html) => {
                    self.check_html(html, &doc.path, &position, &mut violations);
                }
                NodeValue::HtmlBlock(html) => {
                    // NOTE: Skip non-html elements (e.g. comments)
                    //       See https://spec.commonmark.org/0.31.2/#html-blocks
                    if (2..=5).contains(&html.block_type) {
                        continue;
                    }

                    self.check_html(&html.literal, &doc.path, &position, &mut violations);
                }
                _ => {}
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
        let text = "<h1>Inline HTML header</h1>".to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let ast = parse_document(&arena, &text, &Options::default());
        let doc = Document {
            path: path.clone(),
            ast,
            text,
        };
        let rule = MD033::default();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![rule.to_violation(path, Sourcepos::from((1, 1, 1, 27)))];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_errors_with_allowed_tag() {
        let text = "<p>h1</p>".to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let ast = parse_document(&arena, &text, &Options::default());
        let doc = Document {
            path: path.clone(),
            ast,
            text,
        };
        let rule = MD033::new(&["h1".to_owned()]);
        let actual = rule.check(&doc).unwrap();
        let expected = vec![rule.to_violation(path, Sourcepos::from((1, 1, 1, 9)))];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_errors_with_allowed_tag2() {
        let text = "<pre>text</pre>".to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let ast = parse_document(&arena, &text, &Options::default());
        let doc = Document {
            path: path.clone(),
            ast,
            text,
        };
        let rule = MD033::new(&["p".to_owned()]);
        let actual = rule.check(&doc).unwrap();
        let expected = vec![rule.to_violation(path, Sourcepos::from((1, 1, 0, 0)))];
        assert_eq!(actual, expected);
    }

    // TODO: Cannot handle this case because the implicit `<html>` tag
    //       owned by `scraper::html` must be skipped.
    // #[test]
    // fn check_errors_with_html() {
    //     let text = "<html>text</html>".to_owned();
    //     let path = Path::new("test.md").to_path_buf();
    //     let arena = Arena::new();
    //     let ast = parse_document(&arena, &text, &Options::default());
    //     let doc = Document {
    //         path: path.clone(),
    //         ast,
    //         text,
    //     };
    //     let rule = MD033::default();
    //     let actual = rule.check(&doc).unwrap();
    //     let expected = vec![rule.to_violation(path, Sourcepos::from((1, 1, 0, 0)))];
    //     assert_eq!(actual, expected);
    // }

    #[test]
    fn check_errors_with_br() {
        let text = "Some text<br>
Some more text"
            .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let ast = parse_document(&arena, &text, &Options::default());
        let doc = Document {
            path: path.clone(),
            ast,
            text,
        };
        let rule = MD033::default();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![rule.to_violation(path, Sourcepos::from((1, 10, 1, 13)))];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_no_errors() {
        let text = "# Markdown header".to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let ast = parse_document(&arena, &text, &Options::default());
        let doc = Document { path, ast, text };
        let rule = MD033::default();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_no_errors_with_allowed_tag() {
        let text = "<h1>Inline HTML header</h1>".to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let ast = parse_document(&arena, &text, &Options::default());
        let doc = Document {
            path: path.clone(),
            ast,
            text,
        };
        let rule = MD033::new(&["h1".to_owned()]);
        let actual = rule.check(&doc).unwrap();
        let expected = vec![];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_no_errors_with_comment() {
        let text = "<!-- html comment -->".to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let ast = parse_document(&arena, &text, &Options::default());
        let doc = Document {
            path: path.clone(),
            ast,
            text,
        };
        let rule = MD033::default();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![];
        assert_eq!(actual, expected);
    }
}
