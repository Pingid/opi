//! Reading a config from YAML or JSON, by text or by file.

use std::path::Path;

use anyhow::{Result, anyhow, bail};

use super::Config;
use crate::load;

impl Config {
    pub fn from_yaml(source: &str) -> Result<Self> {
        // An empty (or all-comments) file is an empty config.
        let config: Self = if serde_yaml::from_str::<serde_yaml::Value>(source)?.is_null() {
            Self::default()
        } else {
            let de = serde_yaml::Deserializer::from_str(source);
            serde_path_to_error::deserialize(de).map_err(path_error)?
        };
        config.validate()?;
        Ok(config)
    }

    pub(crate) fn from_json(source: &str) -> Result<Self> {
        let mut de = serde_json::Deserializer::from_str(source);
        let config: Self = serde_path_to_error::deserialize(&mut de).map_err(path_error)?;
        config.validate()?;
        Ok(config)
    }

    /// By extension: `.yaml` / `.yml`, or `.json`.
    pub fn from_file(path: &Path) -> Result<Self> {
        load::file(path, |ext, text| match ext {
            "yaml" | "yml" => Self::from_yaml(text),
            "json" => Self::from_json(text),
            "json5" | "toml" => bail!("{ext} configs aren't supported; use YAML or JSON"),
            _ => bail!("config must be .yaml, .yml or .json"),
        })
    }
}

/// `emit[0].shape: unknown field ...`, with serde's own position when it has
/// one.
fn path_error<E: std::fmt::Display>(e: serde_path_to_error::Error<E>) -> anyhow::Error {
    let path = e.path().to_string();
    let inner = e.into_inner();
    if path == "." {
        anyhow!("{inner}")
    } else {
        anyhow!("{path}: {inner}")
    }
}
