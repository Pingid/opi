//! Serialise a facet value to commented, human-oriented TOML. Used by
//! `build.rs` to write the default config (`config.toml`) straight from
//! `Config`, so the file and its comments can't drift from the code.
//! Generic over any facet type; `tests/config_toml.rs` also includes it.
//!
//! Works in two steps, because TOML's layout rules (all of a table's plain
//! keys before its sub-tables; no comments inside inline tables) can't be
//! met while streaming facet's serialisation events:
//! 1. [`TreeSerializer`] records the events as a small [`Node`] tree, with
//!    field / type doc comments attached.
//! 2. [`render`] lays the tree out: root-level tables become `[section]`s and
//!    arrays of tables `[[section]]`s; anything deeper is written inline.
//!    `None` fields are omitted.

use core::fmt::Write;

use facet::{Def, Shape, StructKind, Type, UserType};
use facet_format::{FormatSerializer, ScalarValue, SerializeError};
use facet_reflect::{FieldItem, Peek};

#[derive(Debug, Clone)]
pub struct SerializeOptions {
    /// Write doc comments (field docs, type docs for sections, and the root
    /// type's docs as a file header).
    pub docs: bool,
}

impl Default for SerializeOptions {
    fn default() -> Self {
        Self { docs: true }
    }
}

#[derive(Debug)]
pub struct TomlSerializeError {
    msg: String,
}

impl core::fmt::Display for TomlSerializeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(&self.msg)
    }
}

impl std::error::Error for TomlSerializeError {}

fn error(msg: &str) -> TomlSerializeError {
    TomlSerializeError { msg: msg.into() }
}

pub fn to_string<'facet, T: facet::Facet<'facet>>(
    value: &T,
    options: &SerializeOptions,
) -> Result<String, SerializeError<TomlSerializeError>> {
    let mut ser = TreeSerializer::default();
    facet_format::serialize_root(&mut ser, Peek::new(value))?;
    let root = ser
        .root
        .ok_or(SerializeError::Backend(error("the root value must be a struct or map")))?;
    Ok(render(&root, options))
}

// ---------------------------------------------------------------------------
// Tree
// ---------------------------------------------------------------------------

#[derive(Debug)]
enum Node {
    /// Already rendered as TOML (`"text"`, `true`, `3`).
    Scalar(String),
    Array(Vec<Node>),
    Table(Table),
}

#[derive(Debug, Default)]
struct Table {
    /// Docs of the table's type (used for section headers without field docs).
    doc: Vec<String>,
    entries: Vec<Entry>,
}

#[derive(Debug)]
struct Entry {
    key: String,
    doc: Vec<String>,
    value: Node,
}

enum Frame {
    Table {
        table: Table,
        /// Key (and its docs) of the entry currently being serialised.
        key: Option<(String, Vec<String>)>,
    },
    Array(Vec<Node>),
}

#[derive(Default)]
struct TreeSerializer {
    stack: Vec<Frame>,
    root: Option<Table>,
    /// Docs from `field_metadata_with_value`, waiting for `field_key`.
    field_doc: Vec<String>,
    /// Docs from `struct_metadata`, waiting for `begin_struct`.
    type_doc: Vec<String>,
}

impl TreeSerializer {
    fn push(&mut self, node: Node) -> Result<(), TomlSerializeError> {
        match self.stack.last_mut() {
            None => match node {
                Node::Table(table) => self.root = Some(table),
                _ => return Err(error("the root value must be a struct or map")),
            },
            Some(Frame::Table { table, key }) => {
                let (key, doc) = key.take().ok_or_else(|| error("value without a key"))?;
                table.entries.push(Entry { key, doc, value: node });
            }
            Some(Frame::Array(items)) => items.push(node),
        }
        Ok(())
    }

    /// `None` / unit: drop the pending key rather than write `key = `.
    fn skip(&mut self) {
        if let Some(Frame::Table { key, .. }) = self.stack.last_mut() {
            *key = None;
        }
    }

    fn begin_table(&mut self) {
        let doc = std::mem::take(&mut self.type_doc);
        self.stack.push(Frame::Table {
            table: Table { doc, entries: Vec::new() },
            key: None,
        });
    }

    fn end_table(&mut self) -> Result<(), TomlSerializeError> {
        match self.stack.pop() {
            Some(Frame::Table { table, .. }) => self.push(Node::Table(table)),
            _ => Err(error("end of table without a matching start")),
        }
    }
}

impl FormatSerializer for TreeSerializer {
    type Error = TomlSerializeError;

    fn struct_metadata(&mut self, shape: &Shape) -> Result<(), Self::Error> {
        self.type_doc = clean_doc(shape.doc);
        Ok(())
    }

    fn field_metadata_with_value(
        &mut self,
        field: &FieldItem,
        value: Peek<'_, '_>,
    ) -> Result<bool, Self::Error> {
        let mut doc = field.field.map(|f| clean_doc(f.doc)).unwrap_or_default();
        if let Some(hint) = variants_hint(value.shape()) {
            doc.push(hint);
        }
        self.field_doc = doc;
        Ok(false)
    }

    fn field_key(&mut self, key: &str) -> Result<(), Self::Error> {
        let doc = std::mem::take(&mut self.field_doc);
        match self.stack.last_mut() {
            Some(Frame::Table { key: pending, .. }) => {
                *pending = Some((key.to_string(), doc));
                Ok(())
            }
            _ => Err(error("field key outside of a table")),
        }
    }

    fn begin_struct(&mut self) -> Result<(), Self::Error> {
        self.begin_table();
        Ok(())
    }

    fn end_struct(&mut self) -> Result<(), Self::Error> {
        self.end_table()
    }

    fn begin_map_with_len(&mut self, _len: usize) -> Result<(), Self::Error> {
        self.begin_table();
        Ok(())
    }

    fn end_map(&mut self) -> Result<(), Self::Error> {
        self.end_table()
    }

    fn begin_seq(&mut self) -> Result<(), Self::Error> {
        self.stack.push(Frame::Array(Vec::new()));
        Ok(())
    }

    fn end_seq(&mut self) -> Result<(), Self::Error> {
        match self.stack.pop() {
            Some(Frame::Array(items)) => self.push(Node::Array(items)),
            _ => Err(error("end of array without a matching start")),
        }
    }

    fn serialize_none(&mut self) -> Result<(), Self::Error> {
        self.skip();
        Ok(())
    }

    fn scalar(&mut self, scalar: ScalarValue<'_>) -> Result<(), Self::Error> {
        let rendered = match scalar {
            // TOML has no null.
            ScalarValue::Null | ScalarValue::Unit => {
                self.skip();
                return Ok(());
            }
            ScalarValue::Bool(v) => v.to_string(),
            ScalarValue::Char(c) => toml_string(&c.to_string()),
            ScalarValue::I64(v) => v.to_string(),
            ScalarValue::U64(v) => v.to_string(),
            ScalarValue::I128(v) => v.to_string(),
            ScalarValue::U128(v) => v.to_string(),
            ScalarValue::F64(v) if v.is_nan() => "nan".to_string(),
            ScalarValue::F64(v) if v.is_infinite() => {
                if v > 0.0 { "inf" } else { "-inf" }.to_string()
            }
            // Keep floats recognisably floats (`1.0`, not `1`).
            ScalarValue::F64(v) if v.fract() == 0.0 => format!("{v:.1}"),
            ScalarValue::F64(v) => v.to_string(),
            ScalarValue::Str(s) => toml_string(&s),
            ScalarValue::Bytes(_) => return Err(error("TOML has no byte arrays")),
        };
        self.push(Node::Scalar(rendered))
    }
}

// ---------------------------------------------------------------------------
// Rendering
// ---------------------------------------------------------------------------

fn render(root: &Table, options: &SerializeOptions) -> String {
    let mut out = String::new();
    if options.docs && !root.doc.is_empty() {
        write_doc(&mut out, &root.doc);
        out.push('\n');
    }

    let (sections, plain): (Vec<_>, Vec<_>) =
        root.entries.iter().partition(|e| is_section(&e.value));
    write_entries(&mut out, plain.into_iter(), options.docs);

    for entry in sections {
        if !out.is_empty() {
            out.push('\n');
        }
        // Field docs, else the docs of the table's (or element's) type.
        let doc = match &entry.value {
            _ if !entry.doc.is_empty() => &entry.doc,
            Node::Table(t) => &t.doc,
            Node::Array(items) => match items.first() {
                Some(Node::Table(t)) => &t.doc,
                _ => &entry.doc,
            },
            Node::Scalar(_) => &entry.doc,
        };
        if options.docs {
            write_doc(&mut out, doc);
        }
        let key = toml_key(&entry.key);
        match &entry.value {
            Node::Table(table) => {
                writeln!(out, "[{key}]").unwrap();
                write_entries(&mut out, table.entries.iter(), options.docs);
            }
            Node::Array(items) => {
                for (i, item) in items.iter().enumerate() {
                    let Node::Table(table) = item else { unreachable!("checked by is_section") };
                    if i > 0 {
                        out.push('\n');
                    }
                    writeln!(out, "[[{key}]]").unwrap();
                    // Field docs once, on the first element, not on every one.
                    write_entries(&mut out, table.entries.iter(), options.docs && i == 0);
                }
            }
            Node::Scalar(_) => unreachable!("checked by is_section"),
        }
    }
    out
}

/// Root-level tables and non-empty arrays of tables get their own headers.
fn is_section(node: &Node) -> bool {
    match node {
        Node::Table(_) => true,
        Node::Array(items) => {
            !items.is_empty() && items.iter().all(|i| matches!(i, Node::Table(_)))
        }
        Node::Scalar(_) => false,
    }
}

/// `key = value` lines, with a blank line before each documented key so the
/// comments read as belonging to the line below them.
fn write_entries<'e>(out: &mut String, entries: impl Iterator<Item = &'e Entry>, docs: bool) {
    for (i, entry) in entries.enumerate() {
        if docs && !entry.doc.is_empty() {
            if i > 0 {
                out.push('\n');
            }
            write_doc(out, &entry.doc);
        }
        writeln!(out, "{} = {}", toml_key(&entry.key), inline(&entry.value)).unwrap();
    }
}

fn inline(node: &Node) -> String {
    match node {
        Node::Scalar(s) => s.clone(),
        Node::Array(items) => {
            let items: Vec<_> = items.iter().map(inline).collect();
            format!("[{}]", items.join(", "))
        }
        Node::Table(table) if table.entries.is_empty() => "{}".to_string(),
        Node::Table(table) => {
            let entries: Vec<_> = table
                .entries
                .iter()
                .map(|e| format!("{} = {}", toml_key(&e.key), inline(&e.value)))
                .collect();
            format!("{{ {} }}", entries.join(", "))
        }
    }
}

fn write_doc(out: &mut String, doc: &[String]) {
    for line in doc {
        if line.is_empty() {
            out.push_str("#\n");
        } else {
            writeln!(out, "# {line}").unwrap();
        }
    }
}

/// Rust doc lines -> comment lines: drop rustdoc's leading space and code
/// fences (keeping the example inside), and turn intra-doc links
/// (`` [`Foo`] ``) into plain code spans.
fn clean_doc(doc: &[&str]) -> Vec<String> {
    doc.iter()
        .map(|line| line.strip_prefix(' ').unwrap_or(line).trim_end())
        .filter(|line| !line.starts_with("```"))
        .map(|line| line.replace("[`", "`").replace("`]", "`"))
        .collect()
}

/// `one of: "split", "merged"` for fields holding a fieldless enum.
fn variants_hint(shape: &Shape) -> Option<String> {
    let shape = match shape.def {
        Def::Option(option) => option.t(),
        _ => shape,
    };
    let Type::User(UserType::Enum(enum_type)) = shape.ty else {
        return None;
    };
    let variants = enum_type.variants;
    if variants.is_empty() || variants.iter().any(|v| v.data.kind != StructKind::Unit) {
        return None;
    }
    let names: Vec<_> = variants.iter().map(|v| toml_string(v.effective_name())).collect();
    Some(format!("One of: {}.", names.join(", ")))
}

fn toml_key(key: &str) -> String {
    let bare = !key.is_empty()
        && key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
    if bare { key.to_string() } else { toml_string(key) }
}

fn toml_string(s: &str) -> String {
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
