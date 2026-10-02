//! `emit`: turns each staged item into (part of) a named declaration.
//!
//! ```yaml
//! emit:
//!   type: "{method}{path}"          # declaration name, per item
//!   fields:                         # optional; omitted = the item's own type
//!     method: "{method}"            # one variable alone: its value / type
//!     "{path}":                     # keys are templates too (raw values)
//!       request: "{method}{path}['request']"   # anything else: a reference
//! ```
//!
//! Two phases. [`Emit::collect`] renders every template against every item
//! and merges the results into [`Output`]: items whose `type` renders to the
//! same name share one declaration, their field trees are merged, and a leaf
//! written by several items becomes a union. [`Output::render`] then lowers
//! everything to TS once, after all rules ran, so references can point at
//! declarations from any rule.

mod docs;
mod item;
mod lower;

use anyhow::{Result, anyhow, bail};
use indexmap::IndexMap;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::rule::Stage;
use crate::util::{Case, Template};

pub use item::{Item, Value};
pub use lower::Output;

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct Emit {
    /// Defaults per item: `{name}` for schemas, `{method}{path}` for routes,
    /// `{method}{path}Request` / `{method}{path}{status}Response`.
    #[serde(rename = "type", default, skip_serializing_if = "Option::is_none")]
    pub type_: Option<Template>,
    /// Omitted: the item as itself. A schema, request or response is its type;
    /// a route is `{ method, path, request, response }`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fields: Option<EmitFields>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, JsonSchema)]
#[serde(untagged)]
pub enum EmitFields {
    Value(Template),
    Map(IndexMap<Template, EmitFields>),
}

impl Emit {
    pub const DEFAULT: Emit = Emit {
        type_: None,
        fields: None,
    };

    pub fn collect<'a>(&self, stage: &Stage<'a>, out: &mut Output<'a>) -> Result<()> {
        for item in stage.items() {
            let default = item.default_name();
            let template = self.type_.as_ref().unwrap_or(&default);
            let name = item.render(template, Mode::Name)?.identifier();
            // Docs go where the item is named: a declaration named per item.
            let docs = match template.has_variables() {
                true => item.docs(false),
                false => Vec::new(),
            };
            let body = match &self.fields {
                Some(fields) => build(fields, &item, &name)?,
                None => item.natural(),
            };
            // A schema emitted as itself is where `Ref`s to it resolve.
            let component = match (&self.fields, item) {
                (None, Item::Schema(c)) => Some(c.name.as_str()),
                _ => None,
            };
            out.add(name, body, component, docs)?;
        }
        Ok(())
    }
}

/// How a variable is spelled when rendered into text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Part of a declaration name: `get` -> `Get`, `/a/{id}` -> `AById`.
    Name,
    /// An object key or string literal: the value as is.
    Key,
}

/// The type tree of one declaration, before lowering.
#[derive(Debug, Clone)]
pub enum Node<'a> {
    /// The union of everything written here.
    Leaf(Vec<Expr<'a>>),
    Map(IndexMap<String, Entry<'a>>),
}

/// A field of a [`Node::Map`].
#[derive(Debug, Clone)]
pub struct Entry<'a> {
    pub node: Node<'a>,
    /// The docs of the item a templated key names (`"{METHOD} {path}"`: the
    /// route's summary). Cleared when items with different docs share it.
    pub docs: Vec<String>,
}

impl<'a> Entry<'a> {
    pub fn new(node: Node<'a>) -> Self {
        Self {
            node,
            docs: Vec::new(),
        }
    }

    fn merge(&mut self, other: Entry<'a>, at: &str) -> Result<()> {
        if self.docs != other.docs {
            self.docs.clear();
        }
        self.node.merge(other.node, at)
    }
}

#[derive(Debug, Clone)]
pub enum Expr<'a> {
    /// `"{method}"`, `"{schema}"`, ...: a variable's value or type.
    Value(Value<'a>),
    /// `"'text'"`: a string literal type.
    Literal(String),
    /// `"{method}{path}['request']"`: `GetUsers['request']`.
    Ref { head: String, path: Vec<String> },
}

impl<'a> Node<'a> {
    pub fn leaf(expr: Expr<'a>) -> Self {
        Node::Leaf(vec![expr])
    }

    fn merge(&mut self, other: Node<'a>, at: &str) -> Result<()> {
        match (self, other) {
            (Node::Leaf(a), Node::Leaf(b)) => a.extend(b),
            (Node::Map(a), Node::Map(b)) => {
                for (key, entry) in b {
                    insert(a, key, entry, at)?;
                }
            }
            _ => bail!("`{at}` is emitted both as a type and as an object"),
        }
        Ok(())
    }
}

fn insert<'a>(
    map: &mut IndexMap<String, Entry<'a>>,
    key: String,
    entry: Entry<'a>,
    at: &str,
) -> Result<()> {
    match map.get_mut(&key) {
        Some(existing) => existing.merge(entry, &format!("{at}.{key}")),
        None => {
            map.insert(key, entry);
            Ok(())
        }
    }
}

/// One item's contribution to a declaration's field tree.
fn build<'a>(fields: &EmitFields, item: &Item<'a>, at: &str) -> Result<Node<'a>> {
    Ok(match fields {
        EmitFields::Value(t) => Node::leaf(value(t, item)?),
        EmitFields::Map(entries) => {
            let mut map = IndexMap::new();
            for (template, child) in entries {
                let docs = match template.has_variables() {
                    true => item.docs(true),
                    false => Vec::new(),
                };
                let key = item.render(template, Mode::Key)?;
                let path = format!("{at}.{key}");
                let node = build(child, item, &path)?;
                // Two key templates can render to the same key: merge them.
                insert(&mut map, key, Entry { node, docs }, at)?;
            }
            Node::Map(map)
        }
    })
}

/// A field value: one bare variable is that variable's value; a quoted
/// template is a string literal; anything else names a declaration,
/// optionally followed by `['key']` accessors.
fn value<'a>(t: &Template, item: &Item<'a>) -> Result<Expr<'a>> {
    if let [var] = t.variables.as_slice()
        && t.literals.iter().all(String::is_empty)
    {
        return item
            .var(var)
            .map(Expr::Value)
            .ok_or_else(|| item.unknown(var));
    }
    let quoted = t
        .literals
        .first()
        .is_some_and(|l| l.trim_start().starts_with(['\'', '"']));
    if quoted {
        let text = item.render(t, Mode::Key)?;
        return unquote(text.trim())
            .map(Expr::Literal)
            .ok_or_else(|| anyhow!("unterminated string literal `{text}`"));
    }
    reference(&item.render(t, Mode::Name)?)
}

/// `Head['a']["b"]` -> `Ref { head: "Head", path: ["a", "b"] }`.
fn reference<'a>(text: &str) -> Result<Expr<'a>> {
    let text = text.trim();
    let (head, mut rest) = match text.find('[') {
        Some(i) => (&text[..i], &text[i..]),
        None => (text, ""),
    };
    let head = head.trim();
    if head.is_empty() {
        bail!("`{text}` doesn't name a type");
    }
    let mut path = Vec::new();
    while !rest.is_empty() {
        let (key, tail) = rest
            .strip_prefix('[')
            .and_then(|r| r.split_once(']'))
            .ok_or_else(|| anyhow!("expected `['key']` in `{text}`"))?;
        let key =
            unquote(key.trim()).ok_or_else(|| anyhow!("expected a quoted key in `{text}`"))?;
        path.push(key);
        rest = tail.trim_start();
    }
    Ok(Expr::Ref {
        head: head.identifier(),
        path,
    })
}

fn unquote(s: &str) -> Option<String> {
    ['\'', '"'].into_iter().find_map(|q| {
        let inner = s.strip_prefix(q)?.strip_suffix(q)?;
        Some(inner.to_string())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_references() {
        let Expr::Ref { head, path } = reference("GetUsersById['request'][\"body\"]").unwrap()
        else {
            panic!()
        };
        assert_eq!(head, "GetUsersById");
        assert_eq!(path, ["request", "body"]);
        assert!(reference("['x']").is_err());
        assert!(reference("Foo[x]").is_err());
    }
}
