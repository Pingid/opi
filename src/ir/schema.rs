use indexmap::IndexMap;
use serde::Serialize;

/// A type in the intermediate representation.
///
/// This is deliberately closer to "what a TS type can express" than to JSON
/// Schema: validation-only keywords (`minLength`, `pattern`, ...) are dropped
/// during lowering, and nullability / enums are normalised so emitters don't
/// have to re-derive them.
///
/// Serialised (for `--emit-ir`) as the [`SchemaKind`] under `kind` with the
/// [`Meta`] fields beside it: `{ "kind": { "type": "string" }, "nullable": true }`.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "facet", derive(facet::Facet))]
pub struct Schema {
    pub kind: SchemaKind,
    #[serde(flatten)]
    #[cfg_attr(feature = "facet", facet(flatten))]
    pub meta: Meta,
}

/// Tagged by `type`: `{ "type": "array", "items": {…} }`.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
#[cfg_attr(
    feature = "facet",
    derive(facet::Facet),
    facet(tag = "type", rename_all = "snake_case")
)]
#[repr(u8)]
pub enum SchemaKind {
    /// `{}` / missing schema / anything we can't (yet) lower.
    Unknown,
    /// `false` / `not: {}`.
    Never,
    String {
        #[serde(skip_serializing_if = "Option::is_none")]
        format: Option<String>,
    },
    Number {
        #[serde(skip_serializing_if = "Option::is_none")]
        format: Option<String>,
    },
    Integer {
        #[serde(skip_serializing_if = "Option::is_none")]
        format: Option<String>,
    },
    Boolean,
    Null,
    /// `enum: [...]` / `const` (a single-member enum).
    Enum {
        values: Vec<Literal>,
    },
    Array {
        #[cfg_attr(feature = "facet", facet(recursive_type))]
        items: Box<Schema>,
    },
    Object(Object),
    /// `oneOf` / `anyOf`.
    Union {
        #[cfg_attr(feature = "facet", facet(recursive_type))]
        members: Vec<Schema>,
    },
    /// `allOf`.
    Intersection {
        #[cfg_attr(feature = "facet", facet(recursive_type))]
        members: Vec<Schema>,
    },
    /// Reference to a named schema in [`super::Api::schemas`], by its raw key.
    Ref {
        name: String,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "facet", derive(facet::Facet))]
pub struct Object {
    #[cfg_attr(feature = "facet", facet(recursive_type))]
    pub properties: IndexMap<String, Property>,
    /// `additionalProperties`. `None` means "not specified / closed".
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "facet", facet(recursive_type))]
    pub additional: Option<Box<Schema>>,
}

/// A property: its schema's fields with `required` beside them.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "facet", derive(facet::Facet))]
pub struct Property {
    #[serde(flatten)]
    #[cfg_attr(feature = "facet", facet(flatten, recursive_type))]
    pub schema: Schema,
    pub required: bool,
}

/// A `const` / `enum` value, serialised as itself.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
#[cfg_attr(feature = "facet", derive(facet::Facet), facet(untagged))]
#[repr(u8)]
pub enum Literal {
    String(String),
    Number(f64),
    Bool(bool),
    Null,
}

/// Annotations that don't change the shape of a type but that emitters may
/// want (doc comments, `readonly`, `| null`, ...).
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[cfg_attr(feature = "facet", derive(facet::Facet))]
pub struct Meta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "facet", facet(default))]
    pub deprecated: bool,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "facet", facet(default))]
    pub nullable: bool,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "facet", facet(default))]
    pub read_only: bool,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "facet", facet(default))]
    pub write_only: bool,
    /// The value arrives as a string of this media type (`contentMediaType`,
    /// e.g. `application/json`), and the schema describes it decoded.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encoded: Option<String>,
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

    pub fn with_meta(mut self, meta: Meta) -> Self {
        self.meta = meta;
        self
    }

    /// Direct sub-schemas: array items, properties and `additionalProperties`,
    /// union / intersection members.
    pub fn children(&self) -> Box<dyn Iterator<Item = &Schema> + '_> {
        match &self.kind {
            SchemaKind::Array { items } => Box::new(std::iter::once(&**items)),
            SchemaKind::Object(object) => {
                let properties = object.properties.values().map(|p| &p.schema);
                Box::new(properties.chain(object.additional.as_deref()))
            }
            SchemaKind::Union { members } | SchemaKind::Intersection { members } => {
                Box::new(members.iter())
            }
            _ => Box::new(std::iter::empty()),
        }
    }
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
