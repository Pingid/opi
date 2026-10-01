//! Intermediate representation of an API.
//!
//! Frontends (currently only [`crate::openapi`]) lower a spec into an [`Api`];
//! backends (currently only [`crate::ts`]) turn an [`Api`] into code. The IR
//! is fully resolved: parameters, request bodies and responses referenced via
//! `$ref` are inlined, while named schemas stay as [`SchemaKind::Ref`] so
//! backends can decide whether to emit them as named types.
//!
//! The types are generic over the schema representation `S`: the backend
//! reads `Api<Schema>`, and `--emit-ir` writes the same operations as
//! [`Operation<String>`], each schema rendered to TS (see [`Operation::map`]).
//! That export is the IR's serde form; nothing else serialises it.

mod map;
mod method;
mod reach;
mod schema;
mod status;

use indexmap::IndexMap;
pub use method::Method;
pub use schema::*;
use serde::Serialize;
pub use status::Status;

#[derive(Debug, Clone)]
pub struct Api<S = Schema> {
    /// Named schemas (`#/components/schemas/*`), keyed by their raw name.
    pub schemas: IndexMap<String, S>,
    pub operations: Vec<Operation<S>>,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "facet", derive(facet::Facet))]
pub struct Operation<S = Schema> {
    #[serde(rename = "operation_id", skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "facet", facet(rename = "operation_id"))]
    pub id: Option<String>,
    /// `GET`, `POST`, ...
    pub method: Method,
    /// Templated path, e.g. `/components/{name}`.
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub deprecated: bool,
    pub tags: Vec<String>,
    pub params: Params<S>,
    /// One per body media type, plus one without a `content_type` when the
    /// body is missing or optional. Never empty.
    pub requests: Vec<Request<S>>,
    /// One per status and media type, in spec order.
    pub responses: Vec<Response<S>>,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "facet", derive(facet::Facet))]
pub struct Params<S = Schema> {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "facet", facet(default))]
    pub path: Vec<Param<S>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "facet", facet(default))]
    pub query: Vec<Param<S>>,
    /// OpenAPI 3.2 `in: querystring`: the whole query string as one value.
    /// Exclusive with `query`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub querystring: Option<Param<S>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "facet", facet(default))]
    pub header: Vec<Param<S>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "facet", facet(default))]
    pub cookie: Vec<Param<S>>,
}

impl<S> Default for Params<S> {
    fn default() -> Self {
        Self {
            path: Vec::new(),
            query: Vec::new(),
            querystring: None,
            header: Vec::new(),
            cookie: Vec::new(),
        }
    }
}

/// Where a parameter goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Location {
    Path,
    Query,
    Querystring,
    Header,
    Cookie,
}

impl<S> Params<S> {
    pub fn push(&mut self, location: Location, param: Param<S>) {
        match location {
            Location::Path => self.path.push(param),
            Location::Query => self.query.push(param),
            Location::Querystring => self.querystring = Some(param),
            Location::Header => self.header.push(param),
            Location::Cookie => self.cookie.push(param),
        }
    }

    /// Every param: path, query, querystring, header, cookie.
    pub fn iter(&self) -> impl Iterator<Item = &Param<S>> {
        let query = self.query.iter().chain(&self.querystring);
        self.path
            .iter()
            .chain(query)
            .chain(&self.header)
            .chain(&self.cookie)
    }
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "facet", derive(facet::Facet))]
pub struct Param<S = Schema> {
    pub name: String,
    pub required: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub deprecated: bool,
    pub schema: S,
}

/// One request shape: a body of one media type, or (`content_type: None`) no
/// body, when there is none or it's optional.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "facet", derive(facet::Facet))]
pub struct Request<S = Schema> {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<S>,
    /// A sequence (`text/event-stream`, `application/jsonl`, ... with
    /// OpenAPI 3.2's `itemSchema`): `body` is the type of each item, not of
    /// the whole body.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "facet", facet(default))]
    pub stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// One status and media type pair, or (`content_type: None`) a bodiless
/// response such as 204.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "facet", derive(facet::Facet))]
pub struct Response<S = Schema> {
    pub status: Status,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<S>,
    /// A sequence, as for a request: `body` is the type of each item.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "facet", facet(default))]
    pub stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl<S> Request<S> {
    pub fn bodiless() -> Self {
        Self {
            content_type: None,
            body: None,
            stream: false,
            description: None,
        }
    }
}

impl<S> Response<S> {
    pub fn bodiless(status: Status, description: Option<String>) -> Self {
        Self {
            status,
            content_type: None,
            body: None,
            stream: false,
            description,
        }
    }
}
