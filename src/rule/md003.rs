use comrak::nodes::NodeValue;
use miette::Result;
use serde::{Deserialize, Serialize};

use crate::Document;
use crate::violation::Violation;

use super::{Metadata, RuleLike, Tag};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum HeadingStyle {
    Consistent,
    Atx,
    AtxClosed,
    Setext,
    SetextWithAtx,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct MD003 {
    style: HeadingStyle,
}

impl MD003 {
    const METADATA: Metadata = Metadata {
        name: "MD003",
        description: "Header style",
        tags: &[Tag::Headers],
        aliases: &["header-style"],
    };

    pub const DEFAULT_HEADING_STYLE: HeadingStyle = HeadingStyle::Consistent;

    #[inline]
    #[must_use]
    pub const fn new(style: HeadingStyle) -> Self {
        Self { style }
    }
}

impl Default for MD003 {
    #[inline]
    fn default() -> Self {
        Self {
            style: Self::DEFAULT_HEADING_STYLE,
        }
    }
}

impl RuleLike for MD003 {
    #[inline]
    fn metadata(&self) -> &'static Metadata {
        &Self::METADATA
    }

    #[inline]
    fn check(&self, doc: &Document) -> Result<Vec<Violation>> {
        let mut violations = vec![];
        let mut maybe_heading_style = None;

        for node in doc.ast.children() {
            if let NodeValue::Heading(heading) = node.data.borrow().value {
                // TODO: Handle unmatched case
                let is_atx_closed = node.last_child().map_or(!heading.setext, |child_node| {
                    let heading_position = node.data.borrow().sourcepos;
                    let inner_position = child_node.data.borrow().sourcepos;
                    !heading.setext && heading_position.end.column > inner_position.end.column
                });

                let is_violated = match (&self.style, maybe_heading_style) {
                    (HeadingStyle::Consistent, Some((expected_setext, expected_atx_closed))) => {
                        heading.setext != expected_setext || expected_atx_closed != is_atx_closed
                    }
                    (HeadingStyle::Atx, _) => heading.setext || is_atx_closed,
                    (HeadingStyle::AtxClosed, _) => heading.setext || !is_atx_closed,
                    (HeadingStyle::Setext, _) => !heading.setext,
                    (HeadingStyle::SetextWithAtx, _) => heading.level < 3 && !heading.setext,
                    _ => false,
                };

                if is_violated {
                    let position = node.data.borrow().sourcepos;
                    let violation = self.to_violation(doc.path.clone(), position);
                    violations.push(violation);
                }

                if maybe_heading_style.is_none() {
                    maybe_heading_style = Some((heading.setext, is_atx_closed));
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
    fn check_errors_for_consistent() -> Result<()> {
        let text = indoc! {"
            # ATX style H1

            ## Closed ATX style H2 ##

            Setext style H1
            ===============
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD003::default();
        let actual = rule.check(&doc)?;
        let expected = vec![
            rule.to_violation(path.clone(), Sourcepos::from((3, 1, 3, 25))),
            rule.to_violation(path, Sourcepos::from((5, 1, 6, 15))),
        ];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_errors_for_atx() -> Result<()> {
        let text = indoc! {"
            # ATX style H1

            ## Closed ATX style H2 ##

            Setext style H1
            ===============
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD003::new(HeadingStyle::Atx);
        let actual = rule.check(&doc)?;
        let expected = vec![
            rule.to_violation(path.clone(), Sourcepos::from((3, 1, 3, 25))),
            rule.to_violation(path, Sourcepos::from((5, 1, 6, 15))),
        ];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_errors_for_atx_closed() -> Result<()> {
        let text = indoc! {"
            # ATX style H1

            ## Closed ATX style H2 ##

            Setext style H1
            ===============
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD003::new(HeadingStyle::AtxClosed);
        let actual = rule.check(&doc)?;
        let expected = vec![
            rule.to_violation(path.clone(), Sourcepos::from((1, 1, 1, 14))),
            rule.to_violation(path, Sourcepos::from((5, 1, 6, 15))),
        ];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_errors_for_setext() -> Result<()> {
        let text = indoc! {"
            # ATX style H1

            ## Closed ATX style H2 ##

            Setext style H1
            ===============
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD003::new(HeadingStyle::Setext);
        let actual = rule.check(&doc)?;
        let expected = vec![
            rule.to_violation(path.clone(), Sourcepos::from((1, 1, 1, 14))),
            rule.to_violation(path, Sourcepos::from((3, 1, 3, 25))),
        ];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_errors_for_setext_with_atx() -> Result<()> {
        let text = indoc! {"
            # ATX style H1

            ## ATX style H2

            ### ATX style H3
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD003::new(HeadingStyle::SetextWithAtx);
        let actual = rule.check(&doc)?;
        let expected = vec![
            rule.to_violation(path.clone(), Sourcepos::from((1, 1, 1, 14))),
            rule.to_violation(path, Sourcepos::from((3, 1, 3, 15))),
        ];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_no_errors_for_consistent() -> Result<()> {
        let text = indoc! {"
            # ATX style H1

            ## ATX style H2
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text)?;
        let rule = MD003::default();
        let actual = rule.check(&doc)?;
        let expected = vec![];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_no_errors_for_atx() -> Result<()> {
        let text = indoc! {"
            # ATX style H1

            ## ATX style H2
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text)?;
        let rule = MD003::new(HeadingStyle::Atx);
        let actual = rule.check(&doc)?;
        let expected = vec![];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_no_errors_for_atx_closed() -> Result<()> {
        let text = indoc! {"
            # ATX style H1 #

            ## ATX style H2 ##
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text)?;
        let rule = MD003::new(HeadingStyle::AtxClosed);
        let actual = rule.check(&doc)?;
        let expected = vec![];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_no_errors_for_setext() -> Result<()> {
        let text = indoc! {"
            Setext style H1
            ===============

            Setext style H2
            ---------------
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text)?;
        let rule = MD003::new(HeadingStyle::Setext);
        let actual = rule.check(&doc)?;
        let expected = vec![];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_no_errors_for_setext_with_atx() -> Result<()> {
        let text = indoc! {"
            Setext style H1
            ===============

            Setext style H2
            ---------------

            ### ATX style H3
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text)?;
        let rule = MD003::new(HeadingStyle::SetextWithAtx);
        let actual = rule.check(&doc)?;
        let expected = vec![];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_no_errors_with_front_matter() -> Result<()> {
        let text = indoc! {r#"
            ---
            author: "John Smith"
            ---

            # Header 1
        "#}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text)?;
        let rule = MD003::new(HeadingStyle::Consistent);
        let actual = rule.check(&doc)?;
        let expected = vec![];
        assert_eq!(actual, expected);
        Ok(())
    }
}
