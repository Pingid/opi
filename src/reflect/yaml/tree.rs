//! Step 1: facet's serialisation events, recorded as a tree of nodes with
//! docs attached.

use facet::{Def, Shape, StructKind, Type, UserType};
use facet_format::{FormatSerializer, ScalarValue};
use facet_reflect::{FieldItem, Peek};

use super::{YamlSerializeError, error, yaml_string};
use crate::reflect::doc::clean_doc;

#[derive(Debug)]
pub(super) enum Node {
    /// Already rendered as YAML (`"text"`, `true`, `3`, `null`).
    Scalar(String),
    Array(Vec<Node>),
    Table(Table),
}

#[derive(Debug, Default)]
pub(super) struct Table {
    /// Docs of the table's type (used for section headers without field docs).
    pub(super) doc: Vec<String>,
    pub(super) entries: Vec<Entry>,
}

#[derive(Debug)]
pub(super) struct Entry {
    pub(super) key: String,
    pub(super) doc: Vec<String>,
    pub(super) value: Node,
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
pub(super) struct TreeSerializer {
    stack: Vec<Frame>,
    pub(super) root: Option<Table>,
    /// Docs from `field_metadata_with_value`, waiting for `field_key`.
    field_doc: Vec<String>,
    /// Docs from `struct_metadata`, waiting for `begin_struct`.
    type_doc: Vec<String>,
}

impl TreeSerializer {
    fn push(&mut self, node: Node) -> Result<(), YamlSerializeError> {
        match self.stack.last_mut() {
            None => match node {
                Node::Table(table) => self.root = Some(table),
                _ => return Err(error("the root value must be a struct or map")),
            },
            Some(Frame::Table { table, key }) => {
                let (key, doc) = key.take().ok_or_else(|| error("value without a key"))?;
                table.entries.push(Entry {
                    key,
                    doc,
                    value: node,
                });
            }
            Some(Frame::Array(items)) => items.push(node),
        }
        Ok(())
    }

    fn begin_table(&mut self) {
        let doc = std::mem::take(&mut self.type_doc);
        self.stack.push(Frame::Table {
            table: Table {
                doc,
                entries: Vec::new(),
            },
            key: None,
        });
    }

    fn end_table(&mut self) -> Result<(), YamlSerializeError> {
        match self.stack.pop() {
            Some(Frame::Table { table, .. }) => self.push(Node::Table(table)),
            _ => Err(error("end of table without a matching start")),
        }
    }
}

impl FormatSerializer for TreeSerializer {
    type Error = YamlSerializeError;

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
        self.push(Node::Scalar("null".to_string()))
    }

    fn scalar(&mut self, scalar: ScalarValue<'_>) -> Result<(), Self::Error> {
        let rendered = match scalar {
            ScalarValue::Null | ScalarValue::Unit => "null".to_string(),
            ScalarValue::Bool(v) => v.to_string(),
            ScalarValue::Char(c) => yaml_string(&c.to_string()),
            ScalarValue::I64(v) => v.to_string(),
            ScalarValue::U64(v) => v.to_string(),
            ScalarValue::I128(v) => v.to_string(),
            ScalarValue::U128(v) => v.to_string(),
            ScalarValue::F64(v) if v.is_nan() => ".nan".to_string(),
            ScalarValue::F64(v) if v.is_infinite() => {
                if v > 0.0 { ".inf" } else { "-.inf" }.to_string()
            }
            // Keep floats recognisably floats (`1.0`, not `1`).
            ScalarValue::F64(v) if v.fract() == 0.0 => format!("{v:.1}"),
            ScalarValue::F64(v) => v.to_string(),
            ScalarValue::Str(s) => yaml_string(&s),
            ScalarValue::Bytes(_) => return Err(error("YAML has no byte strings")),
        };
        self.push(Node::Scalar(rendered))
    }
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
    let names: Vec<_> = variants
        .iter()
        .map(|v| yaml_string(v.effective_name()))
        .collect();
    Some(format!("One of: {}.", names.join(", ")))
}
