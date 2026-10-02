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

mod de;

use indexmap::IndexMap;
use serde::Deserialize;
use serde_json::Value;

use crate::util::List;

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct Document {
    pub openapi: Option<String>,
    /// Only read to reject Swagger 2.0 documents with a clear error.
    pub swagger: Option<String>,
    #[serde(deserialize_with = "de::without_extensions")]
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

impl PathItem {
    pub fn operations(&self) -> impl Iterator<Item = (String, &Operation)> {
        self.get
            .iter()
            .map(move |v| ("get".to_string(), v))
            .chain(self.put.iter().map(move |v| ("put".to_string(), v)))
            .chain(self.post.iter().map(move |v| ("post".to_string(), v)))
            .chain(self.delete.iter().map(move |v| ("delete".to_string(), v)))
            .chain(self.options.iter().map(move |v| ("options".to_string(), v)))
            .chain(self.head.iter().map(move |v| ("head".to_string(), v)))
            .chain(self.patch.iter().map(move |v| ("patch".to_string(), v)))
            .chain(self.trace.iter().map(move |v| ("trace".to_string(), v)))
            .chain(self.query.iter().map(move |v| ("query".to_string(), v)))
            .chain(
                self.additional_operations
                    .iter()
                    .map(move |(k, v)| (k.to_string(), v)),
            )
    }
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
    #[serde(deserialize_with = "de::without_extensions")]
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
    pub types: List<String>,
    pub format: Option<String>,
    /// 3.0 only; 3.1+ uses `type: [.., "null"]`.
    pub nullable: bool,
    #[serde(rename = "enum")]
    pub enum_values: Option<Vec<Value>>,
    /// 3.1+. `Some(Null)` for `const: null`, unlike a plain `Option<Value>`.
    #[serde(rename = "const", deserialize_with = "de::present")]
    pub const_value: Option<Value>,
    pub properties: IndexMap<String, SchemaOrBool>,
    #[serde(deserialize_with = "de::names_or_nothing")]
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
