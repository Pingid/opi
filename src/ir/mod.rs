//! Intermediate representation of an API.
//!
//! Frontends (currently only [`crate::openapi`]) lower a spec into an [`Api`];
//! backends (currently only [`crate::ts`]) turn an [`Api`] into code. The IR
//! is fully resolved: parameters, request bodies and responses referenced via
//! `$ref` are inlined, while named schemas stay as [`SchemaKind::Ref`] so
//! backends can decide whether to emit them as named types.

mod matches;
mod method;
mod reach;
mod schema;

use std::fmt;

use indexmap::IndexMap;

pub use method::Method;
pub use schema::*;

#[derive(Debug, Clone, Default)]
pub struct Api {
    /// Named schemas (`#/components/schemas/*`), keyed by their raw name.
    pub schemas: IndexMap<String, Schema>,
    pub operations: Vec<Operation>,
}

#[derive(Debug, Clone)]
pub struct Operation {
    pub id: Option<String>,
    pub method: Method,
    /// Templated path, e.g. `/components/{name}`.
    pub path: String,
    pub summary: Option<String>,
    pub description: Option<String>,
    pub deprecated: bool,
    pub tags: Vec<String>,
    pub params: Params,
    pub body: Option<RequestBody>,
    pub responses: Vec<Response>,
}

#[derive(Debug, Clone, Default)]
pub struct Params {
    pub path: Vec<Param>,
    pub query: Vec<Param>,
    /// OpenAPI 3.2 `in: querystring`: the whole query string as one value.
    /// Exclusive with `query`.
    pub querystring: Option<Param>,
    pub header: Vec<Param>,
    pub cookie: Vec<Param>,
}

#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub required: bool,
    pub description: Option<String>,
    pub deprecated: bool,
    pub schema: Schema,
}

#[derive(Debug, Clone)]
pub struct RequestBody {
    pub required: bool,
    pub description: Option<String>,
    /// Media type (`application/json`, ...) -> body.
    pub content: IndexMap<String, Content>,
}

#[derive(Debug, Clone)]
pub struct Response {
    pub status: Status,
    pub description: Option<String>,
    /// Media type -> body. Empty for bodiless responses (e.g. 204).
    pub content: IndexMap<String, Content>,
}

/// One media type's body.
#[derive(Debug, Clone)]
pub struct Content {
    pub schema: Schema,
    /// A sequence (`text/event-stream`, `application/jsonl`, ... with
    /// OpenAPI 3.2's `itemSchema`): `schema` is the type of each item, not of
    /// the whole body.
    pub stream: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Status {
    /// `200`
    Code(u16),
    /// `2XX`, stored as the leading digit.
    Range(u16),
    /// `default`
    Default,
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Status::Code(c) => write!(f, "{c}"),
            Status::Range(r) => write!(f, "{r}XX"),
            Status::Default => f.write_str("default"),
        }
    }
}

