//! Dependency-free so `build.rs` can include it along with the config.

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
    /// OpenAPI 3.2.
    Query,
    /// OpenAPI 3.2 `additionalOperations`: any other method, as sent (e.g.
    /// `LINK`).
    Other(String),
}

impl Method {
    /// Only the standard methods; see [`Method::Other`] for the rest.
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s.to_ascii_lowercase().as_str() {
            "get" => Self::Get,
            "put" => Self::Put,
            "post" => Self::Post,
            "delete" => Self::Delete,
            "options" => Self::Options,
            "head" => Self::Head,
            "patch" => Self::Patch,
            "trace" => Self::Trace,
            "query" => Self::Query,
            _ => return None,
        })
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
