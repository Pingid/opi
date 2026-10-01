//! OpenAPI -> TypeScript type generation.
//!
//! ```text
//!  spec file ──► Spec ──lower()──► ir::Api ──ts::generate(config)──► .ts
//!               (serde model)      (resolved,        (oxc AST via
//!                                   format-agnostic)  builder::TypeBuilder)
//!                                            └──ts::export──► IR .json (--emit-ir)
//! ```
//!
//! The public surface is what the CLI needs: [`Spec::from_file`],
//! [`Config::from_file`] / [`Config::default`], and [`generate`],
//! [`generate_ir`] and [`explain`]. Inside:
//!
//! - `openapi`: frontend. Parses OpenAPI 3.0 / 3.1 / 3.2 and lowers it into the
//!   IR, inlining parameter / body / response refs.
//! - `ir`: the intermediate AST. The contract between frontends and backends.
//! - `ts`: backend. Emits TS types from the IR, shaped by [`Config`]; and
//!   `ts::export`, the IR with its types rendered, as versioned JSON for
//!   generators of your own.
//! - `config`: YAML / JSON settings; `select` and `template` are the `where`
//!   selectors and `"{Method}{Path}"` templates it's built on.
//! - `builder`: thin helpers over `oxc_ast` / `oxc_codegen`.
//! - `reflect` (tests, feature `facet`): writes the commented default config
//!   and the JSON Schema / TypeScript types for the config and the IR.
//!
//! # Default config
//!
//! [`Config::default`], as written out by `reflect` to `config.default.yaml`:
//!
#![doc = concat!("```yaml\n", include_str!("../config.default.yaml"), "```")]

mod builder;
mod case;
mod config;
mod ir;
mod list;
mod load;
mod openapi;
#[cfg(all(test, feature = "facet"))]
mod reflect;
mod select;
mod template;
mod ts;

pub use config::Config;
pub use openapi::Spec;

/// [`Config::default`] as commented YAML (the checked-in `config.default.yaml`).
pub const DEFAULT_CONFIG_YAML: &str = include_str!("../config.default.yaml");

/// Spec -> TS source.
pub fn generate(spec: &Spec, config: &Config) -> anyhow::Result<String> {
    ts::generate(&spec.lower()?, config)
}

/// Spec -> the IR as JSON, with its types rendered (`ts::export`).
pub fn generate_ir(spec: &Spec, config: &Config) -> anyhow::Result<String> {
    ts::export::emit(&spec.lower()?, config)
}

/// Spec -> a report of what `config` selects from it (`--explain`).
pub fn explain(spec: &Spec, config: &Config) -> anyhow::Result<String> {
    ts::explain(&spec.lower()?, config)
}
