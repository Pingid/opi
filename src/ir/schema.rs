use indexmap::IndexMap;

/// A type in the intermediate representation.
///
/// This is deliberately closer to "what a TS type can express" than to JSON
/// Schema: validation-only keywords (`minLength`, `pattern`, ...) are dropped
/// during lowering, and nullability / enums are normalised so emitters don't
/// have to re-derive them.
#[derive(Debug, Clone, PartialEq)]
pub struct Schema {
    pub kind: SchemaKind,
    pub meta: Meta,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SchemaKind {
    /// `{}` / missing schema / anything we can't (yet) lower.
    Unknown,
    /// `false` / `not: {}`.
    Never,
    String { format: Option<String> },
    Number { format: Option<String> },
    Integer { format: Option<String> },
    Boolean,
    Null,
    /// `enum: [...]` / `const` (a single-member enum).
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

#[derive(Debug, Clone, PartialEq)]
pub struct Object {
    pub properties: IndexMap<String, Property>,
    /// `additionalProperties`. `None` means "not specified / closed".
    pub additional: Option<Box<Schema>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Property {
    pub schema: Schema,
    pub required: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    String(String),
    Number(f64),
    Bool(bool),
    Null,
}

/// Annotations that don't change the shape of a type but that emitters may
/// want (doc comments, `readonly`, `| null`, ...).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Meta {
    pub title: Option<String>,
    pub description: Option<String>,
    pub deprecated: bool,
    pub nullable: bool,
    pub read_only: bool,
    pub write_only: bool,
    /// The value arrives as a string of this media type (`contentMediaType`,
    /// e.g. `application/json`), and the schema describes it decoded.
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
}
