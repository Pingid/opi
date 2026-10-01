//! OpenAPI -> TypeScript type generation.
//!
//! ```text
//!  spec file ──► openapi::Spec ──lower()──► ir::Api ──ts::generate(config)──► .ts
//!                (serde model)              (resolved,        (oxc AST via
//!                                            format-agnostic)  builder::TypeBuilder)
//! ```
//!
//! - [`openapi`]: frontend. Parses OpenAPI 3.0 / 3.1 / 3.2 and lowers it into the IR,
//!   inlining parameter / body / response refs.
//! - [`ir`]: the intermediate AST. The contract between frontends and backends.
//! - [`ts`]: backend. Emits TS types from the IR, shaped by [`Config`].
//! - [`config`]: TOML-loadable settings; [`select`] and [`template`] are the
//!   `match = {..}` selectors and `"{Method}{Path}"` templates it's built on.
//! - [`builder`]: thin helpers over `oxc_ast` / `oxc_codegen`.
//! - [`json_schema`]: experimental raw-JSON-Schema -> TS path (not yet wired
//!   through the IR).
//!
//! # Default config
//!
//! Generated from [`Config`] by `build.rs` (also written to `config.toml`,
//! with `config.json` and the JSON Schema `schema.json` alongside):
//!
#![doc = concat!("```toml\n", include_str!(concat!(env!("OUT_DIR"), "/config.toml")), "```")]

pub mod builder;
pub mod case;
pub mod config;
pub mod ir;
pub mod openapi;
pub mod select;
pub mod template;
pub mod ts;

pub use config::Config;

/// [`Config::default`] as commented TOML (the generated `config.toml`).
pub const DEFAULT_CONFIG_TOML: &str = include_str!(concat!(env!("OUT_DIR"), "/config.toml"));

/// Convenience: spec -> TS source in one call.
pub fn generate(spec: &openapi::Spec, config: &Config) -> anyhow::Result<String> {
    let api = spec.lower()?;
    ts::generate(&api, config)
}
