//! TypeScript backend: [`ir::Api`] -> `.ts` source.
//!
//! Split by what's being emitted:
//! - [`schema`]: `ir::Schema` -> `TSType` (named schemas + inline types)
//! - [`operation`]: `ir::Operation` -> `export type GetFoo = { method, path, request, response }`
//! - [`emit`]: `emit` config entries -> unions / object types over operations
//! - [`naming`]: identifier allocation shared by all of them
//! - [`export`]: the same, as data (`--emit-ir`)
//!
//! All the oxc plumbing lives in [`crate::builder`]; this module only decides
//! *what* to build, driven by [`Config`].

mod docs;
mod emit;
mod explain;
pub mod export;
mod generator;
mod naming;
mod operation;
mod schema;
mod selection;

use anyhow::Result;

use docs::DocLines;
use explain::Report;
pub(crate) use generator::Generator;

use crate::config::Config;
use crate::ir;

pub fn generate(api: &ir::Api, config: &Config) -> Result<String> {
    let code = Generator::run(api, config, |g| Ok(g.b.code_gen(g.statements()?)))?;
    Ok(match config.header.as_str() {
        "" => code,
        header => format!(
            "{header}

{code}"
        ),
    })
}

/// What the config selects from `api`, as a report: how many operations
/// `filter` keeps, and what each `emit` covers and skips.
pub fn explain(api: &ir::Api, config: &Config) -> Result<String> {
    Generator::run(api, config, |g| {
        g.statements()?;
        Ok(Report::new(g).to_string())
    })
}
