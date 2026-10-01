//! `--emit-ir`: the IR opi generates from, as JSON, with every schema's TS
//! source beside it. The escape hatch for output the config can't describe:
//!
//! ```text
//! opi spec.yaml --emit-ir > ir.json
//! node my-generator.mjs ir.json > custom.d.ts
//! ```
//!
//! It is the [`ir`](crate::ir) after `filter`, `schemas` and `formats` have
//! been applied (`emit` isn't), with each schema slot a [`Rendered`]: the
//! schema's own fields plus `ts`, the type the TS backend would write for it.
//! The `ts` sources refer to named schemas by their generated names, which
//! [`Export::names`] maps the raw names to. Optional fields are left out
//! rather than written as `null`.

use anyhow::Result;
use indexmap::IndexMap;
use serde::Serialize;

use super::Generator;
use crate::config::Config;
use crate::ir::{self, Operation, Schema};

/// [`Export::version`]; bumped on breaking changes to the format.
pub const VERSION: u32 = 3;

#[derive(Debug, Serialize)]
#[cfg_attr(feature = "facet", derive(facet::Facet))]
pub struct Export {
    /// Format version, bumped on breaking changes.
    pub version: u32,
    /// Raw schema name -> the TS name it's generated as, which is what the
    /// `ts` sources refer to.
    pub names: IndexMap<String, String>,
    /// Named schemas by raw name (what a `ref` names), after `schemas` pruning.
    pub schemas: IndexMap<String, Rendered>,
    pub operations: Vec<Operation<Rendered>>,
}

/// A schema with its TS source beside it.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "facet", derive(facet::Facet))]
pub struct Rendered {
    #[serde(flatten)]
    #[cfg_attr(feature = "facet", facet(flatten))]
    pub schema: Schema,
    pub ts: String,
}

/// The export as pretty-printed JSON.
pub fn emit(api: &ir::Api, config: &Config) -> Result<String> {
    Ok(serde_json::to_string_pretty(&export(api, config)?)? + "\n")
}

pub fn export(api: &ir::Api, config: &Config) -> Result<Export> {
    // Bare types: docs belong to whatever the consumer generates.
    let config = Config {
        jsdoc: false,
        ..config.clone()
    };
    Generator::run(api, &config, |g| Ok(g.export()))
}

impl Generator<'_> {
    fn export(&self) -> Export {
        let render = |schema: &Schema| Rendered {
            schema: schema.clone(),
            ts: self.b.type_to_string(self.schema(schema)),
        };
        let name = |raw: &str| {
            self.names
                .schema(raw)
                .expect("all schemas are named up front")
        };
        let schemas = &self.sel.schemas;
        Export {
            version: VERSION,
            names: schemas
                .iter()
                .map(|&raw| (raw.to_string(), name(raw).to_string()))
                .collect(),
            schemas: schemas
                .iter()
                .map(|&raw| (raw.to_string(), render(&self.api.schemas[raw])))
                .collect(),
            operations: self.sel.ops.iter().map(|op| op.map(render)).collect(),
        }
    }
}
