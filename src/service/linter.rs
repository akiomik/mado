use comrak::nodes::AstNode;
use comrak::nodes::NodeValue;
use miette::Result;

use crate::config::Config;
use crate::rule;
use crate::rule::line::LineContext;
use crate::rule::node::NodeContext;
use crate::rule::RuleLike;
use crate::rule::RuleType;
use crate::violation::Violation;
use crate::Document;
use crate::Rule;

#[derive(Default)]
pub struct Linter {
    rules: Vec<Box<dyn RuleLike>>,
    new_rules: Vec<RuleType>,
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

        let new_rules = vec![
            RuleType::Node(Box::new(rule::MD001::new())),
            RuleType::Node(Box::new(rule::MD002::new(config.lint.md002.level))),
            RuleType::Node(Box::new(rule::MD003::new(config.lint.md003.style.clone()))),
            RuleType::Node(Box::new(rule::MD004::new(config.lint.md004.style.clone()))),
            RuleType::Node(Box::new(rule::MD005::new())),
            RuleType::Node(Box::new(rule::MD006::new())),
            RuleType::Node(Box::new(rule::MD007::new(config.lint.md007.indent))),
            RuleType::Line(Box::new(rule::MD009::new())),
            RuleType::Line(Box::new(rule::MD010::new())),
            RuleType::Line(Box::new(rule::MD013::new(config.lint.md013.line_length))),
            RuleType::Node(Box::new(rule::MD014::new())),
            RuleType::Node(Box::new(rule::MD018::new())),
            RuleType::Node(Box::new(rule::MD019::new())),
            RuleType::Node(Box::new(rule::MD022::new())),
            RuleType::Node(Box::new(rule::MD023::new())),
            RuleType::Node(Box::new(rule::MD024::new())),
            RuleType::Node(Box::new(rule::MD025::new(config.lint.md025.level))),
        ];

        Self { rules, new_rules }
    }

    #[inline]
    fn check_node_recursive<'a>(
        &mut self,
        ctx: &NodeContext,
        root: &'a AstNode<'a>,
    ) -> Result<Vec<Violation>> {
        let mut violations = vec![];
        for node in root.children() {
            let mut node_ctx = ctx.clone();
            node_ctx.level += 1;

            if let NodeValue::List(_) = &node.data.borrow().value {
                match node_ctx.list_level {
                    Some(list_level) => {
                        node_ctx.list_level = Some(list_level + 1);
                    }
                    None => {
                        node_ctx.list_level = Some(1);
                    }
                }
            }

            for rule in self.new_rules.iter_mut() {
                if let RuleType::Node(node_rule) = rule {
                    if node_rule.matcher().is_match(node) {
                        let rule_violations = node_rule.run(&node_ctx, node)?;
                        violations.extend(rule_violations);
                    }
                }
            }

            let child_violations = self.check_node_recursive(&node_ctx, node)?;
            violations.extend(child_violations);
        }
        Ok(violations)
    }

    #[inline]
    pub fn new_flat_check(&mut self, doc: &Document) -> Result<Vec<Violation>> {
        let mut node_ctx = NodeContext {
            path: doc.path.clone(),
            level: 0,
            list_level: None,
        };

        let mut violations = vec![];
        let mut maybe_prev_list_node: Option<&'_ AstNode<'_>> = None;
        for node in doc.ast.descendants() {
            if let NodeValue::List(_) = &node.data.borrow().value {
                if let Some(prev_list_node) = maybe_prev_list_node {
                    let prev_position = prev_list_node.data.borrow().sourcepos;
                    let position = node.data.borrow().sourcepos;
                    match node_ctx.list_level {
                        Some(list_level) if position.start.column > prev_position.start.column => {
                            node_ctx.list_level = Some(list_level + 1);
                        }
                        Some(_) if position.start.column == prev_position.start.column => {}
                        _ => {
                            node_ctx.list_level = Some(1);
                        }
                    }
                }

                maybe_prev_list_node = Some(node);
            }

            for rule in self.new_rules.iter_mut() {
                if let RuleType::Node(node_rule) = rule {
                    if node_rule.matcher().is_match(node) {
                        let rule_violations = node_rule.run(&node_ctx, node)?;
                        violations.extend(rule_violations);
                    }
                }
            }
        }

        let mut line_ctx = LineContext {
            path: doc.path.clone(),
            lineno: 0,
        };
        for line in doc.text.lines() {
            line_ctx.lineno += 1;
            for rule in self.new_rules.iter() {
                if let RuleType::Line(line_rule) = rule {
                    if line_rule.matcher().is_match(line) {
                        let line_violations = line_rule.run(&line_ctx, line)?;
                        violations.extend(line_violations);
                    }
                }
            }
        }

        for rule in self.new_rules.iter_mut() {
            match rule {
                RuleType::Node(node_rule) => {
                    node_rule.reset();
                }
                RuleType::Line(line_rule) => {
                    line_rule.reset();
                }
            }
        }

        violations.sort();

        Ok(violations)
    }

    #[inline]
    pub fn new_check(&mut self, doc: &Document) -> Result<Vec<Violation>> {
        let node_ctx = NodeContext {
            path: doc.path.clone(),
            level: 0,
            list_level: None,
        };
        let mut violations = self.check_node_recursive(&node_ctx, doc.ast)?;

        let mut line_ctx = LineContext {
            path: doc.path.clone(),
            lineno: 0,
        };
        for line in doc.text.lines() {
            line_ctx.lineno += 1;
            for rule in self.new_rules.iter() {
                if let RuleType::Line(line_rule) = rule {
                    if line_rule.matcher().is_match(line) {
                        let line_violations = line_rule.run(&line_ctx, line)?;
                        violations.extend(line_violations);
                    }
                }
            }
        }

        for rule in self.new_rules.iter_mut() {
            match rule {
                RuleType::Node(node_rule) => {
                    node_rule.reset();
                }
                RuleType::Line(line_rule) => {
                    line_rule.reset();
                }
            }
        }

        violations.sort();

        Ok(violations)
    }

    #[inline]
    pub fn check(&self, doc: &Document) -> Result<Vec<Violation>> {
        // Iterate rules while unrolling Vec<Result<Vec<..>>> to Result<Vec<..>>
        let either_violations: Result<Vec<Violation>> =
            self.rules.iter().try_fold(vec![], |mut unrolled, rule| {
                let result = rule.check(doc);
                unrolled.extend(result?);
                Ok(unrolled)
            });

        either_violations.map(|mut violations| {
            violations.sort();
            violations
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
