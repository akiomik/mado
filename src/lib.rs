#[cfg(test)]
extern crate pretty_assertions;

mod cli;
pub mod collection;
pub mod command;
pub mod config;
mod document;
mod output;
pub mod rule;
pub mod service;
mod violation;

pub use cli::Cli;
pub use command::Command;
pub use config::Config;
pub use document::Document;
pub use rule::Rule;
pub use violation::Violation;
