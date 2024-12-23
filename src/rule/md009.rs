use comrak::nodes::Sourcepos;
use miette::Result;

use crate::{violation::Violation, Document};

use super::{
    line::{LineContext, LineMatcher, LineRule},
    NewRuleLike, RuleLike, RuleMetadata,
};

#[derive(Default)]
#[non_exhaustive]
pub struct MD009;

impl MD009 {
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self {}
    }
}

impl RuleLike for MD009 {
    #[inline]
    fn name(&self) -> &'static str {
        "MD009"
    }

    #[inline]
    fn description(&self) -> &'static str {
        "Trailing spaces"
    }

    #[inline]
    fn tags(&self) -> Vec<&'static str> {
        vec!["whitespace"]
    }

    #[inline]
    fn aliases(&self) -> Vec<&'static str> {
        vec!["no-trailing-spaces"]
    }

    #[inline]
    fn check(&self, doc: &Document) -> Result<Vec<Violation>> {
        let mut violations = vec![];
        for (i, line) in doc.text.lines().enumerate() {
            let trimmed_line = line.trim_end_matches(' ');
            if trimmed_line != line {
                let lineno = i + 1;
                let position =
                    Sourcepos::from((lineno, trimmed_line.len() + 1, lineno, line.len()));
                let violation = self.to_violation(doc.path.clone(), position);
                violations.push(violation);
            }
        }

        Ok(violations)
    }
}

impl NewRuleLike for MD009 {
    fn metadata(&self) -> RuleMetadata {
        RuleMetadata {
            name: "MD009",
            description: "Trailing spaces",
            tags: vec!["whitespace"],
            aliases: vec!["no-trailing-spaces"],
        }
    }
}

impl LineRule for MD009 {
    fn matcher(&self) -> LineMatcher {
        LineMatcher::new(|line| {
            let trimmed_line = line.trim_end_matches(' ');
            trimmed_line != line
        })
    }

    fn run<'a>(&self, ctx: &LineContext, line: &str) -> Result<Vec<Violation>> {
        let trimmed_line = line.trim_end_matches(' ');
        let position =
            Sourcepos::from((ctx.lineno, trimmed_line.len() + 1, ctx.lineno, line.len()));
        let violation = self.to_violation(ctx.path.clone(), position);
        Ok(vec![violation])
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
        let text = "Text with a trailing space 
And text with some trailing spaces   "
            .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let ast = parse_document(&arena, &text, &Options::default());
        let doc = Document {
            path: path.clone(),
            ast,
            text,
        };
        let rule = MD009::new();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![
            rule.to_violation(path.clone(), Sourcepos::from((1, 27, 1, 27))),
            rule.to_violation(path, Sourcepos::from((2, 35, 2, 37))),
        ];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_no_errors() {
        let text = "Text with no trailing spaces".to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let ast = parse_document(&arena, &text, &Options::default());
        let doc = Document { path, ast, text };
        let rule = MD009::new();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_no_errors_full_with_space() {
        let text = "Text with no trailing spaces　".to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let ast = parse_document(&arena, &text, &Options::default());
        let doc = Document { path, ast, text };
        let rule = MD009::new();
        let actual = rule.check(&doc).unwrap();
        let expected = vec![];
        assert_eq!(actual, expected);
    }
}
