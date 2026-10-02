use std::borrow::Cow;
use std::convert::Infallible;
use std::fmt;
use std::str::FromStr;

use anyhow::bail;
use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::ast::Schema;

#[derive(Debug)]
pub struct Route {
    pub path: String,
    pub method: Method,
    pub summary: Option<String>,
    pub description: Option<String>,
    pub deprecated: Option<bool>,
    /// Path-level and operation-level parameters, merged.
    pub params: Params,
    /// Whether a body must be sent. When it needn't be (or there is no body
    /// at all), `{request}` also has a variant without one.
    pub body_required: bool,
    /// One per request body media type.
    pub request: Vec<Request>,
    /// One per status and media type, in spec order. A status without
    /// content (e.g. 204) is one response with no `content_type`.
    pub response: Vec<Response>,
}

impl Route {
    pub fn path(&self) -> &str {
        &self.path
    }
    pub fn method(&self) -> &Method {
        &self.method
    }
    pub fn requests(&self) -> impl Iterator<Item = &Request> {
        self.request.iter()
    }
    pub fn responses(&self) -> impl Iterator<Item = &Response> {
        self.response.iter()
    }
    pub fn deprecated(&self) -> bool {
        self.deprecated.unwrap_or(false)
    }
}

/// A route's parameters by where they go.
#[derive(Debug, Default)]
pub struct Params {
    pub path: Vec<Param>,
    pub query: Vec<Param>,
    /// OpenAPI 3.2 `in: querystring`: the whole query string as one value.
    /// Exclusive with `query`.
    pub querystring: Option<Param>,
    pub header: Vec<Param>,
    pub cookie: Vec<Param>,
}

#[derive(Debug)]
pub struct Param {
    pub name: String,
    /// Always true for a path parameter.
    pub required: bool,
    pub description: Option<String>,
    pub deprecated: bool,
    pub schema: Schema,
}

#[derive(Debug)]
pub struct Request {
    pub content_type: String,
    pub deprecated: Option<bool>,
    /// `None`: the media type has no schema (`unknown`).
    pub body: Option<Schema>,
    /// A sequential media type with OpenAPI 3.2's `itemSchema`: `body` is
    /// the type of each item, not of the whole body.
    pub stream: bool,
    pub description: Option<String>,
}

impl Request {
    pub fn deprecated(&self) -> bool {
        self.deprecated.unwrap_or(false)
    }
    pub fn content_type(&self) -> &str {
        &self.content_type
    }
}

#[derive(Debug)]
pub struct Response {
    pub status: Status,
    /// `None` for a response without content.
    pub content_type: Option<String>,
    /// `None` without content, or when the media type has no schema.
    pub schema: Option<Schema>,
    /// As for [`Request::stream`].
    pub stream: bool,
    pub description: Option<String>,
    pub deprecated: Option<bool>,
}

impl Response {
    pub fn status(&self) -> Status {
        self.status
    }
    pub fn content_type(&self) -> Option<&str> {
        self.content_type.as_deref()
    }
    pub fn deprecated(&self) -> bool {
        self.deprecated.unwrap_or(false)
    }
}

/// A response status as written in a spec: `200`, `2XX` or `default`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Status {
    Code(u16),
    /// `2XX`, stored as the leading digit.
    Range(u16),
    Default,
}

/// `200`, `2xx` / `2XX`, or `default` (case-insensitive).
impl FromStr for Status {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> anyhow::Result<Self> {
        if s.eq_ignore_ascii_case("default") {
            return Ok(Status::Default);
        }
        if let [class @ b'1'..=b'5', x, y] = s.as_bytes()
            && x.eq_ignore_ascii_case(&b'X')
            && y.eq_ignore_ascii_case(&b'X')
        {
            return Ok(Status::Range(u16::from(class - b'0')));
        }
        if let Ok(code) = s.parse() {
            return Ok(Status::Code(code));
        }
        bail!("unknown status {s:?} (expected e.g. 200, 2XX or default)")
    }
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

/// An HTTP method, lowercase. Written in rules case-insensitively.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Method {
    Get,
    Put,
    Post,
    Delete,
    Options,
    Head,
    Patch,
    Trace,
    Query,
    /// OpenAPI 3.2 `additionalOperations` (e.g. `purge`).
    Other(String),
}

impl Method {
    /// The lowercase name.
    pub fn as_str(&self) -> &str {
        match self {
            Method::Get => "get",
            Method::Put => "put",
            Method::Post => "post",
            Method::Delete => "delete",
            Method::Options => "options",
            Method::Head => "head",
            Method::Patch => "patch",
            Method::Trace => "trace",
            Method::Query => "query",
            Method::Other(name) => name,
        }
    }
}

impl From<&str> for Method {
    fn from(s: &str) -> Self {
        match s.to_ascii_lowercase().as_str() {
            "get" => Method::Get,
            "put" => Method::Put,
            "post" => Method::Post,
            "delete" => Method::Delete,
            "options" => Method::Options,
            "head" => Method::Head,
            "patch" => Method::Patch,
            "trace" => Method::Trace,
            "query" => Method::Query,
            other => Method::Other(other.to_string()),
        }
    }
}

impl FromStr for Method {
    type Err = Infallible;
    fn from_str(s: &str) -> Result<Self, Infallible> {
        Ok(s.into())
    }
}

impl Serialize for Method {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for Method {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(String::deserialize(deserializer)?.as_str().into())
    }
}

impl JsonSchema for Method {
    fn schema_name() -> Cow<'static, str> {
        "Method".into()
    }

    fn json_schema(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
        schemars::json_schema!({
            "type": "string",
            "description": "An HTTP method, case-insensitive: get, put, post, delete, options, head, patch, trace, query, or any other a spec uses."
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn statuses() {
        let parse = |s: &str| s.parse::<Status>().unwrap();
        assert_eq!(parse("200"), Status::Code(200));
        assert_eq!(parse("2xX"), Status::Range(2));
        assert_eq!(parse("DEFAULT"), Status::Default);
        for s in ["200", "2XX", "default"] {
            assert_eq!(parse(s).to_string(), s);
        }
        for s in ["2xxx", "02xx", "6xx", "abc", ""] {
            assert!(s.parse::<Status>().is_err(), "{s:?}");
        }
    }

    #[test]
    fn methods() {
        assert_eq!(Method::from("GET"), Method::Get);
        assert_eq!(Method::from("PURGE"), Method::Other("purge".into()));
        assert_eq!(Method::from("PURGE").as_str(), "purge");
    }
}
