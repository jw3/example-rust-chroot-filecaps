use std::path::Path;
use anyhow::Context;
use serde::Deserialize;

pub fn load<P: AsRef<Path>>(path: P) -> anyhow::Result<ChrootCfg> {
    let toml = std::fs::read_to_string(path)?;
    toml::from_str(toml.as_str()).context("failed to parse config file")
}

#[derive(Clone, Debug, Deserialize)]
pub struct ChrootCfg {
    pub exec: Vec<String>,
    pub tree: Vec<String>,
}
