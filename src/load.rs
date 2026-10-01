//! Reading a spec or config file by its extension.

use std::path::Path;

use anyhow::{Context, Result};

/// Read `path` and hand its extension (without the dot) and text to `parse`.
/// Errors read `reading <path>` or `in <path>: ...`.
pub(crate) fn file<T>(path: &Path, parse: impl FnOnce(&str, &str) -> Result<T>) -> Result<T> {
    let text =
        std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default();
    parse(ext, &text).with_context(|| format!("in {}", path.display()))
}
