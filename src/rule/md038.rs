use comrak::nodes::{NodeValue, Sourcepos};
use miette::Result;

use crate::{Document, violation::Violation};

use super::{Metadata, RuleLike, Tag};

#[derive(Debug, Default, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct MD038;

impl MD038 {
    const METADATA: Metadata = Metadata {
        name: "MD038",
        description: "Spaces inside code span elements",
        tags: &[Tag::Whitespace, Tag::Code],
        aliases: &["no-space-in-code"],
    };

    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        Self {}
    }

    /// The text between a code span's delimiters, exactly as it was written.
    ///
    /// `code.literal` cannot stand in for it: `CommonMark` drops one space from
    /// each end of a padded span and turns line endings into spaces, so by the
    /// time comrak hands the span over, the padding this rule looks for is gone.
    /// Reading it back out of the source keeps both facts visible.
    ///
    /// `sourcepos` columns are 1-based byte offsets that cover the delimiters as
    /// well, so the span is `num_backticks` bytes longer at each end than its
    /// content.
    ///
    /// `None` means the content could not be read back, and the caller then skips
    /// the span. That covers a span written across more than one line, which
    /// `markdownlint` skips too — its padding, if any, sits against a line
    /// ending rather than against a delimiter.
    fn content(lines: &[String], position: Sourcepos, num_backticks: usize) -> Option<&str> {
        if position.start.line != position.end.line {
            return None;
        }

        let line = lines.get(position.start.line.checked_sub(1)?)?;
        let span = line.get(position.start.column.checked_sub(1)?..position.end.column)?;
        span.get(num_backticks..span.len().checked_sub(num_backticks)?)
    }

    /// The content as a reader sees it, or `None` when `CommonMark` leaves it
    /// alone.
    ///
    /// `CommonMark` removes one space from each end of a code span that both
    /// begins and ends with one, unless it is nothing but spaces. Those two
    /// spaces are the only ones that can be there out of necessity rather than
    /// carelessness, so which spaces were removed is what this rule turns on.
    fn strip_padding(content: &str) -> Option<&str> {
        // "Entirely of spaces" is about the space character alone, so a tab
        // between two of them is content and the pair around it does come off.
        if content.bytes().all(|byte| byte == b' ') {
            return None;
        }

        content.strip_prefix(' ')?.strip_suffix(' ')
    }

    /// Whether a code span carries a space this rule should report.
    ///
    /// A pair of spaces `CommonMark` removes is not padding when it is the only
    /// way to write the span: a span that starts or ends with a backtick has to
    /// be padded at *both* ends, because the removal happens only when both are
    /// there. ``` `` ` `` ``` is how `CommonMark` spells a literal backtick, and
    /// reporting it would leave no way to author one at all.
    fn is_padded(content: &str) -> bool {
        let stripped = Self::strip_padding(content);
        let visible = stripped.unwrap_or(content);

        // A space still standing after the removal is padding whatever it sits
        // next to; a removed pair is padding unless it shields a backtick.
        visible.starts_with(char::is_whitespace)
            || visible.ends_with(char::is_whitespace)
            || stripped.is_some_and(|inner| !(inner.starts_with('`') || inner.ends_with('`')))
    }
}

impl RuleLike for MD038 {
    #[inline]
    fn metadata(&self) -> &'static Metadata {
        &Self::METADATA
    }

    #[inline]
    fn check(&self, doc: &Document) -> Result<Vec<Violation>> {
        let mut violations = vec![];

        for node in doc.ast.descendants() {
            if let NodeValue::Code(code) = &node.data.borrow().value {
                let position = node.data.borrow().sourcepos;
                let Some(content) = Self::content(&doc.lines, position, code.num_backticks) else {
                    continue;
                };

                if Self::is_padded(content) {
                    let violation = self.to_violation(doc.path.clone(), position);
                    violations.push(violation);
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
    fn check_errors() -> Result<()> {
        let text = indoc! {"
            ` some text `

            `some text `

            ` some text`
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD038::new();
        let actual = rule.check(&doc)?;
        let expected = vec![
            rule.to_violation(path.clone(), Sourcepos::from((1, 1, 1, 13))),
            rule.to_violation(path.clone(), Sourcepos::from((3, 1, 3, 12))),
            rule.to_violation(path, Sourcepos::from((5, 1, 5, 12))),
        ];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_errors_with_multiple_backticks() -> Result<()> {
        let text = indoc! {"
            ``  some text  ``

            ```some text ```
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD038::new();
        let actual = rule.check(&doc)?;
        let expected = vec![
            rule.to_violation(path.clone(), Sourcepos::from((1, 1, 1, 17))),
            rule.to_violation(path, Sourcepos::from((3, 1, 3, 16))),
        ];
        assert_eq!(actual, expected);
        Ok(())
    }

    // The removed pair shields the backtick, but the space it leaves behind is
    // padding all the same.
    #[test]
    fn check_errors_with_padded_backtick_content() -> Result<()> {
        let text = indoc! {"
            ``  ` ``

            `` `  ``
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD038::new();
        let actual = rule.check(&doc)?;
        let expected = vec![
            rule.to_violation(path.clone(), Sourcepos::from((1, 1, 1, 8))),
            rule.to_violation(path, Sourcepos::from((3, 1, 3, 8))),
        ];
        assert_eq!(actual, expected);
        Ok(())
    }

    // Nothing but spaces is left alone by `CommonMark`, so every one of them is
    // visible padding.
    #[test]
    fn check_errors_with_only_spaces() -> Result<()> {
        let text = "` `".to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD038::new();
        let actual = rule.check(&doc)?;
        let expected = vec![rule.to_violation(path, Sourcepos::from((1, 1, 1, 3)))];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_no_errors() -> Result<()> {
        let text = "`some text`".to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text)?;
        let rule = MD038::new();
        let actual = rule.check(&doc)?;
        let expected = vec![];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_no_errors_with_multiple_backticks() -> Result<()> {
        let text = indoc! {"
            ``some text``

            ``some `text` here``

            ```some text```
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text)?;
        let rule = MD038::new();
        let actual = rule.check(&doc)?;
        let expected = vec![];
        assert_eq!(actual, expected);
        Ok(())
    }

    // The spaces are the only way to write a span that starts or ends with a
    // backtick, so they are not padding and must not be reported.
    #[test]
    fn check_no_errors_with_backtick_content() -> Result<()> {
        let text = indoc! {"
            `` ` ``

            `` `some text ``

            `` some text` ``
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text)?;
        let rule = MD038::new();
        let actual = rule.check(&doc)?;
        let expected = vec![];
        assert_eq!(actual, expected);
        Ok(())
    }

    // A span broken across lines has no padding against a delimiter to find, and
    // its columns belong to two different lines. Both point the same way: skip it.
    #[test]
    fn check_no_errors_with_multiline_code_span() -> Result<()> {
        let text = indoc! {"
            `some
            text`
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text)?;
        let rule = MD038::new();
        let actual = rule.check(&doc)?;
        let expected = vec![];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_no_errors_with_multibyte_prefix() -> Result<()> {
        let text = "\u{3042}\u{3044} ``some text`` \u{3046}\u{3048}".to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text)?;
        let rule = MD038::new();
        let actual = rule.check(&doc)?;
        let expected = vec![];
        assert_eq!(actual, expected);
        Ok(())
    }
}
