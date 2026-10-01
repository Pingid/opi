//! The variables a template can reference: an operation's fields, and its
//! request / response variants' with `.`.

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Var {
    TypeName,
    Id,
    Method,
    Path,
    Summary,
    Description,
    Deprecated,
    Tags,
    Request,
    RequestContentType,
    RequestBody,
    RequestParams,
    RequestQuery,
    RequestHeaders,
    RequestCookies,
    Response,
    ResponseStatus,
    ResponseContentType,
    ResponseBody,
}

impl Var {
    /// Every variable, in the order errors list them.
    pub const ALL: [Var; 19] = [
        Var::TypeName,
        Var::Id,
        Var::Method,
        Var::Path,
        Var::Summary,
        Var::Description,
        Var::Deprecated,
        Var::Tags,
        Var::Request,
        Var::RequestContentType,
        Var::RequestBody,
        Var::RequestParams,
        Var::RequestQuery,
        Var::RequestHeaders,
        Var::RequestCookies,
        Var::Response,
        Var::ResponseStatus,
        Var::ResponseContentType,
        Var::ResponseBody,
    ];

    /// Variables with a text value: for names, `shape` keys and literal
    /// values. Several can have more than one value (`tags`, the content
    /// types, `response.status`) or none (`id`, `summary`).
    pub const TEXT: &[Var] = &[
        Var::TypeName,
        Var::Id,
        Var::Method,
        Var::Path,
        Var::Summary,
        Var::Description,
        Var::Deprecated,
        Var::Tags,
        Var::RequestContentType,
        Var::ResponseStatus,
        Var::ResponseContentType,
    ];

    /// [`Var::TEXT`] for choosing a type name: not the name being chosen.
    pub const NAME: &[Var] = &[
        Var::Id,
        Var::Method,
        Var::Path,
        Var::Summary,
        Var::Description,
        Var::Deprecated,
        Var::Tags,
        Var::RequestContentType,
        Var::ResponseStatus,
        Var::ResponseContentType,
    ];

    /// Variables that, as a whole value, reference part of the operation's
    /// type.
    pub const REF: &[Var] = &[
        Var::TypeName,
        Var::Method,
        Var::Path,
        Var::Request,
        Var::RequestContentType,
        Var::RequestBody,
        Var::RequestParams,
        Var::RequestQuery,
        Var::RequestHeaders,
        Var::RequestCookies,
        Var::Response,
        Var::ResponseStatus,
        Var::ResponseContentType,
        Var::ResponseBody,
    ];

    /// As written in a template: `request.content_type`.
    pub const fn name(self) -> &'static str {
        match self {
            Var::TypeName => "type_name",
            Var::Id => "id",
            Var::Method => "method",
            Var::Path => "path",
            Var::Summary => "summary",
            Var::Description => "description",
            Var::Deprecated => "deprecated",
            Var::Tags => "tags",
            Var::Request => "request",
            Var::RequestContentType => "request.content_type",
            Var::RequestBody => "request.body",
            Var::RequestParams => "request.params",
            Var::RequestQuery => "request.query",
            Var::RequestHeaders => "request.headers",
            Var::RequestCookies => "request.cookies",
            Var::Response => "response",
            Var::ResponseStatus => "response.status",
            Var::ResponseContentType => "response.content_type",
            Var::ResponseBody => "response.body",
        }
    }

    /// By name, any case (`{REQUEST.CONTENT_TYPE}` is `request.content_type`).
    pub fn parse(s: &str) -> Option<Var> {
        Self::ALL
            .into_iter()
            .find(|v| v.name().eq_ignore_ascii_case(s))
    }
}

impl fmt::Display for Var {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}
