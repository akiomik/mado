use std::fs;
use std::path::Path;

use etcetera::choose_base_strategy;
use etcetera::BaseStrategy as _;
use miette::miette;
use miette::IntoDiagnostic as _;
use miette::Result;
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

pub fn load<P: AsRef<Path>>(path: P) -> Result<Config> {
    let config_text = fs::read_to_string(path).into_diagnostic()?;
    toml::from_str(&config_text).map_err(|err| miette!(err))
}

pub fn resolve() -> Result<Config> {
    let local_path = Path::new("downlint.toml");
    let exists_local = fs::exists(local_path).into_diagnostic()?;
    if exists_local {
        return load(local_path);
    }

    let hidden_local_path = Path::new(".downlint.toml");
    let exists_hidden_local = fs::exists(hidden_local_path).into_diagnostic()?;
    if exists_hidden_local {
        return load(hidden_local_path);
    }

    let strategy = choose_base_strategy().into_diagnostic()?;
    let mut config_path = strategy.config_dir();
    config_path.push("downlint.toml");
    let exists_config = fs::exists(&config_path).into_diagnostic()?;
    if exists_config {
        return load(&config_path);
    }

    Ok(Config::default())
}
