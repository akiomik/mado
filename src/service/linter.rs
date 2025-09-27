use miette::Result;

use crate::config::Config;
use crate::violation::Violation;
use crate::Document;
use crate::Rule;

#[derive(Default)]
pub struct Linter {
    rules: Vec<Rule>,
}

impl Linter {
    #[inline]
    #[must_use]
    pub const fn new(rules: Vec<Rule>) -> Self {
        Self { rules }
    }

    #[inline]
    pub fn check(&self, doc: &Document) -> Result<Vec<Violation>> {
        // Iterate rules while unrolling Vec<Result<Vec<..>>> to Result<Vec<..>>
        self.rules.iter().try_fold(vec![], |mut unrolled, rule| {
            let result = rule.check(doc);
            unrolled.extend(result?);
            Ok(unrolled)
        })
    }
    #[inline]
    #[must_use]
    pub fn from_config(config: &Config) -> Self {
        let rules = config.lint.to_rules();
        Self { rules }
    }
}

// Note: prefer `Linter::from_config(&Config)` over From<&Config> impl.

#[cfg(test)]
mod tests {
    use std::path::Path;

    use comrak::{nodes::Sourcepos, Arena};
    use indoc::indoc;
    use pretty_assertions::assert_eq;

    use crate::config::lint::RuleSet;
    use crate::rule::RuleLike as _;
    use crate::rule::MD026;

    use super::*;

    #[test]
    fn check() -> Result<()> {
        let text = indoc! {"
            ---
            comments: false
            description: Some text
            ---

            # This is a header.
        "}
        .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let doc = Document::new(&arena, path.clone(), text)?;
        let md026 = MD026::default();
        let rules = vec![Rule::MD026(md026.clone())];
        let linter = Linter::new(rules);
        let actual = linter.check(&doc)?;
        let expected = vec![md026.to_violation(path, Sourcepos::from((6, 1, 6, 19)))];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn from_config() {
        let md026 = MD026::default();
        let rules = vec![RuleSet::MD026];
        let mut config = Config::default();
        config.lint.rules = rules;
        let linter = Linter::from_config(&config);
        let expected = vec![Rule::MD026(md026)];
        assert_eq!(linter.rules, expected);
    }
}
