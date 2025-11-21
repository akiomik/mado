use comrak::nodes::NodeValue;
use miette::Result;
use rustc_hash::FxHashMap;

use crate::{Document, violation::Violation};

use super::{Metadata, RuleLike, Tag, helper::inline_text_of};

#[derive(Default, Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct MD024 {
    allow_different_nesting: bool,
}

impl MD024 {
    const METADATA: Metadata = Metadata {
        name: "MD024",
        description: "Multiple headers with the same content",
        tags: &[Tag::Headers],
        aliases: &["no-duplicate-header"],
    };

    #[inline]
    #[must_use]
    pub const fn new(allow_different_nesting: bool) -> Self {
        Self {
            allow_different_nesting,
        }
    }
}

impl RuleLike for MD024 {
    #[inline]
    fn metadata(&self) -> &'static Metadata {
        &Self::METADATA
    }

    #[inline]
    fn check(&self, doc: &Document) -> Result<Vec<Violation>> {
        let mut violations = vec![];
        let mut contents: FxHashMap<String, Vec<u8>> = FxHashMap::default();

        for node in doc.ast.children() {
            if let NodeValue::Heading(heading) = &node.data.borrow().value {
                let text = inline_text_of(node);
                if let Some(levels) = contents.get_mut(&text) {
                    let is_different_nesting = levels.len() == 1 && levels.contains(&heading.level);
                    if !self.allow_different_nesting || !is_different_nesting {
                        let position = node.data.borrow().sourcepos;
                        let violation = self.to_violation(doc.path.clone(), position);
                        violations.push(violation);
                    }

                    if !levels.contains(&heading.level) {
                        levels.push(heading.level);
                    }
                } else {
                    contents.insert(text.clone(), vec![heading.level]);
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
    fn check_errors_false() -> Result<()> {
        let text = indoc! {"
            # A

            ## A

            ## B

            ### C

            ## D

            ### C

            ## E

            #### C
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD024::default();
        let actual = rule.check(&doc)?;
        let expected = vec![
            rule.to_violation(path.clone(), Sourcepos::from((3, 1, 3, 4))),
            rule.to_violation(path.clone(), Sourcepos::from((11, 1, 11, 5))),
            rule.to_violation(path, Sourcepos::from((15, 1, 15, 6))),
        ];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_errors_true() -> Result<()> {
        let text = indoc! {"
            # A

            ## A

            ## B

            ### C

            ## D

            ### C

            ## E

            #### C
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let rule = MD024::new(true);
        let actual = rule.check(&doc)?;
        let expected = vec![
            rule.to_violation(path.clone(), Sourcepos::from((3, 1, 3, 4))),
            rule.to_violation(path, Sourcepos::from((15, 1, 15, 6))),
        ];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_no_errors_false() -> Result<()> {
        let text = indoc! {"
            # A

            ## B

            ## C

            ### D

            ## E

            ### F

            ## G

            #### H
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text)?;
        let rule = MD024::default();
        let actual = rule.check(&doc)?;
        let expected = vec![];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn check_no_errors_true() -> Result<()> {
        let text = indoc! {"
            # A

            ## B

            ## C

            ### D

            ## E

            ### D

            ## F

            #### G
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path, text)?;
        let rule = MD024::new(true);
        let actual = rule.check(&doc)?;
        let expected = vec![];
        assert_eq!(actual, expected);
        Ok(())
    }
}
