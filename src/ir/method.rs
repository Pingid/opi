use std::fmt;

use serde::{Serialize, Serializer};

/// Serialised (and reflected) as sent: `"GET"`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "facet", derive(facet::Facet), facet(proxy = String))]
#[repr(u8)]
pub enum Method {
    Get,
    Put,
    Post,
    Delete,
    Options,
    Head,
    Patch,
    Trace,
    /// OpenAPI 3.2.
    Query,
    /// OpenAPI 3.2 `additionalOperations`: any other method, as sent (e.g.
    /// `LINK`).
    Other(String),
}

impl Method {
    /// The standard methods, in the order operations are emitted.
    pub const STANDARD: [Method; 9] = [
        Method::Get,
        Method::Put,
        Method::Post,
        Method::Delete,
        Method::Options,
        Method::Head,
        Method::Patch,
        Method::Trace,
        Method::Query,
    ];

    /// Only the standard methods, any case; see [`Method::Other`] for the rest.
    pub fn parse(s: &str) -> Option<Self> {
        Self::STANDARD
            .iter()
            .find(|m| m.as_str().eq_ignore_ascii_case(s))
            .cloned()
    }

    pub fn as_str(&self) -> &str {
        match self {
            Self::Get => "GET",
            Self::Put => "PUT",
            Self::Post => "POST",
            Self::Delete => "DELETE",
            Self::Options => "OPTIONS",
            Self::Head => "HEAD",
            Self::Patch => "PATCH",
            Self::Trace => "TRACE",
            Self::Query => "QUERY",
            Self::Other(method) => method,
        }
    }
}

impl fmt::Display for Method {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Serialize for Method {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

/// Conversions for facet's `proxy = String`: anything non-standard is
/// [`Method::Other`].
impl From<String> for Method {
    fn from(s: String) -> Self {
        Method::parse(&s).unwrap_or(Method::Other(s))
    }
}

impl From<&Method> for String {
    fn from(method: &Method) -> Self {
        method.as_str().to_string()
    }
}
