use miette::Result;

use crate::violation::Violation;

mod md001;
mod md022;

pub trait Rule {
    fn name(&self) -> String;

    fn description(&self) -> String;

    fn tags(&self) -> Vec<String>;

    fn aliases(&self) -> Vec<String>;

    fn check(&self, doc: &markdown::mdast::Node) -> Result<Vec<Violation>>;
}

pub use md001::MD001;
pub use md022::MD022;
