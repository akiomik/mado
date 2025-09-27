use globset::{Glob, GlobSet, GlobSetBuilder};
use miette::{IntoDiagnostic as _, Result};
use serde::{Deserialize, Serialize};

use crate::{output::Format, rule, rule::Rule};

mod md002;
mod md003;
mod md004;
mod md007;
mod md013;
mod md024;
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
pub use md024::MD024;
pub use md025::MD025;
pub use md026::MD026;
pub use md029::MD029;
pub use md030::MD030;
pub use md033::MD033;
pub use md035::MD035;
pub use md036::MD036;
pub use md041::MD041;
pub use md046::MD046;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
#[allow(clippy::exhaustive_structs)]
pub struct Lint {
    pub respect_ignore: bool,
    pub respect_gitignore: bool,
    pub output_format: Format,
    pub quiet: bool,
    pub exclude: Vec<Glob>,
    pub rules: Vec<RuleSet>,
    pub md002: MD002,
    pub md003: MD003,
    pub md004: MD004,
    pub md007: MD007,
    pub md013: MD013,
    pub md024: MD024,
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

impl Lint {
    #[inline]
    pub fn exclude_set(&self) -> Result<GlobSet> {
        let mut builder = GlobSetBuilder::new();
        for glob in &self.exclude {
            builder.add(glob.clone());
        }
        builder.build().into_diagnostic()
    }

    fn flatten_rules(&self) -> Vec<RuleSet> {
        let mut flatten: Vec<_> = self
            .rules
            .iter()
            .flat_map(|rule| match rule {
                // TODO: Use rule::Metadata#tags
                RuleSet::Atx => vec![RuleSet::MD018, RuleSet::MD019],
                RuleSet::AtxClosed => vec![RuleSet::MD020, RuleSet::MD021],
                RuleSet::BlankLines => vec![
                    RuleSet::MD012,
                    RuleSet::MD022,
                    RuleSet::MD031,
                    RuleSet::MD032,
                    RuleSet::MD047,
                ],
                RuleSet::Blockquote => vec![RuleSet::MD027, RuleSet::MD028],
                RuleSet::Bullet => vec![
                    RuleSet::MD004,
                    RuleSet::MD005,
                    RuleSet::MD006,
                    RuleSet::MD007,
                    RuleSet::MD032,
                ],
                RuleSet::Code => vec![
                    RuleSet::MD014,
                    RuleSet::MD031,
                    RuleSet::MD038,
                    RuleSet::MD040,
                    RuleSet::MD046,
                ],
                RuleSet::Emphasis => vec![RuleSet::MD036, RuleSet::MD037],
                RuleSet::HardTab => vec![RuleSet::MD010],
                RuleSet::Headers => vec![
                    RuleSet::MD001,
                    RuleSet::MD002,
                    RuleSet::MD003,
                    RuleSet::MD018,
                    RuleSet::MD019,
                    RuleSet::MD020,
                    RuleSet::MD021,
                    RuleSet::MD022,
                    RuleSet::MD023,
                    RuleSet::MD024,
                    RuleSet::MD025,
                    RuleSet::MD026,
                    RuleSet::MD036,
                    RuleSet::MD041,
                ],
                RuleSet::Hr => vec![RuleSet::MD035],
                RuleSet::Html => vec![RuleSet::MD033],
                RuleSet::Indentation => vec![
                    RuleSet::MD005,
                    RuleSet::MD006,
                    RuleSet::MD007,
                    RuleSet::MD027,
                ],
                RuleSet::Language => vec![RuleSet::MD040],
                RuleSet::LineLength => vec![RuleSet::MD013],
                RuleSet::Links => vec![RuleSet::MD034, RuleSet::MD039],
                RuleSet::Ol => vec![RuleSet::MD029, RuleSet::MD030, RuleSet::MD032],
                RuleSet::Spaces => vec![
                    RuleSet::MD018,
                    RuleSet::MD019,
                    RuleSet::MD020,
                    RuleSet::MD021,
                    RuleSet::MD023,
                ],
                RuleSet::Ul => vec![
                    RuleSet::MD004,
                    RuleSet::MD005,
                    RuleSet::MD006,
                    RuleSet::MD007,
                    RuleSet::MD030,
                    RuleSet::MD032,
                ],
                RuleSet::Url => vec![RuleSet::MD034],
                RuleSet::Whitespace => vec![
                    RuleSet::MD009,
                    RuleSet::MD010,
                    RuleSet::MD012,
                    RuleSet::MD027,
                    RuleSet::MD028,
                    RuleSet::MD030,
                    RuleSet::MD037,
                    RuleSet::MD038,
                    RuleSet::MD039,
                ],
                ruleset => vec![ruleset.clone()],
            })
            .collect();
        flatten.sort();
        flatten.dedup();
        flatten
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
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
    #[serde(rename = "atx")]
    Atx,
    #[serde(rename = "atx-closed")]
    AtxClosed,
    #[serde(rename = "blank-lines")]
    BlankLines,
    #[serde(rename = "blockquote")]
    Blockquote,
    #[serde(rename = "bullet")]
    Bullet,
    #[serde(rename = "code")]
    Code,
    #[serde(rename = "emphasis")]
    Emphasis,
    #[serde(rename = "hard-tab")]
    HardTab,
    #[serde(rename = "headers")]
    Headers,
    #[serde(rename = "hr")]
    Hr,
    #[serde(rename = "html")]
    Html,
    #[serde(rename = "indentation")]
    Indentation,
    #[serde(rename = "language")]
    Language,
    #[serde(rename = "line-length")]
    LineLength,
    #[serde(rename = "links")]
    Links,
    #[serde(rename = "ol")]
    Ol,
    #[serde(rename = "spaces")]
    Spaces,
    #[serde(rename = "ul")]
    Ul,
    #[serde(rename = "url")]
    Url,
    #[serde(rename = "whitespace")]
    Whitespace,
}

impl Default for Lint {
    #[inline]
    fn default() -> Self {
        Self {
            respect_ignore: true,
            respect_gitignore: true,
            output_format: Format::Concise,
            quiet: false,
            exclude: vec![],
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
            md024: MD024::default(),
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

impl Lint {
    #[inline]
    #[must_use]
    pub fn to_rules(&self) -> Vec<Rule> {
        self.flatten_rules()
            .iter()
            .map(|rule| match rule {
                RuleSet::MD001 => Rule::MD001(rule::MD001::new()),
                RuleSet::MD002 => Rule::MD002(rule::MD002::from(&self.md002)),
                RuleSet::MD003 => Rule::MD003(rule::MD003::from(&self.md003)),
                RuleSet::MD004 => Rule::MD004(rule::MD004::from(&self.md004)),
                RuleSet::MD005 => Rule::MD005(rule::MD005::new()),
                RuleSet::MD006 => Rule::MD006(rule::MD006::new()),
                RuleSet::MD007 => Rule::MD007(rule::MD007::from(&self.md007)),
                RuleSet::MD009 => Rule::MD009(rule::MD009::new()),
                RuleSet::MD010 => Rule::MD010(rule::MD010::new()),
                RuleSet::MD012 => Rule::MD012(rule::MD012::new()),
                RuleSet::MD013 => Rule::MD013(rule::MD013::from(&self.md013)),
                RuleSet::MD014 => Rule::MD014(rule::MD014::new()),
                RuleSet::MD018 => Rule::MD018(rule::MD018::new()),
                RuleSet::MD019 => Rule::MD019(rule::MD019::new()),
                RuleSet::MD020 => Rule::MD020(rule::MD020::new()),
                RuleSet::MD021 => Rule::MD021(rule::MD021::new()),
                RuleSet::MD022 => Rule::MD022(rule::MD022::new()),
                RuleSet::MD023 => Rule::MD023(rule::MD023::new()),
                RuleSet::MD024 => Rule::MD024(rule::MD024::from(&self.md024)),
                RuleSet::MD025 => Rule::MD025(rule::MD025::from(&self.md025)),
                RuleSet::MD026 => Rule::MD026(rule::MD026::from(&self.md026)),
                RuleSet::MD027 => Rule::MD027(rule::MD027::new()),
                RuleSet::MD028 => Rule::MD028(rule::MD028::new()),
                RuleSet::MD029 => Rule::MD029(rule::MD029::from(&self.md029)),
                RuleSet::MD030 => Rule::MD030(rule::MD030::from(&self.md030)),
                RuleSet::MD031 => Rule::MD031(rule::MD031::new()),
                RuleSet::MD032 => Rule::MD032(rule::MD032::new()),
                RuleSet::MD033 => Rule::MD033(rule::MD033::from(&self.md033)),
                RuleSet::MD034 => Rule::MD034(rule::MD034::new()),
                RuleSet::MD035 => Rule::MD035(rule::MD035::from(&self.md035)),
                RuleSet::MD036 => Rule::MD036(rule::MD036::from(&self.md036)),
                RuleSet::MD037 => Rule::MD037(rule::MD037::new()),
                RuleSet::MD038 => Rule::MD038(rule::MD038::new()),
                RuleSet::MD039 => Rule::MD039(rule::MD039::new()),
                RuleSet::MD040 => Rule::MD040(rule::MD040::new()),
                RuleSet::MD041 => Rule::MD041(rule::MD041::from(&self.md041)),
                RuleSet::MD046 => Rule::MD046(rule::MD046::from(&self.md046)),
                RuleSet::MD047 => Rule::MD047(rule::MD047::new()),
                _ => unreachable!("tags are flatten"),
            })
            .collect()
    }
}

// Note: public conversion moved to inherent method `Lint::to_rules()`.

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use rule::Tag;

    use super::*;

    #[test]
    fn exclude_set() -> Result<()> {
        let config = Lint {
            exclude: vec![
                Glob::new("*.md").into_diagnostic()?,
                Glob::new("foo/bar/baz/test.md").into_diagnostic()?,
                Glob::new("foo/**/test.md").into_diagnostic()?,
            ],
            ..Lint::default()
        };

        let set = config.exclude_set()?;

        assert_eq!(set.matches("foo/bar/test.md"), vec![0, 2]);
        Ok(())
    }

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
            Rule::MD024(rule::MD024::default()),
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
        assert_eq!(config.to_rules(), expected);
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn from_lint_for_vec_rule_unique() {
        let config = Lint {
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
                RuleSet::Atx,
                RuleSet::AtxClosed,
                RuleSet::BlankLines,
                RuleSet::Blockquote,
                RuleSet::Bullet,
                RuleSet::Code,
                RuleSet::Emphasis,
                RuleSet::HardTab,
                RuleSet::Headers,
                RuleSet::Hr,
                RuleSet::Html,
                RuleSet::Indentation,
                RuleSet::Language,
                RuleSet::LineLength,
                RuleSet::Links,
                RuleSet::Ol,
                RuleSet::Spaces,
                RuleSet::Ul,
                RuleSet::Url,
                RuleSet::Whitespace,
            ],
            ..Lint::default()
        };
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
            Rule::MD024(rule::MD024::default()),
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
        assert_eq!(config.to_rules(), expected);
    }

    #[test]
    fn from_lint_for_vec_rule_tag_association() {
        let ruleset_list = vec![
            (RuleSet::Atx, Tag::Atx),
            (RuleSet::AtxClosed, Tag::AtxClosed),
            (RuleSet::BlankLines, Tag::BlankLines),
            (RuleSet::Blockquote, Tag::Blockquote),
            (RuleSet::Bullet, Tag::Bullet),
            (RuleSet::Code, Tag::Code),
            (RuleSet::Emphasis, Tag::Emphasis),
            (RuleSet::HardTab, Tag::HardTab),
            (RuleSet::Headers, Tag::Headers),
            (RuleSet::Hr, Tag::Hr),
            (RuleSet::Html, Tag::Html),
            (RuleSet::Indentation, Tag::Indentation),
            (RuleSet::Language, Tag::Language),
            (RuleSet::LineLength, Tag::LineLength),
            (RuleSet::Links, Tag::Links),
            (RuleSet::Ol, Tag::Ol),
            (RuleSet::Spaces, Tag::Spaces),
            (RuleSet::Ul, Tag::Ul),
            (RuleSet::Url, Tag::Url),
            (RuleSet::Whitespace, Tag::Whitespace),
        ];

        for (ruleset, tag) in ruleset_list {
            let config = Lint {
                rules: vec![ruleset],
                ..Lint::default()
            };
            let rules = config.to_rules();
            for rule in rules {
                assert!(rule.metadata().tags.contains(&tag));
            }
        }
    }
}
