use std::sync::LazyLock;

use comrak::nodes::Sourcepos;
use miette::Result;
use regex::Regex;

use crate::{violation::Violation, Document};

use super::{
    line::{LineContext, LineMatcher, LineRule},
    NewRuleLike, RuleLike, RuleMetadata,
};

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct MD013 {
    line_length: usize,
}

impl MD013 {
    pub const DEFAULT_LINE_LENGTH: usize = 80;

    #[inline]
    #[must_use]
    pub fn new(line_length: usize) -> Self {
        Self { line_length }
    }
}

impl Default for MD013 {
    #[inline]
    fn default() -> Self {
        Self {
            line_length: Self::DEFAULT_LINE_LENGTH,
        }
    }
}

impl RuleLike for MD013 {
    #[inline]
    fn name(&self) -> &'static str {
        "MD013"
    }

    #[inline]
    fn description(&self) -> &'static str {
        "Line length"
    }

    #[inline]
    fn tags(&self) -> Vec<&'static str> {
        vec!["line_length"]
    }

    #[inline]
    fn aliases(&self) -> Vec<&'static str> {
        vec!["line-length"]
    }

    #[inline]
    fn check(&self, doc: &Document) -> Result<Vec<Violation>> {
        static RE: LazyLock<Regex> = LazyLock::new(|| {
            #[allow(clippy::unwrap_used)]
            Regex::new(r".*\s").unwrap()
        });

        let mut violations = vec![];

        for (i, line) in doc.text.lines().enumerate() {
            let lineno = i + 1;

            if line.len() > self.line_length && RE.is_match_at(line, self.line_length) {
                let position = Sourcepos::from((lineno, self.line_length + 1, lineno, line.len()));
                let violation = self.to_violation(doc.path.clone(), position);
                violations.push(violation);
            }
        }

        Ok(violations)
    }
}

impl NewRuleLike for MD013 {
    fn metadata(&self) -> RuleMetadata {
        RuleMetadata {
            name: "MD013",
            description: "Line length",
            tags: vec!["line_length"],
            aliases: vec!["line-length"],
        }
    }
}

impl LineRule for MD013 {
    fn matcher(&self) -> LineMatcher {
        LineMatcher::new(|_line| true)
    }

    fn run<'a>(&self, ctx: &LineContext, line: &str) -> Result<Vec<Violation>> {
        static RE: LazyLock<Regex> = LazyLock::new(|| {
            #[allow(clippy::unwrap_used)]
            Regex::new(r".*\s").unwrap()
        });

        let mut violations = vec![];

        if line.len() > self.line_length && RE.is_match_at(line, self.line_length) {
            let position =
                Sourcepos::from((ctx.lineno, self.line_length + 1, ctx.lineno, line.len()));
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
        let text = "
IF THIS LINE IS THE MAXIMUM LENGTH
This line is a violation because there are spaces beyond that length"
            .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let ast = parse_document(&arena, &text, &Options::default());
        let doc = Document {
            path: path.clone(),
            ast,
            text,
        };
        let rule = MD013::new(34);
        let actual = rule.check(&doc).unwrap();
        let expected = vec![rule.to_violation(path, Sourcepos::from((3, 35, 3, 68)))];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_errors_with_other_nodes() {
        let text = "
IF THIS LINE IS THE MAXIMUM LENGTH
This line is a violation because [there are spaces beyond that](https://example.com)
This line is a violation because `there are spaces beyond that`"
            .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let ast = parse_document(&arena, &text, &Options::default());
        let doc = Document {
            path: path.clone(),
            ast,
            text,
        };
        let rule = MD013::new(34);
        let actual = rule.check(&doc).unwrap();
        let expected = vec![
            rule.to_violation(path.clone(), Sourcepos::from((3, 35, 3, 84))),
            rule.to_violation(path, Sourcepos::from((4, 35, 4, 63))),
        ];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_errors_table() {
        let text = "
IF THIS LINE IS THE MAXIMUM LENGTH
| foo | bar | baz | foo | bar | baz |
|-----|-----|-----|-----|-----|-----|
| foo | bar | baz | foo | bar | baz |"
            .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let ast = parse_document(&arena, &text, &Options::default());
        let doc = Document {
            path: path.clone(),
            ast,
            text,
        };
        let rule = MD013::new(34);
        let actual = rule.check(&doc).unwrap();
        let expected = vec![
            rule.to_violation(path.clone(), Sourcepos::from((3, 35, 3, 37))),
            rule.to_violation(path, Sourcepos::from((5, 35, 5, 37))),
        ];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_errors_codeblock() {
        let text = "
IF THIS LINE IS THE MAXIMUM LENGTH

```ruby
puts 'This line is a violation because there are spaces beyond that length'
```"
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let ast = parse_document(&arena, &text, &Options::default());
        let doc = Document {
            path: path.clone(),
            ast,
            text,
        };
        let rule = MD013::new(34);
        let actual = rule.check(&doc).unwrap();
        let expected = vec![rule.to_violation(path, Sourcepos::from((5, 35, 5, 75)))];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_no_errors() {
        let text = "
IF THIS LINE IS THE MAXIMUM LENGTH
This line is okay because there are-no-spaces-beyond-that-length
This-line-is-okay-because-there-are-no-spaces-anywhere-within"
            .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let ast = parse_document(&arena, &text, &Options::default());
        let doc = Document { path, ast, text };
        let rule = MD013::new(34);
        let actual = rule.check(&doc).unwrap();
        let expected = vec![];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_no_errors_with_other_nodes() {
        let text = "
IF THIS LINE IS THE MAXIMUM LENGTH
This line is okay because there [are-no-spaces-beyond-that-length](https://example.com)
This line is okay because there `are-no-spaces-beyond-that-length`"
            .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let ast = parse_document(&arena, &text, &Options::default());
        let doc = Document { path, ast, text };
        let rule = MD013::new(34);
        let actual = rule.check(&doc).unwrap();
        let expected = vec![];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_no_errors_table() {
        let text = "
IF THIS LINE IS THE MAXIMUM LENGTH
|foo|bar|baz|foo|bar|baz|foo|bar|baz|foo|bar|baz|foo|bar|baz|foo|bar|baz|foo|bar|baz|
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
|foo|bar|baz|foo|bar|baz|foo|bar|baz|foo|bar|baz|foo|bar|baz|foo|bar|baz|foo|bar|baz|"
            .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let ast = parse_document(&arena, &text, &Options::default());
        let doc = Document { path, ast, text };
        let rule = MD013::new(34);
        let actual = rule.check(&doc).unwrap();
        let expected = vec![];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_no_errors_codeblock() {
        let text = "
IF THIS LINE IS THE MAXIMUM LENGTH

```ruby
puts 'This line is okay because there-are-no-spaces-beyond-that-length'
puts 'This-line-is-okay-because-there-are-no-spaces-anywhere-within'
```"
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let ast = parse_document(&arena, &text, &Options::default());
        let doc = Document { path, ast, text };
        let rule = MD013::new(34);
        let actual = rule.check(&doc).unwrap();
        let expected = vec![];
        assert_eq!(actual, expected);
    }
}
