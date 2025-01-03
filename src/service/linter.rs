use miette::Result;

use crate::config::Config;
use crate::rule;
use crate::rule::RuleLike;
use crate::violation::Violation;
use crate::Document;
use crate::Rule;

#[derive(Default)]
pub struct Linter {
    rules: Vec<Box<dyn RuleLike>>,
}

impl Linter {
    #[inline]
    #[must_use]
    pub fn new(config: &Config) -> Self {
        let rules: Vec<_> = config
            .lint
            .rules
            .iter()
            .map(|rule| {
                let boxed: Box<dyn RuleLike> = match rule {
                    Rule::MD001 => Box::new(rule::MD001::new()),
                    Rule::MD002 => Box::new(rule::MD002::new(config.lint.md002.level)),
                    Rule::MD003 => Box::new(rule::MD003::new(config.lint.md003.style.clone())),
                    Rule::MD004 => Box::new(rule::MD004::new(config.lint.md004.style.clone())),
                    Rule::MD005 => Box::new(rule::MD005::new()),
                    Rule::MD006 => Box::new(rule::MD006::new()),
                    Rule::MD007 => Box::new(rule::MD007::new(config.lint.md007.indent)),
                    Rule::MD009 => Box::new(rule::MD009::new()),
                    Rule::MD010 => Box::new(rule::MD010::new()),
                    Rule::MD012 => Box::new(rule::MD012::new()),
                    Rule::MD013 => Box::new(rule::MD013::new(config.lint.md013.line_length)),
                    Rule::MD014 => Box::new(rule::MD014::new()),
                    Rule::MD018 => Box::new(rule::MD018::new()),
                    Rule::MD019 => Box::new(rule::MD019::new()),
                    Rule::MD020 => Box::new(rule::MD020::new()),
                    Rule::MD021 => Box::new(rule::MD021::new()),
                    Rule::MD022 => Box::new(rule::MD022::new()),
                    Rule::MD023 => Box::new(rule::MD023::new()),
                    Rule::MD024 => Box::new(rule::MD024::new()),
                    Rule::MD025 => Box::new(rule::MD025::new(config.lint.md025.level)),
                    Rule::MD026 => {
                        Box::new(rule::MD026::new(config.lint.md026.punctuation.clone()))
                    }
                    Rule::MD027 => Box::new(rule::MD027::new()),
                    Rule::MD028 => Box::new(rule::MD028::new()),
                    Rule::MD029 => Box::new(rule::MD029::new(config.lint.md029.style.clone())),
                    Rule::MD030 => Box::new(rule::MD030::new(
                        config.lint.md030.ul_single,
                        config.lint.md030.ol_single,
                        config.lint.md030.ul_multi,
                        config.lint.md030.ol_multi,
                    )),
                    Rule::MD031 => Box::new(rule::MD031::new()),
                    Rule::MD032 => Box::new(rule::MD032::new()),
                    Rule::MD033 => Box::new(rule::MD033::new(&config.lint.md033.allowed_elements)),
                    Rule::MD034 => Box::new(rule::MD034::new()),
                    Rule::MD035 => Box::new(rule::MD035::new(config.lint.md035.style.clone())),
                    Rule::MD036 => {
                        Box::new(rule::MD036::new(config.lint.md036.punctuation.clone()))
                    }
                    Rule::MD037 => Box::new(rule::MD037::new()),
                    Rule::MD038 => Box::new(rule::MD038::new()),
                    Rule::MD039 => Box::new(rule::MD039::new()),
                    Rule::MD040 => Box::new(rule::MD040::new()),
                    Rule::MD041 => Box::new(rule::MD041::new(config.lint.md041.level)),
                    Rule::MD046 => Box::new(rule::MD046::new(config.lint.md046.style.clone())),
                    Rule::MD047 => Box::new(rule::MD047::new()),
                };
                boxed
            })
            .collect();

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
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use comrak::{nodes::Sourcepos, parse_document, Arena, Options};
    use pretty_assertions::assert_eq;

    use super::*;
    use rule::MD026;

    #[test]
    fn check_with_front_matter() {
        let text = "---
comments: false
description: Some text
---

# This is a header."
            .to_owned();
        let path = Path::new("test.md").to_path_buf();
        let arena = Arena::new();
        let mut options = Options::default();
        options.extension.front_matter_delimiter = Some("---".to_owned());
        let ast = parse_document(&arena, &text, &options);
        let doc = Document {
            path: path.clone(),
            ast,
            text,
        };
        let md026 = MD026::default();
        let rules = vec![Rule::MD026];
        let mut config = Config::default();
        config.lint.rules = rules;
        let linter = Linter::new(&config);
        let actual = linter.check(&doc).unwrap();
        let expected = vec![md026.to_violation(path, Sourcepos::from((6, 1, 6, 19)))];
        assert_eq!(actual, expected);
    }
}
