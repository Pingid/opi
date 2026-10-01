//! Writes the default config out, generated from the config types so it
//! can't drift from them: `config.toml` (commented, the main documentation),
//! `config.json`, and the JSON Schema `schema.json`.
//!
//! A build script can't depend on its own crate, so the config modules are
//! compiled in here directly (see `build/mod.rs`). They must stay free of the
//! rest of the crate (IR, backend).

#![allow(dead_code, unused_imports)]

use std::path::Path;

#[path = "build/mod.rs"]
mod build;

// Where the shared modules expect each other (`crate::select`, ...).
use build::{case, config, ir, select, template};

use config::Config;

/// Everything the output depends on.
const SOURCES: &[&str] = &[
    "build",
    "src/case.rs",
    "src/config/mod.rs",
    "src/ir/method.rs",
    "src/select.rs",
    "src/template.rs",
];

fn main() {
    for source in SOURCES {
        println!("cargo::rerun-if-changed={source}");
    }

    let config = Config::default();
    generated("config.toml", &config.to_toml().expect("default config -> TOML"));
    generated("config.json", &config.to_json("./schema.json").expect("default config -> JSON"));
    generated("schema.json", &Config::json_schema().expect("config JSON Schema"));
}

/// Writes `name` to `OUT_DIR` (for `include_str!`) and to the crate root (the
/// checked-in copy). The latter only when it changes, so an up-to-date
/// checkout isn't modified (which `cargo package` would reject).
fn generated(name: &str, contents: &str) {
    let out_dir = std::env::var("OUT_DIR").expect("cargo sets OUT_DIR");
    std::fs::write(Path::new(&out_dir).join(name), contents).unwrap();

    let checked_in = Path::new(env!("CARGO_MANIFEST_DIR")).join(name);
    if std::fs::read_to_string(&checked_in).ok().as_deref() != Some(contents) {
        std::fs::write(&checked_in, contents).unwrap();
    }
}
