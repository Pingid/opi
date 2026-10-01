//! Everything `build.rs` compiles: the config modules shared with the crate
//! (included from `src/`), and the build-only serialisers.
//!
//! The shared modules refer to each other as `crate::select` etc., so
//! `build.rs` re-exports them at its root.

#[path = "../src/case.rs"]
pub mod case;
#[path = "../src/config/mod.rs"]
pub mod config;
#[path = "../src/ir/method.rs"]
pub mod method;
#[path = "../src/select.rs"]
pub mod select;
#[path = "../src/template.rs"]
pub mod template;

pub mod ir {
    pub use super::method::Method;
}

pub mod fmt;
mod serialize;
