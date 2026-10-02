use std::collections::{HashMap, HashSet};

use anyhow::{Context, Result, anyhow, bail};
use indexmap::IndexMap;
use oxc_ast::ast::TSType;

use crate::ast::{Doc, Literal, Meta, Object, Schema, SchemaKind};
use crate::code::TypeBuilder;
use crate::util::Case;

use super::docs::DocLines;
use super::{Expr, Node, Value};

mod route;

/// Globals that generated types reference (or that users will expect to
/// still work), so a declaration called `Error` is emitted as `Error2`
/// rather than shadowing them. `formats` types are added to these.
const RESERVED: &[&str] = &[
    "Array", "Blob", "Boolean", "Date", "Error", "Exclude", "Extract", "File", "Map", "Number",
    "Object", "Omit", "Partial", "Pick", "Promise", "Readonly", "Record", "Set", "String",
];

/// Declarations from every rule, by name, in first-emitted order.
#[derive(Debug, Default)]
pub struct Output<'a> {
    decls: IndexMap<String, Decl<'a>>,
}

#[derive(Debug)]
struct Decl<'a> {
    body: Node<'a>,
    /// The component this declaration *is*, if exactly one schema was emitted
    /// as itself under this name. `Ref`s to that component point here.
    component: Option<&'a str>,
    /// The docs of the item it's named for; cleared when items with
    /// different docs share the name.
    docs: Vec<String>,
}

impl<'a> Output<'a> {
    pub(super) fn add(
        &mut self,
        name: String,
        body: Node<'a>,
        component: Option<&'a str>,
        docs: Vec<String>,
    ) -> Result<()> {
        if let Some(decl) = self.decls.get_mut(&name) {
            if decl.component != component {
                decl.component = None;
            }
            if decl.docs != docs {
                decl.docs.clear();
            }
            return decl.body.merge(body, &name);
        }
        let decl = Decl {
            body,
            component,
            docs,
        };
        self.decls.insert(name, decl);
        Ok(())
    }

    /// Declaration name -> the identifier it's emitted as: itself, or
    /// suffixed (`Error` -> `Error2`) when it would shadow a [`RESERVED`]
    /// global or a `formats` type.
    fn identifiers(&self, formats: &IndexMap<String, String>) -> HashMap<&str, String> {
        let reserved: HashSet<&str> = RESERVED
            .iter()
            .copied()
            .chain(
                formats
                    .values()
                    .map(String::as_str)
                    .filter(|t| t.is_identifier()),
            )
            .collect();
        let mut taken: HashSet<String> = self.decls.keys().cloned().collect();
        taken.extend(reserved.iter().map(|name| name.to_string()));
        let mut out = HashMap::with_capacity(self.decls.len());
        for name in self.decls.keys() {
            let mut identifier = name.clone();
            if reserved.contains(name.as_str()) {
                let mut n = 2;
                while taken.contains(&format!("{name}{n}")) {
                    n += 1;
                }
                identifier = format!("{name}{n}");
                taken.insert(identifier.clone());
            }
            out.insert(name.as_str(), identifier);
        }
        out
    }

    /// `formats`: schema `format` -> the TS type to use instead of the base
    /// type (`binary` -> `Blob`).
    pub fn render(&self, doc: &'a Doc, formats: &IndexMap<String, String>) -> Result<String> {
        let allocator = TypeBuilder::allocator();
        let tb = TypeBuilder::new(&allocator);
        let identifiers = self.identifiers(formats);
        let mut lower = Lower {
            tb: &tb,
            doc,
            decls: &self.decls,
            identifiers: &identifiers,
            components: self
                .decls
                .iter()
                .filter_map(|(name, d)| Some((d.component?, identifiers[name.as_str()].as_str())))
                .collect(),
            formats,
            stack: Vec::new(),
        };

        let mut body = Vec::with_capacity(self.decls.len());
        for (name, decl) in &self.decls {
            let ty = lower
                .node(&decl.body)
                .with_context(|| format!("emitting `{name}`"))?;
            let identifier = tb.str(&identifiers[name.as_str()]);
            let statement = tb.export(tb.type_alias(identifier, ty));
            let docs = match decl.docs.is_empty() {
                false => decl.docs.clone(),
                // A fixed name (`type: Foo`) for one schema still documents it.
                true => decl
                    .component
                    .and_then(|c| doc.schema(c))
                    .map(|s| doc_lines(&s.meta))
                    .unwrap_or_default(),
            };
            body.push(tb.with_doc(statement, &docs));
        }
        Ok(tb.code_gen(body))
    }
}

struct Lower<'s, 'a, 'b> {
    tb: &'s TypeBuilder<'b>,
    doc: &'a Doc,
    decls: &'s IndexMap<String, Decl<'a>>,
    /// Declaration name -> the identifier it's emitted as.
    identifiers: &'s HashMap<&'s str, String>,
    /// Raw component name -> the identifier of the declaration emitted for it.
    components: HashMap<&'a str, &'s str>,
    formats: &'s IndexMap<String, String>,
    /// Components being inlined, to cut cycles.
    stack: Vec<String>,
}

impl<'b> Lower<'_, '_, 'b> {
    fn node(&mut self, node: &Node<'_>) -> Result<TSType<'b>> {
        let tb = self.tb;
        match node {
            Node::Leaf(exprs) => {
                let mut types = Vec::with_capacity(exprs.len());
                for expr in exprs {
                    types.push(self.expr(expr)?);
                }
                // Several items writing one leaf: the union (deduplicated).
                Ok(tb.union(types))
            }
            Node::Map(fields) => {
                let mut members = Vec::with_capacity(fields.len());
                for (key, entry) in fields {
                    let ty = self
                        .node(&entry.node)
                        .with_context(|| format!("in `{key}`"))?;
                    members.push(tb.with_doc(tb.prop(tb.str(key), ty), &entry.docs));
                }
                Ok(tb.type_literal(members))
            }
        }
    }

    fn expr(&mut self, expr: &Expr<'_>) -> Result<TSType<'b>> {
        let tb = self.tb;
        Ok(match expr {
            Expr::Value(value) => self.value(value),
            Expr::Literal(s) => tb.string_literal(tb.str(s)),
            Expr::Ref { head, path } => {
                let mut ty = if let Some(identifier) = self.identifiers.get(head.as_str()) {
                    self.check_path(head, path)?;
                    tb.reference(tb.str(identifier))
                } else if let Some(keyword) = keyword(tb, head) {
                    keyword
                } else {
                    bail!(
                        "`{head}` isn't an emitted type (for a string literal, quote it: `'{head}'`)"
                    );
                };
                for key in path {
                    ty = tb.indexed_access(ty, tb.str(key));
                }
                ty
            }
        })
    }

    /// `Foo['a']['b']` must name fields that exist, as far as we can tell
    /// (a field holding a schema is left for `tsc` to check).
    fn check_path(&self, head: &str, path: &[String]) -> Result<()> {
        let mut node = &self.decls[head].body;
        for key in path {
            match node {
                Node::Map(fields) => {
                    node = fields
                        .get(key)
                        .map(|entry| &entry.node)
                        .ok_or_else(|| anyhow!("`{head}` has no field `{key}`"))?;
                }
                Node::Leaf(_) => break,
            }
        }
        Ok(())
    }

    fn value(&mut self, value: &Value<'_>) -> TSType<'b> {
        let tb = self.tb;
        match value {
            Value::Str(s) => tb.string_literal(tb.str(s)),
            Value::Path(s) => tb.string_literal(tb.str(s)),
            Value::Null => tb.null(),
            Value::Request(route) => self.request(route),
            Value::Response(route) => self.response(route),
            Value::Num(n) => tb.number_literal(*n),
            Value::Bool(b) => tb.boolean_literal(*b),
            Value::Type(schemas) if schemas.is_empty() => tb.undefined(),
            Value::Type(schemas) => {
                let mut types = Vec::with_capacity(schemas.len());
                for schema in schemas {
                    types.push(match schema {
                        Some(schema) => self.schema(schema),
                        None => tb.unknown(),
                    });
                }
                tb.union(types)
            }
        }
    }

    fn schema(&mut self, s: &Schema) -> TSType<'b> {
        let tb = self.tb;
        let ty = match &s.kind {
            SchemaKind::Unknown => tb.unknown(),
            SchemaKind::Never => tb.never(),
            SchemaKind::String(format) => self.format(format).unwrap_or_else(|| tb.string()),
            SchemaKind::Number(format) | SchemaKind::Integer(format) => {
                self.format(format).unwrap_or_else(|| tb.number())
            }
            SchemaKind::Boolean => tb.boolean(),
            SchemaKind::Null => return tb.null(),
            SchemaKind::Enum(values) => tb.union(values.iter().map(|v| literal(tb, v))),
            SchemaKind::Array(items) => {
                let item = self.schema(items);
                array(tb, item)
            }
            SchemaKind::Object(object) => self.object(object),
            SchemaKind::Union(members) => {
                let types = self.all(members);
                tb.union(types)
            }
            SchemaKind::Intersection(members) => {
                let types = self.all(members);
                tb.intersection(types)
            }
            SchemaKind::Ref(name) => self.reference(name),
        };
        if s.meta.nullable { tb.nullable(ty) } else { ty }
    }

    /// The `formats` type for a `format`, written as given (`Blob`, `Date`).
    fn format(&self, format: &Option<String>) -> Option<TSType<'b>> {
        let ts = self.formats.get(format.as_deref()?)?;
        Some(self.tb.reference(self.tb.str(ts)))
    }

    fn all(&mut self, members: &[Schema]) -> Vec<TSType<'b>> {
        members.iter().map(|m| self.schema(m)).collect()
    }

    fn object(&mut self, object: &Object) -> TSType<'b> {
        let tb = self.tb;
        if object.properties.is_empty()
            && let Some(additional) = &object.additional
        {
            let value = self.schema(additional);
            return tb.record(value);
        }
        let mut members = Vec::with_capacity(object.properties.len() + 1);
        for (key, property) in &object.properties {
            let meta = &property.schema.meta;
            let ty = self.schema(&property.schema);
            let member = tb.property(tb.str(key), ty, !property.required, meta.read_only);
            members.push(tb.with_doc(member, &doc_lines(meta)));
        }
        if let Some(additional) = &object.additional {
            let value = self.index_signature(object, additional);
            members.push(tb.index_signature(value));
        }
        tb.type_literal(members)
    }

    /// TS requires every named property to be assignable to the index
    /// signature, so `{ a?: string; [key: string]: string }` is an error
    /// (`a` is `string | undefined`). Widen the index type to cover them.
    fn index_signature(&mut self, object: &Object, additional: &Schema) -> TSType<'b> {
        let tb = self.tb;
        if additional.kind == SchemaKind::Unknown && !additional.meta.nullable {
            return tb.unknown();
        }
        let mut types = vec![self.schema(additional)];
        for property in object.properties.values() {
            types.push(self.schema(&property.schema));
        }
        if object.properties.values().any(|p| !p.required) {
            types.push(tb.undefined());
        }
        tb.union(types)
    }

    /// A component emitted as its own declaration is referenced by name;
    /// any other is inlined. A cycle through non-emitted components can't be
    /// inlined, so the back edge becomes `unknown`.
    fn reference(&mut self, name: &str) -> TSType<'b> {
        let tb = self.tb;
        if let Some(decl) = self.components.get(name) {
            return tb.reference(tb.str(decl));
        }
        let doc = self.doc;
        let Some(target) = doc.schema(name) else {
            return tb.unknown(); // dangling ref
        };
        if self.stack.iter().any(|n| n == name) {
            return tb.unknown();
        }
        self.stack.push(name.to_string());
        let ty = self.schema(target);
        self.stack.pop();
        ty
    }
}

/// `T[]`, or `Array<T>` where `T[]` would need parentheses.
fn array<'b>(tb: &TypeBuilder<'b>, item: TSType<'b>) -> TSType<'b> {
    match item {
        TSType::TSUnionType(_) | TSType::TSIntersectionType(_) => tb.generic("Array", [item]),
        item => tb.array(item),
    }
}

fn literal<'b>(tb: &TypeBuilder<'b>, value: &Literal) -> TSType<'b> {
    match value {
        Literal::String(s) => tb.string_literal(tb.str(s)),
        Literal::Number(n) => tb.number_literal(*n),
        Literal::Bool(b) => tb.boolean_literal(*b),
        Literal::Null => tb.null(),
    }
}

/// Built-in type names a reference may use besides emitted declarations.
fn keyword<'b>(tb: &TypeBuilder<'b>, name: &str) -> Option<TSType<'b>> {
    Some(match name {
        "string" => tb.string(),
        "number" => tb.number(),
        "boolean" => tb.boolean(),
        "null" => tb.null(),
        "undefined" => tb.undefined(),
        "unknown" => tb.unknown(),
        "never" => tb.never(),
        _ => return None,
    })
}

fn doc_lines(meta: &Meta) -> Vec<String> {
    DocLines::schema(meta).done()
}
