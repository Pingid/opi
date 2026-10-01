//! `ir::Schema` -> `TSType`.

use oxc_ast::ast::*;

use super::{DocLines, Generator};
use crate::ir::{Literal, Object, Schema, SchemaKind};

impl<'a> Generator<'a> {
    /// `export type Foo = ...` for an entry in `components.schemas`.
    pub(super) fn named_schema(&self, raw: &str, schema: &Schema) -> Statement<'a> {
        let name = self.names.schema(raw).expect("all schemas are named up front");
        let docs = DocLines::default()
            .tag("title", schema.meta.title.as_deref())
            .meta(&schema.meta)
            .done();
        self.export_type(name, self.schema(schema), docs)
    }

    pub(super) fn schema(&self, schema: &Schema) -> TSType<'a> {
        let b = self.b;
        let ty = match &schema.kind {
            SchemaKind::Unknown => b.unknown(),
            SchemaKind::Never => b.never(),
            SchemaKind::String { format } => self.format(format).unwrap_or_else(|| b.string()),
            SchemaKind::Number { format } | SchemaKind::Integer { format } => {
                self.format(format).unwrap_or_else(|| b.number())
            }
            SchemaKind::Boolean => b.boolean(),
            SchemaKind::Null => b.null(),
            SchemaKind::Enum(values) => b.union(values.iter().map(|v| self.literal(v))),
            SchemaKind::Array(items) => b.array(self.schema(items)),
            SchemaKind::Object(object) => self.object(object),
            SchemaKind::Union(members) => b.union(members.iter().map(|m| self.schema(m))),
            SchemaKind::Intersection(members) => {
                b.intersection(members.iter().map(|m| self.schema(m)))
            }
            SchemaKind::Ref(raw) => match self.names.schema(raw) {
                Some(name) => b.reference(b.str(name)),
                // Dangling ref: degrade rather than emit a name that doesn't exist.
                None => b.unknown(),
            },
        };
        if schema.meta.nullable {
            b.nullable(ty)
        } else {
            ty
        }
    }

    fn object(&self, object: &Object) -> TSType<'a> {
        let b = self.b;
        if object.properties.is_empty() {
            let value = object.additional.as_deref().map(|s| self.schema(s));
            return b.record(value.unwrap_or_else(|| b.unknown()));
        }

        let mut members = b.member_builder();
        for (key, property) in &object.properties {
            let meta = &property.schema.meta;
            let member = b.property(
                b.str(key),
                b.annotation(self.schema(&property.schema)),
                false,
                !property.required,
                self.config.readonly && meta.read_only,
            );
            members = members.with(self.doc(member, DocLines::default().meta(meta).done()));
        }
        if let Some(additional) = object.additional.as_deref() {
            members = members.index_signature(self.index_signature(object, additional));
        }
        members.into_literal()
    }

    /// TS requires every named property to be assignable to the index
    /// signature, so `{ a?: string; [key: string]: number }` is an error.
    /// Widen the index type to cover the named properties too.
    fn index_signature(&self, object: &Object, additional: &Schema) -> TSType<'a> {
        let b = self.b;
        if additional.kind == SchemaKind::Unknown {
            return b.unknown();
        }
        let properties = object.properties.values().map(|p| self.schema(&p.schema));
        let optional = object
            .properties
            .values()
            .any(|p| !p.required)
            .then(|| b.undefined());
        b.union(std::iter::once(self.schema(additional)).chain(properties).chain(optional))
    }

    fn literal(&self, literal: &Literal) -> TSType<'a> {
        let b = self.b;
        match literal {
            Literal::String(s) => b.string_literal(b.str(s)),
            Literal::Number(n) => b.number_literal(*n),
            Literal::Bool(v) => b.boolean_literal(*v),
            Literal::Null => b.null(),
        }
    }

    /// `format: binary` -> `Blob` etc. via [`crate::Config::formats`].
    fn format(&self, format: &Option<String>) -> Option<TSType<'a>> {
        let ts = self.config.formats.get(format.as_deref()?)?;
        Some(self.b.reference(self.b.str(ts)))
    }
}
