use serde::Deserialize;

use crate::{output::Format, Rule};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Default)]
#[serde(default)]
pub struct Config {
    pub lint: Lint,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct Lint {
    #[serde(rename = "output-format")]
    pub output_format: Format,
    pub rules: Vec<Rule>,
}

impl Default for Lint {
    fn default() -> Self {
        Self {
            output_format: Format::Concise,
            rules: vec![
                Rule::MD001,
                Rule::MD002,
                Rule::MD003,
                Rule::MD004,
                Rule::MD005,
                Rule::MD006,
                Rule::MD007,
                Rule::MD009,
                Rule::MD010,
                Rule::MD012,
                Rule::MD013,
                Rule::MD014,
                Rule::MD018,
                Rule::MD019,
                Rule::MD022,
                Rule::MD023,
                Rule::MD024,
                Rule::MD025,
                Rule::MD026,
                Rule::MD027,
                Rule::MD028,
                Rule::MD029,
            ],
        }
    }
}
