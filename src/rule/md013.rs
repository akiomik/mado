use std::sync::LazyLock;

use comrak::nodes::{NodeValue, Sourcepos};
use miette::Result;
use regex::Regex;

use crate::{collection::RangeSet, violation::Violation, Document};

use super::{
    line::{LineContext, LineMatcher, LineRule},
    NewRuleLike, RuleLike, RuleMetadata,
};

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct MD013 {
    line_length: usize,
    code_blocks: bool,
    tables: bool,
}

impl MD013 {
    pub const DEFAULT_LINE_LENGTH: usize = 80;
    pub const DEFAULT_CODE_BLOCKS: bool = true;
    pub const DEFAULT_TABLES: bool = true;

    #[inline]
    #[must_use]
    pub fn new(line_length: usize, code_blocks: bool, tables: bool) -> Self {
        Self {
            line_length,
            code_blocks,
            tables,
        }
    }
}

impl Default for MD013 {
    #[inline]
    fn default() -> Self {
        Self {
            line_length: Self::DEFAULT_LINE_LENGTH,
            code_blocks: Self::DEFAULT_CODE_BLOCKS,
            tables: Self::DEFAULT_TABLES,
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

        let mut code_block_ranges = RangeSet::new();
        let mut table_ranges = RangeSet::new();

        if !self.code_blocks || !self.tables {
            for node in doc.ast.descendants() {
                if !self.code_blocks {
                    if let NodeValue::CodeBlock(_code) = &node.data.borrow().value {
                        let position = node.data.borrow().sourcepos;
                        let range = position.start.line..=position.end.line;
                        code_block_ranges.insert(range);
                    }
                }

                if !self.tables {
                    if let NodeValue::Table(_table) = &node.data.borrow().value {
                        let position = node.data.borrow().sourcepos;
                        let range = position.start.line..=position.end.line;
                        table_ranges.insert(range);
                    }
                }
            }
        }

        for (i, line) in doc.lines.iter().enumerate() {
            let lineno = i + 1;

            if !self.code_blocks && code_block_ranges.contains(&lineno) {
                continue;
            }

            if !self.tables && table_ranges.contains(&lineno) {
                continue;
            }

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

    use comrak::Arena;
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
        let doc = Document::new(&arena, path.clone(), text).unwrap();
        let rule = MD013::new(34, true, true);
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
        let doc = Document::new(&arena, path.clone(), text).unwrap();
        let rule = MD013::new(34, true, true);
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
        let doc = Document::new(&arena, path.clone(), text).unwrap();
        let rule = MD013::new(34, true, true);
        let actual = rule.check(&doc).unwrap();
        let expected = vec![
            rule.to_violation(path.clone(), Sourcepos::from((3, 35, 3, 37))),
            rule.to_violation(path, Sourcepos::from((5, 35, 5, 37))),
        ];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_errors_code_block() {
        let text = "
IF THIS LINE IS THE MAXIMUM LENGTH

```ruby
puts 'This line is a violation because there are spaces beyond that length'
```"
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text).unwrap();
        let rule = MD013::new(34, true, true);
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
        let doc = Document::new(&arena, path, text).unwrap();
        let rule = MD013::new(34, true, true);
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
        let doc = Document::new(&arena, path, text).unwrap();
        let rule = MD013::new(34, true, true);
        let actual = rule.check(&doc).unwrap();
        let expected = vec![];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_no_errors_table() {
        let text = "
IF THIS LINE IS THE MAXIMUM LENGTH
| foo | bar | baz | foo | bar | baz |
|-----|-----|-----|-----|-----|-----|
| foo | bar | baz | foo | bar | baz |"
            .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text).unwrap();
        let rule = MD013::new(34, true, false);
        let actual = rule.check(&doc).unwrap();
        let expected = vec![];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_no_errors_table_without_spaces() {
        let text = "
IF THIS LINE IS THE MAXIMUM LENGTH
|foo|bar|baz|foo|bar|baz|foo|bar|baz|foo|bar|baz|foo|bar|baz|foo|bar|baz|foo|bar|baz|
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
|foo|bar|baz|foo|bar|baz|foo|bar|baz|foo|bar|baz|foo|bar|baz|foo|bar|baz|foo|bar|baz|"
            .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text).unwrap();
        let rule = MD013::new(34, true, true);
        let actual = rule.check(&doc).unwrap();
        let expected = vec![];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_no_errors_code_block() {
        let text = "
IF THIS LINE IS THE MAXIMUM LENGTH

```ruby
puts 'This line is a violation because there are spaces beyond that length'
```"
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text).unwrap();
        let rule = MD013::new(34, false, true);
        let actual = rule.check(&doc).unwrap();
        let expected = vec![];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_no_errors_code_block_without_spaces() {
        let text = "
IF THIS LINE IS THE MAXIMUM LENGTH

```ruby
puts 'This line is okay because there-are-no-spaces-beyond-that-length'
puts 'This-line-is-okay-because-there-are-no-spaces-anywhere-within'
```"
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text).unwrap();
        let rule = MD013::new(34, true, true);
        let actual = rule.check(&doc).unwrap();
        let expected = vec![];
        assert_eq!(actual, expected);
    }

    #[test]
    fn check_no_errors_with_tables_false_and_code_blocks_false() {
        let text = r"
IF THIS LINE IS THE MAXIMUM LENGTH
```ruby
puts 'This line is a violation because there are spaces beyond that length'
```"
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text).unwrap();
        let rule = MD013::new(34, false, false);
        let actual = rule.check(&doc).unwrap();
        let expected = vec![];
        assert_eq!(actual, expected);
    }
}
