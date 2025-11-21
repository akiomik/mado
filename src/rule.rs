use std::path::PathBuf;

use comrak::nodes::Sourcepos;
use miette::Result;

use crate::{Document, violation::Violation};

mod helper;
mod md001;
mod md002;
pub mod md003;
pub mod md004;
mod md005;
mod md006;
mod md007;
mod md009;
mod md010;
mod md012;
mod md013;
mod md014;
mod md018;
mod md019;
mod md020;
mod md021;
mod md022;
mod md023;
mod md024;
mod md025;
mod md026;
mod md027;
mod md028;
pub mod md029;
mod md030;
mod md031;
mod md032;
mod md033;
mod md034;
pub mod md035;
mod md036;
mod md037;
mod md038;
mod md039;
mod md040;
mod md041;
pub mod md046;
mod md047;
mod metadata;
mod tag;

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Rule {
    MD001(MD001),
    MD002(MD002),
    MD003(MD003),
    MD004(MD004),
    MD005(MD005),
    MD006(MD006),
    MD007(MD007),
    MD009(MD009),
    MD010(MD010),
    MD012(MD012),
    MD013(MD013),
    MD014(MD014),
    MD018(MD018),
    MD019(MD019),
    MD020(MD020),
    MD021(MD021),
    MD022(MD022),
    MD023(MD023),
    MD024(MD024),
    MD025(MD025),
    MD026(MD026),
    MD027(MD027),
    MD028(MD028),
    MD029(MD029),
    MD030(MD030),
    MD031(MD031),
    MD032(MD032),
    MD033(MD033),
    MD034(MD034),
    MD035(MD035),
    MD036(MD036),
    MD037(MD037),
    MD038(MD038),
    MD039(MD039),
    MD040(MD040),
    MD041(MD041),
    MD046(MD046),
    MD047(MD047),
}

impl Rule {
    #[inline]
    pub fn check(&self, doc: &Document) -> Result<Vec<Violation>> {
        match self {
            Self::MD001(rule) => rule.check(doc),
            Self::MD002(rule) => rule.check(doc),
            Self::MD003(rule) => rule.check(doc),
            Self::MD004(rule) => rule.check(doc),
            Self::MD005(rule) => rule.check(doc),
            Self::MD006(rule) => rule.check(doc),
            Self::MD007(rule) => rule.check(doc),
            Self::MD009(rule) => rule.check(doc),
            Self::MD010(rule) => rule.check(doc),
            Self::MD012(rule) => rule.check(doc),
            Self::MD013(rule) => rule.check(doc),
            Self::MD014(rule) => rule.check(doc),
            Self::MD018(rule) => rule.check(doc),
            Self::MD019(rule) => rule.check(doc),
            Self::MD020(rule) => rule.check(doc),
            Self::MD021(rule) => rule.check(doc),
            Self::MD022(rule) => rule.check(doc),
            Self::MD023(rule) => rule.check(doc),
            Self::MD024(rule) => rule.check(doc),
            Self::MD025(rule) => rule.check(doc),
            Self::MD026(rule) => rule.check(doc),
            Self::MD027(rule) => rule.check(doc),
            Self::MD028(rule) => rule.check(doc),
            Self::MD029(rule) => rule.check(doc),
            Self::MD030(rule) => rule.check(doc),
            Self::MD031(rule) => rule.check(doc),
            Self::MD032(rule) => rule.check(doc),
            Self::MD033(rule) => rule.check(doc),
            Self::MD034(rule) => rule.check(doc),
            Self::MD035(rule) => rule.check(doc),
            Self::MD036(rule) => rule.check(doc),
            Self::MD037(rule) => rule.check(doc),
            Self::MD038(rule) => rule.check(doc),
            Self::MD039(rule) => rule.check(doc),
            Self::MD040(rule) => rule.check(doc),
            Self::MD041(rule) => rule.check(doc),
            Self::MD046(rule) => rule.check(doc),
            Self::MD047(rule) => rule.check(doc),
        }
    }

    #[inline]
    #[must_use]
    pub fn metadata(&self) -> &Metadata {
        match self {
            Self::MD001(rule) => rule.metadata(),
            Self::MD002(rule) => rule.metadata(),
            Self::MD003(rule) => rule.metadata(),
            Self::MD004(rule) => rule.metadata(),
            Self::MD005(rule) => rule.metadata(),
            Self::MD006(rule) => rule.metadata(),
            Self::MD007(rule) => rule.metadata(),
            Self::MD009(rule) => rule.metadata(),
            Self::MD010(rule) => rule.metadata(),
            Self::MD012(rule) => rule.metadata(),
            Self::MD013(rule) => rule.metadata(),
            Self::MD014(rule) => rule.metadata(),
            Self::MD018(rule) => rule.metadata(),
            Self::MD019(rule) => rule.metadata(),
            Self::MD020(rule) => rule.metadata(),
            Self::MD021(rule) => rule.metadata(),
            Self::MD022(rule) => rule.metadata(),
            Self::MD023(rule) => rule.metadata(),
            Self::MD024(rule) => rule.metadata(),
            Self::MD025(rule) => rule.metadata(),
            Self::MD026(rule) => rule.metadata(),
            Self::MD027(rule) => rule.metadata(),
            Self::MD028(rule) => rule.metadata(),
            Self::MD029(rule) => rule.metadata(),
            Self::MD030(rule) => rule.metadata(),
            Self::MD031(rule) => rule.metadata(),
            Self::MD032(rule) => rule.metadata(),
            Self::MD033(rule) => rule.metadata(),
            Self::MD034(rule) => rule.metadata(),
            Self::MD035(rule) => rule.metadata(),
            Self::MD036(rule) => rule.metadata(),
            Self::MD037(rule) => rule.metadata(),
            Self::MD038(rule) => rule.metadata(),
            Self::MD039(rule) => rule.metadata(),
            Self::MD040(rule) => rule.metadata(),
            Self::MD041(rule) => rule.metadata(),
            Self::MD046(rule) => rule.metadata(),
            Self::MD047(rule) => rule.metadata(),
        }
    }
}

pub trait RuleLike: Send {
    #[must_use]
    fn metadata(&self) -> &'static Metadata;

    fn check(&self, doc: &Document) -> Result<Vec<Violation>>;

    #[inline]
    fn to_violation(&self, path: PathBuf, position: Sourcepos) -> Violation {
        Violation::new(path, self.metadata(), position)
    }
}

pub use md001::MD001;
pub use md002::MD002;
pub use md003::MD003;
pub use md004::MD004;
pub use md005::MD005;
pub use md006::MD006;
pub use md007::MD007;
pub use md009::MD009;
pub use md010::MD010;
pub use md012::MD012;
pub use md013::MD013;
pub use md014::MD014;
pub use md018::MD018;
pub use md019::MD019;
pub use md020::MD020;
pub use md021::MD021;
pub use md022::MD022;
pub use md023::MD023;
pub use md024::MD024;
pub use md025::MD025;
pub use md026::MD026;
pub use md027::MD027;
pub use md028::MD028;
pub use md029::MD029;
pub use md030::MD030;
pub use md031::MD031;
pub use md032::MD032;
pub use md033::MD033;
pub use md034::MD034;
pub use md035::MD035;
pub use md036::MD036;
pub use md037::MD037;
pub use md038::MD038;
pub use md039::MD039;
pub use md040::MD040;
pub use md041::MD041;
pub use md046::MD046;
pub use md047::MD047;
pub use metadata::Metadata;
pub use tag::Tag;
