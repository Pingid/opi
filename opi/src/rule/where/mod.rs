use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ast;
use crate::rule::ViewRule;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, JsonSchema)]
pub struct Where {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deprecated: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request: Option<WhereRequest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response: Option<WhereResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<ast::Method>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

impl Where {
    pub fn deprecated(mut self, deprecated: bool) -> Self {
        self.deprecated = Some(deprecated);
        self
    }
    pub fn request(mut self, request: WhereRequest) -> Self {
        self.request = Some(request);
        self
    }
    pub fn response(mut self, response: WhereResponse) -> Self {
        self.response = Some(response);
        self
    }

    /// The route-level keys; unset keys match anything.
    fn matches_route(&self, r: &ast::Route) -> bool {
        Wildcard::<bool>::check_compatible(self.deprecated, r.deprecated())
            && Wildcard::<&ast::Method>::check_compatible(self.method.as_ref(), r.method())
            && Wildcard::<&str>::check_compatible(self.path.as_deref(), r.path())
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, JsonSchema)]
pub struct WhereRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deprecated: Option<bool>,
}

impl WhereRequest {
    pub fn matches(&self, q: &ast::Request) -> bool {
        Wildcard::<bool>::check_compatible(self.deprecated, q.deprecated())
            && Wildcard::<&str>::check_compatible(self.content_type.as_deref(), q.content_type())
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, JsonSchema)]
pub struct WhereResponse {
    /// A status code; matches only that code, not a `2XX` range.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deprecated: Option<bool>,
}

impl WhereResponse {
    pub fn matches(&self, s: &ast::Response) -> bool {
        self.status
            .is_none_or(|code| s.status() == ast::Status::Code(code))
            && Wildcard::<bool>::check_compatible(self.deprecated, s.deprecated())
            && self
                .content_type
                .as_deref()
                .is_none_or(|ct| s.content_type() == Some(ct))
    }
}

/// On routes, `request` / `response` mean "has at least one that matches".
/// On requests / responses, the route-level keys test the parent route.
impl<'a> ViewRule<'a> for Where {
    fn retain_route(&mut self, r: &&'a ast::Route) -> bool {
        self.matches_route(r)
            && self
                .request
                .as_ref()
                .is_none_or(|w| r.requests().any(|q| w.matches(q)))
            && self
                .response
                .as_ref()
                .is_none_or(|w| r.responses().any(|s| w.matches(s)))
    }
    fn retain_request(&mut self, (q, r): &(&'a ast::Request, &'a ast::Route)) -> bool {
        self.matches_route(r) && self.request.as_ref().is_none_or(|w| w.matches(q))
    }
    fn retain_response(&mut self, (s, r): &(&'a ast::Response, &'a ast::Route)) -> bool {
        self.matches_route(r) && self.response.as_ref().is_none_or(|w| w.matches(s))
    }
    fn retain_schema(&mut self, c: &&'a ast::Component) -> bool {
        Wildcard::<bool>::check_compatible(self.deprecated, c.schema.meta.deprecated)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Wildcard<T> {
    Value(T),
    #[default]
    Any,
}

impl<T> From<Option<T>> for Wildcard<T> {
    fn from(opt: Option<T>) -> Self {
        opt.map_or(Wildcard::Any, Wildcard::Value)
    }
}

impl<T> From<T> for Wildcard<T> {
    fn from(v: T) -> Self {
        Wildcard::Value(v)
    }
}

impl<T: PartialEq> Wildcard<T> {
    pub fn check_compatible(a: impl Into<Wildcard<T>>, b: impl Into<Wildcard<T>>) -> bool {
        a.into().is_compatible_with(b)
    }
    /// `Any` on either side matches; two values must be equal.
    pub fn is_compatible_with(&self, other: impl Into<Wildcard<T>>) -> bool {
        match (self, &other.into()) {
            (Wildcard::Value(a), Wildcard::Value(b)) => a == b,
            _ => true,
        }
    }
}
