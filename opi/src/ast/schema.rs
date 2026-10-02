use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct Schema {
    pub kind: SchemaKind,
    #[serde(flatten)]
    pub meta: Meta,
}

impl Schema {
    pub fn new(kind: SchemaKind) -> Self {
        Self {
            kind,
            meta: Meta::default(),
        }
    }
    pub fn unknown() -> Self {
        Self::new(SchemaKind::Unknown)
    }
    pub fn never() -> Self {
        Self::new(SchemaKind::Never)
    }
    pub fn with_meta(mut self, meta: Meta) -> Self {
        self.meta = meta;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SchemaKind {
    /// `{}` / missing schema / anything we can't (yet) lower.
    Unknown,
    /// `false` / `not: {}`.
    Never,
    String(Option<String>),
    Number(Option<String>),
    Integer(Option<String>),
    Boolean,
    Null,
    Enum(Vec<Literal>),
    Array(Box<Schema>),
    Object(Object),
    /// `oneOf` / `anyOf`.
    Union(Vec<Schema>),
    /// `allOf`.
    Intersection(Vec<Schema>),
    /// Reference to a named schema in [`super::Api::schemas`], by its raw key.
    Ref(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]

/// `additionalProperties` -> `additional`. `None` means "not specified / closed".
pub struct Object {
    pub properties: IndexMap<String, Property>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional: Option<Box<Schema>>,
}

/// A property: its schema's fields with `required` beside them.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Property {
    #[serde(flatten)]
    pub schema: Schema,
    pub required: bool,
}

/// A `const` / `enum` value, serialised as itself.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Literal {
    String(String),
    Number(f64),
    Bool(bool),
    Null,
}

/// Annotations that don't change the shape of a type but that emitters may
/// want (doc comments, `readonly`, `| null`, ...).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Meta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub deprecated: bool,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub nullable: bool,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub read_only: bool,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub write_only: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encoded: Option<String>,
}

impl Meta {
    /// A wrapping schema's annotations laid over these: flags OR together and
    /// its docs win when set; `encoded` is untouched.
    pub fn overlay(&mut self, outer: Meta) {
        self.nullable |= outer.nullable;
        self.deprecated |= outer.deprecated;
        self.read_only |= outer.read_only;
        self.write_only |= outer.write_only;
        if outer.description.is_some() {
            self.description = outer.description;
        }
        if outer.title.is_some() {
            self.title = outer.title;
        }
    }
}
