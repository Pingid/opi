//! What a template variable means for each kind of staged item.
//!
//! | item               | variables                                                  |
//! |--------------------|------------------------------------------------------------|
//! | schema             | `name`, `schema`, `deprecated`                             |
//! | route              | `method`, `path`, `deprecated`, `request`, `response`      |
//! | `{route: request}` | `content_type`, `schema` (= `body`), `deprecated` + route's |
//! | `{route: response}`| `status`, `content_type`, `schema`, `deprecated` + route's |
//!
//! A route's `request` is the union of its request variants (one per body
//! media type, plus one without a body when it's optional), each with its
//! parameters; `response` the union of `{ status, contentType, body }`.
//!
//! A text variable can be re-cased by how it's spelled: `{method}` -> `get`,
//! `{METHOD}` -> `GET`, `{Method}` -> `Get`; `{Path}` on `/a/{id}` -> `AById`.

use std::borrow::Cow;

use anyhow::{Result, anyhow};
use indexmap::IndexMap;

use super::docs::DocLines;
use super::{Entry, Expr, Mode, Node};
use crate::ast::{Component, Request, Response, Route, Schema, Status};
use crate::rule::Stage;
use crate::util::{Case, Template};

#[derive(Debug, Clone, Copy)]
pub enum Item<'a> {
    Route(&'a Route),
    Schema(&'a Component),
    Request(&'a Request, &'a Route),
    Response(&'a Response, &'a Route),
}

#[derive(Debug, Clone)]
pub enum Value<'a> {
    Str(Cow<'a, str>),
    /// Like `Str`, but `/a/{id}` becomes `AById` in a name.
    Path(&'a str),
    Num(f64),
    Bool(bool),
    /// A response without content's `content_type`.
    Null,
    /// The union of these; `None` is a body without a schema (`unknown`),
    /// and no members at all is `undefined` (a response without content).
    Type(Vec<Option<&'a Schema>>),
    /// A route's `{request}`.
    Request(&'a Route),
    /// A route's `{response}`.
    Response(&'a Route),
}

impl<'a> Stage<'a> {
    pub fn items(&self) -> Vec<Item<'a>> {
        match self {
            Stage::Routes(v) => v.iter().map(|&r| Item::Route(r)).collect(),
            Stage::Schemas(v) => v.iter().map(|&c| Item::Schema(c)).collect(),
            Stage::Requests(v) => v.iter().map(|&(q, r)| Item::Request(q, r)).collect(),
            Stage::Responses(v) => v.iter().map(|&(s, r)| Item::Response(s, r)).collect(),
        }
    }
}

impl<'a> Item<'a> {
    pub fn kind(&self) -> &'static str {
        match self {
            Item::Route(_) => "route",
            Item::Schema(_) => "schema",
            Item::Request(..) => "request",
            Item::Response(..) => "response",
        }
    }

    /// The variable as spelled, or a text variable re-cased: all caps is
    /// upper case, a leading capital Pascal case.
    pub fn var(&self, name: &str) -> Option<Value<'a>> {
        if let Some(value) = self.raw_var(name) {
            return Some(value);
        }
        let upper = !name.bytes().any(|b| b.is_ascii_lowercase());
        let pascal = name.starts_with(|c: char| c.is_ascii_uppercase());
        if !upper && !pascal {
            return None;
        }
        Some(match self.raw_var(&name.to_ascii_lowercase())? {
            Value::Str(s) if upper => Value::Str(s.to_uppercase().into()),
            Value::Path(s) if upper => Value::Str(s.to_uppercase().into()),
            Value::Str(s) => Value::Str(s.pascal_case().into()),
            Value::Path(s) => Value::Str(s.path_pascal_case().into()),
            _ => return None,
        })
    }

    fn raw_var(&self, name: &str) -> Option<Value<'a>> {
        match *self {
            Item::Schema(c) => match name {
                "name" => Some(Value::Str(c.name.as_str().into())),
                "schema" => Some(Value::Type(vec![Some(&c.schema)])),
                "deprecated" => Some(Value::Bool(c.schema.meta.deprecated)),
                _ => None,
            },
            Item::Route(r) => route_var(r, name),
            Item::Request(q, r) => match name {
                "content_type" => Some(Value::Str(q.content_type.as_str().into())),
                "schema" | "body" => Some(request_body(q)),
                "deprecated" => Some(Value::Bool(q.deprecated())),
                _ => route_var(r, name),
            },
            Item::Response(s, r) => match name {
                "status" => Some(match s.status {
                    Status::Code(code) => Value::Num(code.into()),
                    status => Value::Str(status.to_string().into()),
                }),
                "content_type" => Some(match &s.content_type {
                    Some(content_type) => Value::Str(content_type.as_str().into()),
                    None => Value::Null,
                }),
                "schema" | "body" => Some(response_body(s)),
                "deprecated" => Some(Value::Bool(s.deprecated())),
                _ => route_var(r, name),
            },
        }
    }

    pub fn unknown(&self, var: &str) -> anyhow::Error {
        anyhow!("unknown variable `{{{var}}}` for a {}", self.kind())
    }

    /// The template as text. Type-valued variables (`{schema}`, ...) can't be
    /// spelled, so they're an error here.
    pub fn render(&self, t: &Template, mode: Mode) -> Result<String> {
        let mut out = String::new();
        for (i, literal) in t.literals.iter().enumerate() {
            out.push_str(literal);
            let Some(var) = t.variables.get(i) else {
                continue;
            };
            let value = self.var(var).ok_or_else(|| self.unknown(var))?;
            let text = value.text(mode).ok_or_else(|| {
                anyhow!("`{{{var}}}` is a type; it can't be part of a name or key")
            })?;
            out.push_str(&text);
        }
        Ok(out)
    }

    pub fn default_name(&self) -> Template {
        let source = match self {
            Item::Schema(_) => "{name}",
            Item::Route(_) => "{method}{path}",
            Item::Request(..) => "{method}{path}Request",
            Item::Response(..) => "{method}{path}{status}Response",
        };
        Template::parse(source).expect("built-in templates parse")
    }

    /// Docs for where this item is named: a declaration, or (`brief`) a key
    /// inside a bigger type.
    pub fn docs(&self, brief: bool) -> Vec<String> {
        match *self {
            Item::Schema(c) => DocLines::schema(&c.schema.meta),
            Item::Route(r) if brief => DocLines::route_brief(r),
            Item::Route(r) => DocLines::route(r),
            Item::Request(q, _) => DocLines::default().tag("description", q.description.as_deref()),
            Item::Response(s, _) => {
                DocLines::default().tag("description", s.description.as_deref())
            }
        }
        .done()
    }

    /// What an item is when `fields` is omitted: a schema, request or
    /// response is its schema; a route is an object of its parts.
    pub fn natural(&self) -> Node<'a> {
        match *self {
            Item::Route(r) => Node::Map(
                ["method", "path", "request", "response"]
                    .into_iter()
                    .filter_map(|k| {
                        let node = Node::leaf(Expr::Value(route_var(r, k)?));
                        Some((k.to_string(), Entry::new(node)))
                    })
                    .collect::<IndexMap<_, _>>(),
            ),
            Item::Schema(c) => Node::leaf(Expr::Value(Value::Type(vec![Some(&c.schema)]))),
            Item::Request(q, _) => Node::leaf(Expr::Value(request_body(q))),
            Item::Response(s, _) => Node::leaf(Expr::Value(response_body(s))),
        }
    }
}

fn route_var<'a>(r: &'a Route, name: &str) -> Option<Value<'a>> {
    Some(match name {
        "method" => Value::Str(r.method.as_str().into()),
        "path" => Value::Path(&r.path),
        "deprecated" => Value::Bool(r.deprecated()),
        "request" => Value::Request(r),
        "response" => Value::Response(r),
        _ => return None,
    })
}

fn request_body(q: &Request) -> Value<'_> {
    Value::Type(vec![q.body.as_ref()])
}

/// `undefined` without content, `unknown` for content without a schema.
fn response_body(s: &Response) -> Value<'_> {
    match s.content_type {
        None => Value::Type(vec![]),
        Some(_) => Value::Type(vec![s.schema.as_ref()]),
    }
}

impl Value<'_> {
    fn text(&self, mode: Mode) -> Option<String> {
        Some(match (self, mode) {
            (Value::Str(s), Mode::Name) => s.pascal_case(),
            (Value::Path(s), Mode::Name) => s.path_pascal_case(),
            (Value::Str(s), Mode::Key) => s.to_string(),
            (Value::Path(s), Mode::Key) => s.to_string(),
            (Value::Num(n), _) => n.to_string(),
            (Value::Bool(b), Mode::Name) => (if *b { "True" } else { "False" }).to_string(),
            (Value::Bool(b), Mode::Key) => b.to_string(),
            (Value::Null, Mode::Name) => "Null".to_string(),
            (Value::Null, Mode::Key) => "null".to_string(),
            (Value::Type(_) | Value::Request(_) | Value::Response(_), _) => return None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{Method, Params};

    fn route() -> Route {
        Route {
            path: "/a/{id}".into(),
            method: Method::Get,
            summary: None,
            description: None,
            deprecated: None,
            params: Params::default(),
            body_required: false,
            request: vec![],
            response: vec![],
        }
    }

    fn text(item: Item, var: &str) -> Option<String> {
        item.var(var)?.text(Mode::Key)
    }

    #[test]
    fn variables_are_recased_by_spelling() {
        let r = route();
        let item = Item::Route(&r);
        assert_eq!(text(item, "method").as_deref(), Some("get"));
        assert_eq!(text(item, "METHOD").as_deref(), Some("GET"));
        assert_eq!(text(item, "Method").as_deref(), Some("Get"));
        assert_eq!(text(item, "path").as_deref(), Some("/a/{id}"));
        assert_eq!(text(item, "Path").as_deref(), Some("AById"));
        assert!(item.var("mEthod").is_none());
        // Types have no case.
        assert!(item.var("REQUEST").is_none());
    }
}
