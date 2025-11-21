use core::result;

use comrak::nodes::{NodeValue, Sourcepos};
use miette::Result;
use serde::{Deserialize, Serialize};

use crate::{Document, violation::Violation};

use super::{Metadata, RuleLike, Tag};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum HorizontalRuleStyle {
    Consistent,
    Custom(String),
}

impl<'de> Deserialize<'de> for HorizontalRuleStyle {
    #[inline]
    fn deserialize<D>(deserializer: D) -> result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        if s == "consistent" {
            Ok(Self::Consistent)
        } else {
            Ok(Self::Custom(s))
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct MD035 {
    style: HorizontalRuleStyle,
}

impl MD035 {
    const METADATA: Metadata = Metadata {
        name: "MD035",
        description: "Horizontal rule style",
        tags: &[Tag::Hr],
        aliases: &["hr-style"],
    };

    pub const DEFAULT_STYLE: HorizontalRuleStyle = HorizontalRuleStyle::Consistent;

    #[inline]
    #[must_use]
    pub const fn new(style: HorizontalRuleStyle) -> Self {
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
    fn metadata(&self) -> &'static Metadata {
        &Self::METADATA
    }

    // TODO: Use safe casting
    #[inline]
    #[allow(clippy::cast_possible_wrap)]
    fn check(&self, doc: &Document) -> Result<Vec<Violation>> {
        let mut positions = vec![];
        let mut violations = vec![];
        let mut maybe_hr = None;

        for node in doc.ast.children() {
            if node.data.borrow().value == NodeValue::ThematicBreak {
                positions.push(node.data.borrow().sourcepos.start.line);
            }
        }

        for (i, line) in doc.lines.iter().enumerate() {
            let lineno = i + 1;
            if positions.contains(&lineno) {
                let is_violated = match (&self.style, maybe_hr) {
                    (HorizontalRuleStyle::Consistent, Some(hr))
                    | (HorizontalRuleStyle::Custom(hr), _) => line != hr,
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

    use comrak::{Arena, nodes::Sourcepos};
    use indoc::indoc;
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn check_errors_for_consistent() -> Result<()> {
        let text = indoc! {"
            ---

            - - -

            ***

            * * *

            ****
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD035::default();
        let actual = rule.check(&doc)?;
        let expected = vec![
            rule.to_violation(path.clone(), Sourcepos::from((3, 1, 3, 5))),
            rule.to_violation(path.clone(), Sourcepos::from((5, 1, 5, 3))),
            rule.to_violation(path.clone(), Sourcepos::from((7, 1, 7, 5))),
            rule.to_violation(path, Sourcepos::from((9, 1, 9, 4))),
        ];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_errors_for_custom() -> Result<()> {
        let text = indoc! {"
            ---

            - - -

            ***

            * * *

            ****
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD035::new(HorizontalRuleStyle::Custom("***".to_owned()));
        let actual = rule.check(&doc)?;
        let expected = vec![
            rule.to_violation(path.clone(), Sourcepos::from((1, 1, 1, 3))),
            rule.to_violation(path.clone(), Sourcepos::from((3, 1, 3, 5))),
            rule.to_violation(path.clone(), Sourcepos::from((7, 1, 7, 5))),
            rule.to_violation(path, Sourcepos::from((9, 1, 9, 4))),
        ];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_no_errors_for_consistent() -> Result<()> {
        let text = indoc! {"
            ---

            ---
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text)?;
        let rule = MD035::default();
        let actual = rule.check(&doc)?;
        let expected = vec![];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_no_errors_for_custom() -> Result<()> {
        let text = indoc! {"
            ***

            ***
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text)?;
        let rule = MD035::new(HorizontalRuleStyle::Custom("***".to_owned()));
        let actual = rule.check(&doc)?;
        let expected = vec![];
        assert_eq!(actual, expected);
        Ok(())
    }
}
