use std::sync::LazyLock;

use comrak::nodes::Sourcepos;
use miette::Result;
use regex::Regex;

use crate::violation::Violation;
use crate::Document;

use super::{
    line::{LineContext, LineMatcher, LineRule},
    NewRuleLike, RuleLike, RuleMetadata,
};

#[derive(Default, Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct MD010;

impl MD010 {
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self {}
    }
}

impl RuleLike for MD010 {
    #[inline]
    fn name(&self) -> &'static str {
        "MD010"
    }

    #[inline]
    fn description(&self) -> &'static str {
        "Hard tabs"
    }

    #[inline]
    fn tags(&self) -> Vec<&'static str> {
        vec!["whitespace", "hard_tab"]
    }

    #[inline]
    fn aliases(&self) -> Vec<&'static str> {
        vec!["no-hard-tabs"]
    }

    #[inline]
    fn check(&self, doc: &Document) -> Result<Vec<Violation>> {
        static RE: LazyLock<Regex> = LazyLock::new(|| {
            #[allow(clippy::unwrap_used)]
            Regex::new("\t").unwrap()
        });

        let mut violations = vec![];
        for (i, line) in doc.text.lines().enumerate() {
            let lineno = i + 1;
            if let Some(m) = RE.find(line) {
                let position = Sourcepos::from((lineno, m.start() + 1, lineno, m.end()));
                let violation = self.to_violation(doc.path.clone(), position);
                violations.push(violation);
            }
        }

        Ok(violations)
    }
}

impl NewRuleLike for MD010 {
    #[inline]
    fn metadata(&self) -> RuleMetadata {
        RuleMetadata {
            name: "MD010",
            description: "Hard tabs",
            tags: vec!["whitespace", "hard_tab"],
            aliases: vec!["no-hard-tabs"],
        }
    }

    #[inline]
    fn reset(&mut self) {}
}

impl LineRule for MD010 {
    #[inline]
    fn matcher(&self) -> LineMatcher {
        LineMatcher::new(|_line| true)
    }

    #[inline]
    fn run<'a>(&self, ctx: &LineContext, line: &str) -> Result<Vec<Violation>> {
        static RE: LazyLock<Regex> = LazyLock::new(|| {
            #[allow(clippy::unwrap_used)]
            Regex::new("\t").unwrap()
        });

        let mut violations = vec![];
        if let Some(m) = RE.find(line) {
            let position = Sourcepos::from((ctx.lineno, m.start() + 1, ctx.lineno, m.end()));
            let violation = self.to_violation(ctx.path.clone(), position);
            violations.push(violation);
        }

        Ok(violations)
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use comrak::{parse_document, Arena, Options};
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn check_errors() {
        let text = "Some text

	* hard tab character used to indent the list item"
            .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let ast = parse_document(&arena, &text, &Options::default());
        let doc = Document {
            path: path.clone(),
            ast,
            text,
        };
        let rule = MD010::new();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![rule.to_violation(path, Sourcepos::from((3, 1, 3, 1)))];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_no_errors() {
        let text = "Some text

    * Spaces used to indent the list item instead"
            .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let ast = parse_document(&arena, &text, &Options::default());
        let doc = Document { path, ast, text };
        let rule = MD010::new();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![];
        assert_eq!(actual, expected);
    }
}
