use std::path::PathBuf;

use comrak::nodes::{AstNode, Sourcepos};
use miette::Result;
use serde::Deserialize;

use crate::{matcher::Matcher, violation::Violation, Document};

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

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[non_exhaustive]
pub enum Rule {
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

pub trait RuleLike: Send {
    fn name(&self) -> &'static str;

    fn description(&self) -> &'static str;

    fn tags(&self) -> Vec<&'static str>;

    fn aliases(&self) -> Vec<&'static str>;

    fn check(&self, doc: &Document) -> Result<Vec<Violation>>;

    #[inline]
    fn to_violation(&self, path: PathBuf, position: Sourcepos) -> Violation {
        Violation::new(
            path,
            self.name().to_owned(),
            self.aliases()[0].to_owned(),
            self.description().to_owned(),
            position,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeContext {
    pub path: PathBuf,
    pub level: usize,
    pub list_level: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineContext {
    pub path: PathBuf,
    pub lineno: usize,
}

pub enum RuleType {
    Node(Box<dyn NodeRule>),
    Line(Box<dyn LineRule>),
}

pub trait NodeRule: Send {
    fn matcher(&self) -> Matcher;

    fn run<'a>(&self, ctx: &NodeContext, node: &'a AstNode<'a>) -> Result<Vec<Violation>>;
}

pub trait LineRule: Send {
    fn matcher(&self) -> Matcher;

    fn run<'a>(&self, ctx: &LineContext, line: &str) -> Result<Vec<Violation>>;
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
