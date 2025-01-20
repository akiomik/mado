use serde::Deserialize;

use crate::{output::Format, rule, rule::Rule};

mod md002;
mod md003;
mod md004;
mod md007;
mod md013;
mod md025;
mod md026;
mod md029;
mod md030;
mod md033;
mod md035;
mod md036;
mod md041;
mod md046;

pub use md002::MD002;
pub use md003::MD003;
pub use md004::MD004;
pub use md007::MD007;
pub use md013::MD013;
pub use md025::MD025;
pub use md026::MD026;
pub use md029::MD029;
pub use md030::MD030;
pub use md033::MD033;
pub use md035::MD035;
pub use md036::MD036;
pub use md041::MD041;
pub use md046::MD046;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
#[allow(clippy::exhaustive_structs)]
pub struct Lint {
    pub output_format: Format,
    pub quiet: bool,
    pub rules: Vec<RuleSet>,
    pub md002: MD002,
    pub md003: MD003,
    pub md004: MD004,
    pub md007: MD007,
    pub md013: MD013,
    pub md025: MD025,
    pub md026: MD026,
    pub md029: MD029,
    pub md030: MD030,
    pub md033: MD033,
    pub md035: MD035,
    pub md036: MD036,
    pub md041: MD041,
    pub md046: MD046,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[non_exhaustive]
pub enum RuleSet {
    MD001,
    MD002,
    MD003,
    MD004,
    MD005,
    MD006,
    MD007,
    MD009,
    MD010,
    MD012,
    MD013,
    MD014,
    MD018,
    MD019,
    MD020,
    MD021,
    MD022,
    MD023,
    MD024,
    MD025,
    MD026,
    MD027,
    MD028,
    MD029,
    MD030,
    MD031,
    MD032,
    MD033,
    MD034,
    MD035,
    MD036,
    MD037,
    MD038,
    MD039,
    MD040,
    MD041,
    MD046,
    MD047,
}

impl Default for Lint {
    #[inline]
    fn default() -> Self {
        Self {
            output_format: Format::Concise,
            quiet: false,
            rules: vec![
                RuleSet::MD001,
                RuleSet::MD002,
                RuleSet::MD003,
                RuleSet::MD004,
                RuleSet::MD005,
                RuleSet::MD006,
                RuleSet::MD007,
                RuleSet::MD009,
                RuleSet::MD010,
                RuleSet::MD012,
                RuleSet::MD013,
                RuleSet::MD014,
                RuleSet::MD018,
                RuleSet::MD019,
                RuleSet::MD020,
                RuleSet::MD021,
                RuleSet::MD022,
                RuleSet::MD023,
                RuleSet::MD024,
                RuleSet::MD025,
                RuleSet::MD026,
                RuleSet::MD027,
                RuleSet::MD028,
                RuleSet::MD029,
                RuleSet::MD030,
                RuleSet::MD031,
                RuleSet::MD032,
                RuleSet::MD033,
                RuleSet::MD034,
                RuleSet::MD035,
                RuleSet::MD036,
                RuleSet::MD037,
                RuleSet::MD038,
                RuleSet::MD039,
                RuleSet::MD040,
                RuleSet::MD041,
                RuleSet::MD046,
                RuleSet::MD047,
            ],
            md002: MD002::default(),
            md003: MD003::default(),
            md004: MD004::default(),
            md007: MD007::default(),
            md013: MD013::default(),
            md025: MD025::default(),
            md026: MD026::default(),
            md029: MD029::default(),
            md030: MD030::default(),
            md033: MD033::default(),
            md035: MD035::default(),
            md036: MD036::default(),
            md041: MD041::default(),
            md046: MD046::default(),
        }
    }
}

impl From<&Lint> for Vec<Rule> {
    #[inline]
    #[must_use]
    fn from(config: &Lint) -> Self {
        config
            .rules
            .iter()
            .map(|rule| match rule {
                RuleSet::MD001 => Rule::MD001(rule::MD001::new()),
                RuleSet::MD002 => Rule::MD002(rule::MD002::from(&config.md002)),
                RuleSet::MD003 => Rule::MD003(rule::MD003::from(&config.md003)),
                RuleSet::MD004 => Rule::MD004(rule::MD004::from(&config.md004)),
                RuleSet::MD005 => Rule::MD005(rule::MD005::new()),
                RuleSet::MD006 => Rule::MD006(rule::MD006::new()),
                RuleSet::MD007 => Rule::MD007(rule::MD007::from(&config.md007)),
                RuleSet::MD009 => Rule::MD009(rule::MD009::new()),
                RuleSet::MD010 => Rule::MD010(rule::MD010::new()),
                RuleSet::MD012 => Rule::MD012(rule::MD012::new()),
                RuleSet::MD013 => Rule::MD013(rule::MD013::from(&config.md013)),
                RuleSet::MD014 => Rule::MD014(rule::MD014::new()),
                RuleSet::MD018 => Rule::MD018(rule::MD018::new()),
                RuleSet::MD019 => Rule::MD019(rule::MD019::new()),
                RuleSet::MD020 => Rule::MD020(rule::MD020::new()),
                RuleSet::MD021 => Rule::MD021(rule::MD021::new()),
                RuleSet::MD022 => Rule::MD022(rule::MD022::new()),
                RuleSet::MD023 => Rule::MD023(rule::MD023::new()),
                RuleSet::MD024 => Rule::MD024(rule::MD024::new()),
                RuleSet::MD025 => Rule::MD025(rule::MD025::from(&config.md025)),
                RuleSet::MD026 => Rule::MD026(rule::MD026::from(&config.md026)),
                RuleSet::MD027 => Rule::MD027(rule::MD027::new()),
                RuleSet::MD028 => Rule::MD028(rule::MD028::new()),
                RuleSet::MD029 => Rule::MD029(rule::MD029::from(&config.md029)),
                RuleSet::MD030 => Rule::MD030(rule::MD030::from(&config.md030)),
                RuleSet::MD031 => Rule::MD031(rule::MD031::new()),
                RuleSet::MD032 => Rule::MD032(rule::MD032::new()),
                RuleSet::MD033 => Rule::MD033(rule::MD033::from(&config.md033)),
                RuleSet::MD034 => Rule::MD034(rule::MD034::new()),
                RuleSet::MD035 => Rule::MD035(rule::MD035::from(&config.md035)),
                RuleSet::MD036 => Rule::MD036(rule::MD036::from(&config.md036)),
                RuleSet::MD037 => Rule::MD037(rule::MD037::new()),
                RuleSet::MD038 => Rule::MD038(rule::MD038::new()),
                RuleSet::MD039 => Rule::MD039(rule::MD039::new()),
                RuleSet::MD040 => Rule::MD040(rule::MD040::new()),
                RuleSet::MD041 => Rule::MD041(rule::MD041::from(&config.md041)),
                RuleSet::MD046 => Rule::MD046(rule::MD046::from(&config.md046)),
                RuleSet::MD047 => Rule::MD047(rule::MD047::new()),
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn from_lint_for_vec_rule() {
        let config = Lint::default();
        let expected = vec![
            Rule::MD001(rule::MD001::new()),
            Rule::MD002(rule::MD002::default()),
            Rule::MD003(rule::MD003::default()),
            Rule::MD004(rule::MD004::default()),
            Rule::MD005(rule::MD005::new()),
            Rule::MD006(rule::MD006::new()),
            Rule::MD007(rule::MD007::default()),
            Rule::MD009(rule::MD009::new()),
            Rule::MD010(rule::MD010::new()),
            Rule::MD012(rule::MD012::new()),
            Rule::MD013(rule::MD013::default()),
            Rule::MD014(rule::MD014::new()),
            Rule::MD018(rule::MD018::new()),
            Rule::MD019(rule::MD019::new()),
            Rule::MD020(rule::MD020::new()),
            Rule::MD021(rule::MD021::new()),
            Rule::MD022(rule::MD022::new()),
            Rule::MD023(rule::MD023::new()),
            Rule::MD024(rule::MD024::new()),
            Rule::MD025(rule::MD025::default()),
            Rule::MD026(rule::MD026::default()),
            Rule::MD027(rule::MD027::new()),
            Rule::MD028(rule::MD028::new()),
            Rule::MD029(rule::MD029::default()),
            Rule::MD030(rule::MD030::default()),
            Rule::MD031(rule::MD031::new()),
            Rule::MD032(rule::MD032::new()),
            Rule::MD033(rule::MD033::default()),
            Rule::MD034(rule::MD034::new()),
            Rule::MD035(rule::MD035::default()),
            Rule::MD036(rule::MD036::default()),
            Rule::MD037(rule::MD037::new()),
            Rule::MD038(rule::MD038::new()),
            Rule::MD039(rule::MD039::new()),
            Rule::MD040(rule::MD040::new()),
            Rule::MD041(rule::MD041::default()),
            Rule::MD046(rule::MD046::default()),
            Rule::MD047(rule::MD047::new()),
        ];
        assert_eq!(Vec::from(&config), expected);
    }
}
