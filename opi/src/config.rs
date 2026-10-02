use std::path::{Path, PathBuf};

use indexmap::IndexMap;
use schemars::JsonSchema;
use serde::Deserialize;

use crate::{rule::Rule, util::JsonOrYamlFile};

const DEFAULT_CONFIG: &str = include_str!("../../config.default.yaml");

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    /// Path or URL of this file's JSON Schema, for editors. Ignored.
    #[serde(rename = "$schema", default)]
    pub schema: Option<String>,

    /// Source OpenAPI spec to read (.json, .yaml or .yml). The CLI's
    /// `openapi` argument takes precedence. Relative to this file.
    /// Example:
    /// ```yaml
    /// source: ./schemas/openapi.yaml
    /// ```
    #[serde(default)]
    pub source: Option<PathBuf>,

    /// Schema `format` -> the TS type to use instead of the base type, for
    /// strings, numbers and integers. `{}` turns it off.
    /// Example:
    /// ```yaml
    /// formats: { binary: Blob, date-time: Date }
    /// ```
    #[serde(default = "default_formats")]
    pub formats: IndexMap<String, String>,

    /// Rules whose output goes to the CLI's `output` argument, or stdout.
    /// Empty when omitted; the built-in rules only apply without a config.
    #[serde(default)]
    pub rules: Vec<Rule>,

    /// Every other key is a file to write, relative to this file, with the
    /// rules that produce it.
    /// Example:
    /// ```yaml
    /// ./my.openapi.ts:
    ///   - each: "schema"
    ///     emit: { type: "{name}" }
    /// ```
    #[serde(flatten)]
    pub files: IndexMap<PathBuf, Vec<Rule>>,
}

impl Default for Config {
    fn default() -> Self {
        JsonOrYamlFile::<Config>::from_yaml(DEFAULT_CONFIG).unwrap()
    }
}

impl Config {
    pub fn from_file(path: &Path) -> anyhow::Result<Self> {
        JsonOrYamlFile::from_file(path)
    }

    /// The JSON Schema for config files, as checked in at `config.schema.json`.
    pub fn json_schema() -> schemars::Schema {
        schemars::schema_for!(Config)
    }
}

fn default_formats() -> IndexMap<String, String> {
    IndexMap::from([("binary".to_string(), "Blob".to_string())])
}
