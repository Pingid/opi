//! Serde model of an OpenAPI 3.0 / 3.1 / 3.2 document: only the parts
//! lowering reads, and deliberately loose.
//!
//! - Referable objects carry an optional `$ref` rather than being an untagged
//!   `Ref | Item` enum. Untagged enums buffer every value and discard the real
//!   error, which is slow on big specs and unhelpful on broken ones.
//! - Keyword values are kept as written (`format` is any string, `enum` /
//!   `const` are raw JSON, `type` is one or many) and interpreted in lowering.
//!   That's also where the 3.0 vs 3.1+ differences (`nullable` vs
//!   `type: [.., "null"]`) are handled, so no version-specific parsing.
//! - Unknown fields are ignored; maps keep spec order.

use std::fmt;

use indexmap::IndexMap;
use serde::de::{self, IgnoredAny, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer};
use serde_json::Value;

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct Document {
    pub openapi: Option<String>,
    /// Only read to reject Swagger 2.0 documents with a clear error.
    pub swagger: Option<String>,
    #[serde(deserialize_with = "without_extensions")]
    pub paths: IndexMap<String, PathItem>,
    pub components: Components,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Components {
    pub schemas: IndexMap<String, SchemaOrBool>,
    pub parameters: IndexMap<String, Parameter>,
    pub request_bodies: IndexMap<String, RequestBody>,
    pub responses: IndexMap<String, Response>,
    /// 3.1+.
    pub path_items: IndexMap<String, PathItem>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct PathItem {
    /// 3.0: external only. 3.1+: also `#/components/pathItems/*`.
    #[serde(rename = "$ref")]
    pub reference: Option<String>,
    pub get: Option<Operation>,
    pub put: Option<Operation>,
    pub post: Option<Operation>,
    pub delete: Option<Operation>,
    pub options: Option<Operation>,
    pub head: Option<Operation>,
    pub patch: Option<Operation>,
    pub trace: Option<Operation>,
    /// 3.2.
    pub query: Option<Operation>,
    /// 3.2: other methods, keyed by the method name as sent (e.g. `LINK`).
    pub additional_operations: IndexMap<String, Operation>,
    pub parameters: Vec<Parameter>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Operation {
    pub operation_id: Option<String>,
    pub summary: Option<String>,
    pub description: Option<String>,
    pub deprecated: bool,
    pub tags: Vec<String>,
    pub parameters: Vec<Parameter>,
    pub request_body: Option<RequestBody>,
    /// Status (`200`, `2XX`, `default`) -> response. Required before 3.1.
    #[serde(deserialize_with = "without_extensions")]
    pub responses: IndexMap<String, Response>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct Parameter {
    #[serde(rename = "$ref")]
    pub reference: Option<String>,
    pub name: String,
    #[serde(rename = "in")]
    pub location: Option<Location>,
    pub required: bool,
    pub deprecated: bool,
    pub description: Option<String>,
    pub schema: Option<SchemaOrBool>,
    /// Instead of `schema`: exactly one media type. Required for `querystring`.
    pub content: IndexMap<String, MediaType>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Location {
    Path,
    Query,
    /// 3.2: the whole query string as one value (described by `content`).
    Querystring,
    Header,
    Cookie,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct RequestBody {
    #[serde(rename = "$ref")]
    pub reference: Option<String>,
    pub description: Option<String>,
    pub required: bool,
    pub content: IndexMap<String, MediaType>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct Response {
    #[serde(rename = "$ref")]
    pub reference: Option<String>,
    /// Required before 3.2.
    pub description: Option<String>,
    pub content: IndexMap<String, MediaType>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct MediaType {
    pub schema: Option<SchemaOrBool>,
    /// 3.2, for sequential media types (`text/event-stream`,
    /// `application/jsonl`, ...): the schema of each item.
    pub item_schema: Option<SchemaOrBool>,
}

/// A schema, or JSON Schema 2020-12's bare `true` (anything) / `false`
/// (nothing).
#[derive(Debug)]
pub enum SchemaOrBool {
    Bool(bool),
    Schema(Box<Schema>),
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Schema {
    /// 3.1+ allows siblings (`description`, and even shape keywords).
    #[serde(rename = "$ref")]
    pub reference: Option<String>,
    /// 3.0: always one. 3.1+: one or many, `"null"` included.
    #[serde(rename = "type")]
    pub types: OneOrMany,
    pub format: Option<String>,
    /// 3.0 only; 3.1+ uses `type: [.., "null"]`.
    pub nullable: bool,
    #[serde(rename = "enum")]
    pub enum_values: Option<Vec<Value>>,
    /// 3.1+. `Some(Null)` for `const: null`, unlike a plain `Option<Value>`.
    #[serde(rename = "const", deserialize_with = "present")]
    pub const_value: Option<Value>,
    pub properties: IndexMap<String, SchemaOrBool>,
    #[serde(deserialize_with = "names_or_nothing")]
    pub required: Vec<String>,
    pub additional_properties: Option<SchemaOrBool>,
    pub items: Option<SchemaOrBool>,
    pub all_of: Vec<SchemaOrBool>,
    pub any_of: Vec<SchemaOrBool>,
    pub one_of: Vec<SchemaOrBool>,
    pub not: Option<SchemaOrBool>,
    /// 3.1+, on strings: the media type the string holds, e.g. JSON in a
    /// server-sent event's `data`.
    pub content_media_type: Option<String>,
    /// 3.1+, on strings: e.g. `base64`.
    pub content_encoding: Option<String>,
    /// 3.1+: the schema of the decoded content.
    pub content_schema: Option<SchemaOrBool>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub deprecated: bool,
    pub read_only: bool,
    pub write_only: bool,
}

/// `"string"` or `["string", "null"]`.
#[derive(Debug, Default)]
pub struct OneOrMany(pub Vec<String>);

// ---------------------------------------------------------------------------
// Deserialize impls
// ---------------------------------------------------------------------------

impl<'de> Deserialize<'de> for SchemaOrBool {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct V;
        impl<'de> Visitor<'de> for V {
            type Value = SchemaOrBool;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a schema object or boolean")
            }

            fn visit_bool<E: de::Error>(self, v: bool) -> Result<Self::Value, E> {
                Ok(SchemaOrBool::Bool(v))
            }

            fn visit_map<A: MapAccess<'de>>(self, map: A) -> Result<Self::Value, A::Error> {
                let schema = Schema::deserialize(de::value::MapAccessDeserializer::new(map))?;
                Ok(SchemaOrBool::Schema(Box::new(schema)))
            }

            /// Draft-4 tuple `items: [A, B]`. No IR equivalent; accept as
            /// "anything" rather than failing the whole document.
            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                while seq.next_element::<IgnoredAny>()?.is_some() {}
                Ok(SchemaOrBool::Bool(true))
            }
        }
        deserializer.deserialize_any(V)
    }
}

impl<'de> Deserialize<'de> for OneOrMany {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct V;
        impl<'de> Visitor<'de> for V {
            type Value = OneOrMany;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a type name or list of type names")
            }

            fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
                Ok(OneOrMany(vec![v.to_string()]))
            }

            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                let mut types = Vec::new();
                while let Some(t) = seq.next_element()? {
                    types.push(t);
                }
                Ok(OneOrMany(types))
            }
        }
        deserializer.deserialize_any(V)
    }
}

/// A present key's value, `null` included.
fn present<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Option<Value>, D::Error> {
    Value::deserialize(deserializer).map(Some)
}

/// `required: [..]` on an object schema. Some specs also put the 2.0-style
/// `required: true` on a property schema; that's ignored, not an error.
fn names_or_nothing<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<String>, D::Error> {
    Ok(match Value::deserialize(deserializer)? {
        Value::Array(names) => names
            .into_iter()
            .filter_map(|n| n.as_str().map(str::to_string))
            .collect(),
        _ => Vec::new(),
    })
}

/// A map minus its `x-*` extension keys, whose values can be anything.
/// Keys may be integers (YAML `200:` in `responses`).
fn without_extensions<'de, D, T>(deserializer: D) -> Result<IndexMap<String, T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    struct V<T>(std::marker::PhantomData<T>);
    impl<'de, T: Deserialize<'de>> Visitor<'de> for V<T> {
        type Value = IndexMap<String, T>;

        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
            f.write_str("a map")
        }

        fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
            let mut out = IndexMap::new();
            while let Some(Key(key)) = map.next_key()? {
                if key.starts_with("x-") {
                    map.next_value::<IgnoredAny>()?;
                } else {
                    let value = map.next_value()?;
                    out.insert(key, value);
                }
            }
            Ok(out)
        }
    }
    deserializer.deserialize_map(V(std::marker::PhantomData))
}

/// A map key written as a string or an integer.
struct Key(String);

impl<'de> Deserialize<'de> for Key {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct V;
        impl Visitor<'_> for V {
            type Value = Key;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a string or integer key")
            }

            fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
                Ok(Key(v.to_string()))
            }

            fn visit_u64<E: de::Error>(self, v: u64) -> Result<Self::Value, E> {
                Ok(Key(v.to_string()))
            }

            fn visit_i64<E: de::Error>(self, v: i64) -> Result<Self::Value, E> {
                Ok(Key(v.to_string()))
            }
        }
        deserializer.deserialize_any(V)
    }
}
