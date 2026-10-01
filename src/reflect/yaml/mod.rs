//! Serialise a facet value to commented, human-oriented YAML: how
//! [`super::example_config`] writes the default config (`config.default.yaml`)
//! straight from `Config`, so the file and its comments can't drift from the
//! code.
//!
//! Works in two steps:
//! 1. [`TreeSerializer`] records facet's serialisation events as a small
//!    [`Node`] tree, with field / type doc comments attached.
//! 2. [`render`] lays the tree out as block YAML, with each key's docs as
//!    `#` comments above it. `None` is written as `null` (so documented
//!    options still appear) unless the field skips it.

mod render;
mod tree;

use core::fmt::Write;

use facet_format::SerializeError;
use facet_reflect::Peek;

use render::render;
use tree::TreeSerializer;

#[derive(Debug, Clone)]
pub struct SerializeOptions {
    /// Write doc comments (field docs, type docs for sections, and the root
    /// type's docs as a file header).
    pub docs: bool,
    /// Comment lines for the top of the file, before the docs.
    pub header: Vec<String>,
}

impl Default for SerializeOptions {
    fn default() -> Self {
        Self {
            docs: true,
            header: Vec::new(),
        }
    }
}

#[derive(Debug)]
pub struct YamlSerializeError {
    msg: String,
}

impl core::fmt::Display for YamlSerializeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(&self.msg)
    }
}

impl std::error::Error for YamlSerializeError {}

fn error(msg: &str) -> YamlSerializeError {
    YamlSerializeError { msg: msg.into() }
}

pub fn to_string<'facet, T: facet::Facet<'facet>>(
    value: &T,
    options: &SerializeOptions,
) -> Result<String, SerializeError<YamlSerializeError>> {
    let mut ser = TreeSerializer::default();
    facet_format::serialize_root(&mut ser, Peek::new(value))?;
    let root = ser.root.ok_or(SerializeError::Backend(error(
        "the root value must be a struct or map",
    )))?;
    Ok(render(&root, options))
}

/// Bare when that can't be misread (`name`, `request.content_type`),
/// otherwise quoted (`"{path}"`).
pub(super) fn yaml_key(key: &str) -> String {
    let bare = key.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_')
        && key
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'))
        && !matches!(
            key,
            "null" | "true" | "false" | "yes" | "no" | "on" | "off" | "y" | "n"
        );
    if bare {
        key.to_string()
    } else {
        yaml_string(key)
    }
}

/// Always double-quoted: a JSON string is a valid YAML one, and quoting
/// sidesteps YAML's many special plain scalars (`{`, `: `, `#`, `null`, ...).
pub(super) fn yaml_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str(r#"\""#),
            '\\' => out.push_str(r"\\"),
            '\n' => out.push_str(r"\n"),
            '\r' => out.push_str(r"\r"),
            '\t' => out.push_str(r"\t"),
            c if c.is_control() => write!(out, "\\u{:04X}", c as u32).unwrap(),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}
