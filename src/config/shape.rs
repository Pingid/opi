//! `shape`: the layout of an `emit` type, mirroring the output. A map is an
//! object type, anything else a value (and a bare value at the root is a
//! union over the operations).
//!
//! ```yaml
//! shape: { "{METHOD} {path}": "{type_name}" }   # object type
//! shape: "{type_name}"                          # union
//! shape:
//!   "{path}":
//!     method: "{method}"                        # GetFoo['method']
//!     request: { ref: "{request}", pick: [body, query] }
//!     response: { ref: "{response}", where: { content_type: application/json } }
//! ```
//!
//! Values reference the generated operation types and never introduce new
//! TS: a variable as the whole value is that part of the operation's type
//! (`"{response.body}"` -> `GetFoo['response']['body']`), narrowed by `where`
//! (`Extract<..>`) and `pick` (`Pick<..>`). Any other template is a string
//! literal type.

use std::fmt;

use anyhow::{Result, bail};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::select::{List, Scope, Where};
use crate::template::{Template, Var};

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
#[cfg_attr(feature = "facet", derive(facet::Facet), facet(untagged))]
#[repr(u8)]
pub enum Shape {
    /// A type: `"{type_name}"`, `"{request}"`, `{ ref, where, pick }`, ..., or
    /// a string literal type from any other template.
    Value(Value),
    /// `{ "<key template>": shape }`: an object type.
    Map(#[cfg_attr(feature = "facet", facet(recursive_type))] IndexMap<String, Shape>),
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
#[cfg_attr(feature = "facet", derive(facet::Facet), facet(untagged))]
#[repr(u8)]
pub enum Value {
    Template(Template),
    Ref(Box<RefValue>),
}

/// `{ ref: "{response}", where: { content_type: application/json } }` ->
/// `Extract<GetFoo['response'], { contentType: 'application/json' }>`;
/// `{ ref: "{request}", pick: [body, query] }` ->
/// `Pick<GetFoo['request'], 'body' | 'query'>`;
/// `{ ref: "{response.body}", where: { status: 2xx } }` ->
/// `Extract<GetFoo['response'], { status: 200 }>['body']`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "facet", derive(facet::Facet), facet(deny_unknown_fields))]
pub struct RefValue {
    /// A reference variable: `"{type_name}"`, `"{request}"`,
    /// `"{response.body}"`, ...
    #[serde(rename = "ref")]
    #[cfg_attr(feature = "facet", facet(rename = "ref"))]
    pub target: Template,
    /// Keep only the request / response variants matching this (before
    /// taking `.body` etc.).
    #[serde(rename = "where", default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(
        feature = "facet",
        facet(rename = "where", default, skip_serializing_if = Option::is_none)
    )]
    pub filter: Option<Where>,
    /// Keep only these keys of `{type_name}`, `{request}` or `{response}`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "facet", facet(default, skip_serializing_if = Option::is_none))]
    pub pick: Option<List<String>>,
}

/// What a value refers to: part of the operation's generated type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RefTarget {
    pub root: Root,
    /// A key of `root`'s type: `GetFoo['request']['body']`.
    pub key: Option<&'static str>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Root {
    /// The operation's type, `GetFoo`.
    Op,
    /// `GetFoo['request']`.
    Request,
    /// `GetFoo['response']`.
    Response,
}

impl RefTarget {
    /// The part of the operation's type a variable stands for; `None` for the
    /// text-only ones.
    pub fn from_var(var: Var) -> Option<Self> {
        let (root, key) = match var {
            Var::TypeName => (Root::Op, None),
            Var::Method => (Root::Op, Some("method")),
            Var::Path => (Root::Op, Some("path")),
            Var::Request => (Root::Request, None),
            Var::RequestContentType => (Root::Request, Some("contentType")),
            Var::RequestBody => (Root::Request, Some("body")),
            Var::RequestParams => (Root::Request, Some("params")),
            Var::RequestQuery => (Root::Request, Some("query")),
            Var::RequestHeaders => (Root::Request, Some("headers")),
            Var::RequestCookies => (Root::Request, Some("cookies")),
            Var::Response => (Root::Response, None),
            Var::ResponseStatus => (Root::Response, Some("status")),
            Var::ResponseContentType => (Root::Response, Some("contentType")),
            Var::ResponseBody => (Root::Response, Some("body")),
            Var::Id | Var::Summary | Var::Description | Var::Deprecated | Var::Tags => return None,
        };
        Some(Self { root, key })
    }
}

impl Root {
    /// The keys of its type, for `pick`.
    pub fn keys(self) -> &'static [&'static str] {
        match self {
            Self::Op => &["method", "path", "request", "response"],
            Self::Request => &[
                "body",
                "contentType",
                "params",
                "query",
                "headers",
                "cookies",
            ],
            Self::Response => &["status", "contentType", "body"],
        }
    }

    /// What a value-level `where` selects among, if it can have one.
    pub fn scope(self) -> Option<Scope> {
        match self {
            Self::Op => None,
            Self::Request => Some(Scope::Request),
            Self::Response => Some(Scope::Response),
        }
    }
}

/// A [`Value`], decoded.
#[derive(Debug, Clone, Copy)]
pub enum Resolved<'a> {
    Ref {
        target: RefTarget,
        filter: Option<&'a Where>,
        pick: &'a [String],
    },
    Literal(&'a Template),
}

impl Value {
    /// Decode and check the value; `vars` are the text variables a literal
    /// may use.
    pub fn resolve(&self, vars: &[Var], context: &str) -> Result<Resolved<'_>> {
        match self {
            Value::Template(template) => resolve_template(template, vars, context),
            Value::Ref(value) => value.resolve(&format!("{context}.ref")),
        }
    }
}

/// A lone reference variable is a [`Resolved::Ref`]; anything else is text,
/// which may not embed a reference variable.
fn resolve_template<'a>(
    template: &'a Template,
    vars: &[Var],
    context: &str,
) -> Result<Resolved<'a>> {
    if let Some(target) = template.single_var().and_then(RefTarget::from_var) {
        return Ok(Resolved::Ref {
            target,
            filter: None,
            pick: &[],
        });
    }
    if let Some(var) = template
        .vars()
        .find(|v| !vars.contains(v) && Var::REF.contains(v))
    {
        bail!(
            "{context}: {:?}: `{{{var}}}` is a type, so it can only be the whole value \
             (narrow it with `{{ ref, where, pick }}`)",
            template.source()
        );
    }
    template.check(vars, context)?;
    Ok(Resolved::Literal(template))
}

impl RefValue {
    fn resolve(&self, context: &str) -> Result<Resolved<'_>> {
        self.target.check(Var::REF, context)?;
        let Some(target) = self.target.single_var().and_then(RefTarget::from_var) else {
            bail!(
                "{context}: must be one variable, e.g. \"{{response}}\", not {:?}",
                self.target.source()
            );
        };
        if let Some(filter) = &self.filter {
            self.check_where(target, filter, context)?;
        }
        let pick = self.pick.as_ref().map(List::as_slice).unwrap_or_default();
        self.check_pick(target, pick, context)?;
        Ok(Resolved::Ref {
            target,
            filter: self.filter.as_ref(),
            pick,
        })
    }

    /// `where` narrows a request or response union, with that scope's fields.
    fn check_where(&self, target: RefTarget, filter: &Where, context: &str) -> Result<()> {
        let Some(scope) = target.root.scope() else {
            bail!(
                "{context}: `where` narrows a request or response union; {:?} isn't part of one",
                self.target.source()
            );
        };
        filter.validate(scope, &format!("{context}.where"))
    }

    /// `pick` takes keys of a whole type (`{request}`, not `{request.body}`).
    fn check_pick(&self, target: RefTarget, pick: &[String], context: &str) -> Result<()> {
        if !pick.is_empty() && target.key.is_some() {
            bail!(
                "{context}: `pick` takes keys of \"{{type_name}}\", \"{{request}}\" or \
                 \"{{response}}\", not of {:?}",
                self.target.source()
            );
        }
        for key in pick {
            if !target.root.keys().contains(&key.as_str()) {
                bail!(
                    "{context}: can't pick `{key}` (keys: {})",
                    target.root.keys().join(", ")
                );
            }
        }
        Ok(())
    }
}

impl Shape {
    /// Check every key template and value; `vars` are the template variables
    /// available per operation.
    pub fn validate(&self, vars: &[Var], context: &str) -> Result<()> {
        match self {
            Shape::Value(value) => value.resolve(vars, context).map(|_| ()),
            Shape::Map(map) => {
                if map.is_empty() {
                    bail!("{context}: an empty map has no members");
                }
                for (key, shape) in map {
                    let context = format!("{context}.{key:?}");
                    Template::lenient(key.clone()).check(vars, &format!("{context} (key)"))?;
                    shape.validate(vars, &context)?;
                }
                Ok(())
            }
        }
    }
}

/// Buffered through `serde_json::Value` because what an object is depends on
/// its keys: one with a `ref` key is a [`RefValue`], any other a map.
impl<'de> Deserialize<'de> for Shape {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = serde_json::Value::deserialize(deserializer)?;
        from_json(value, &mut Vec::new()).map_err(serde::de::Error::custom)
    }
}

fn from_json(value: serde_json::Value, path: &mut Vec<String>) -> Result<Shape, ShapeError> {
    use serde_json::Value as Json;
    let error = |path: &[String], message: String| ShapeError {
        path: path.to_vec(),
        message,
    };
    match value {
        Json::String(source) => Ok(Shape::Value(Value::Template(Template::lenient(source)))),
        Json::Object(map) if map.contains_key("ref") => serde_json::from_value(Json::Object(map))
            .map(|r| Shape::Value(Value::Ref(Box::new(r))))
            .map_err(|e| error(path, e.to_string())),
        Json::Object(map) => {
            let mut shape = IndexMap::new();
            for (key, value) in map {
                path.push(key.clone());
                let value = from_json(value, path)?;
                path.pop();
                shape.insert(key, value);
            }
            Ok(Shape::Map(shape))
        }
        other => Err(error(
            path,
            format!(
                "expected a template string, a `{{ ref }}` object or a map, found {}",
                match other {
                    Json::Null => "null",
                    Json::Bool(_) => "a boolean",
                    Json::Number(_) => "a number",
                    Json::Array(_) => "a list",
                    Json::String(_) | Json::Object(_) => unreachable!(),
                }
            ),
        )),
    }
}

struct ShapeError {
    path: Vec<String>,
    message: String,
}

impl fmt::Display for ShapeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.path.is_empty() {
            f.write_str(&self.message)
        } else {
            let path: Vec<_> = self.path.iter().map(|k| format!("{k:?}")).collect();
            write!(f, "at {}: {}", path.join("."), self.message)
        }
    }
}
