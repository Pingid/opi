use std::{
    marker::PhantomData,
    path::{Path, PathBuf},
};

use anyhow::Context;
use serde::de::DeserializeOwned;

pub struct JsonOrYamlFile<T> {
    pub path: PathBuf,
    _marker: PhantomData<T>,
}

impl<T: DeserializeOwned> JsonOrYamlFile<T> {
    pub fn from_file(path: &Path) -> anyhow::Result<T> {
        let content = Self::read_file(path)?;
        match path.extension() {
            Some(ext) if ext == "json" => Self::from_json(&content),
            Some(ext) if ext == "yaml" || ext == "yml" => Self::from_yaml(&content),
            _ => anyhow::bail!("File must be a JSON or YAML file"),
        }
    }

    pub fn from_json(string: &str) -> anyhow::Result<T> {
        serde_json::from_str(string).context("Failed to parse file as JSON")
    }

    pub fn from_yaml(string: &str) -> anyhow::Result<T> {
        serde_yaml::from_str(string).context("Failed to parse file as YAML")
    }

    fn read_file(path: &Path) -> anyhow::Result<String> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read file: {}", path.display()))?;
        Ok(content)
    }
}

pub struct JsonOrYaml<T> {
    _marker: PhantomData<T>,
}

impl<T: DeserializeOwned> JsonOrYaml<T> {
    pub fn deserialize(string: &str) -> anyhow::Result<T> {
        match serde_json::from_str(string) {
            Ok(value) => Ok(value),
            Err(_) => match serde_yaml::from_str(string) {
                Ok(value) => Ok(value),
                Err(_) => anyhow::bail!("Failed to deserialize as JSON or YAML"),
            },
        }
    }
}
