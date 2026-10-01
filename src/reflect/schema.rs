//! JSON Schema (2020-12) from facet reflection, describing what the serde
//! side reads and writes:
//! - proxied types as their proxy (`Template` is a string);
//! - untagged enums as `anyOf` their variants;
//! - `Option` and defaulted fields as optional.
//!
//! Recursive types (`Where`, `Shape`) are defined once in `$defs` and
//! referenced with `$ref`; everything else is inlined.

use std::any::TypeId;
use std::collections::{HashMap, HashSet};

use facet::{Def, EnumType, Field, Shape, StructKind, StructType, Type, UserType};
use serde_json::{Map, Value, json};

use super::doc::doc_text;

/// The schema for `root`. `overrides` replaces the schema of the named types
/// (by `type_identifier`) for ones whose serde form reflection can't see.
pub fn schema_for(root: &'static Shape, overrides: &[(&'static str, Value)]) -> Value {
    let mut ctx = Context {
        overrides,
        in_progress: Vec::new(),
        recursive: HashSet::new(),
        defined: HashMap::new(),
        names: HashMap::new(),
        defs: Map::new(),
    };
    let body = ctx.schema(root);

    let mut schema = Map::new();
    schema.insert(
        "$schema".into(),
        "https://json-schema.org/draft/2020-12/schema".into(),
    );
    if let Value::Object(body) = body {
        schema.extend(body);
    }
    if !ctx.defs.is_empty() {
        schema.insert("$defs".into(), Value::Object(ctx.defs));
    }
    Value::Object(schema)
}

struct Context<'o> {
    overrides: &'o [(&'static str, Value)],
    /// Named types being expanded, to spot recursion.
    in_progress: Vec<TypeId>,
    /// Named types found to contain themselves; they go in `$defs`.
    recursive: HashSet<TypeId>,
    /// Types already in `$defs`, by their name there.
    defined: HashMap<TypeId, String>,
    /// `$defs` name -> type, so two generic instances don't share a name.
    names: HashMap<String, TypeId>,
    defs: Map<String, Value>,
}

impl Context<'_> {
    fn schema(&mut self, shape: &'static Shape) -> Value {
        if let Some((_, schema)) = self
            .overrides
            .iter()
            .find(|(n, _)| *n == shape.type_identifier)
        {
            return schema.clone();
        }
        if let Some(proxy) = shape.proxy {
            return self.schema(proxy.shape);
        }
        let id = shape.id.get();
        if let Some(name) = self.defined.get(&id) {
            return reference(name);
        }
        if self.in_progress.contains(&id) {
            self.recursive.insert(id);
            return reference(&self.def_name(shape));
        }

        match shape.def {
            Def::Option(option) => json!({ "anyOf": [self.schema(option.t), { "type": "null" }] }),
            Def::List(list) => json!({ "type": "array", "items": self.schema(list.t) }),
            Def::Array(array) => json!({ "type": "array", "items": self.schema(array.t) }),
            Def::Set(set) => json!({ "type": "array", "items": self.schema(set.t) }),
            Def::Map(map) => {
                json!({ "type": "object", "additionalProperties": self.schema(map.v) })
            }
            Def::Pointer(pointer) => match pointer.pointee() {
                Some(pointee) => self.schema(pointee),
                None => json!({}),
            },
            Def::Scalar => scalar(shape.type_identifier),
            _ => match shape.ty {
                Type::User(UserType::Struct(st)) => self.named(shape, |c| c.structure(shape, &st)),
                Type::User(UserType::Enum(en)) => self.named(shape, |c| c.enumeration(shape, &en)),
                _ => match shape.inner {
                    Some(inner) => self.schema(inner),
                    None => json!({}),
                },
            },
        }
    }

    /// Expand a named type, moving it into `$defs` if it turned out to be
    /// recursive.
    fn named(&mut self, shape: &'static Shape, expand: impl FnOnce(&mut Self) -> Value) -> Value {
        let id = shape.id.get();
        self.in_progress.push(id);
        let schema = expand(self);
        self.in_progress.pop();
        if !self.recursive.contains(&id) {
            return schema;
        }
        let name = self.def_name(shape);
        self.defs.insert(name.clone(), schema);
        self.defined.insert(id, name.clone());
        reference(&name)
    }

    /// `Where`, or `Where2` if another type already has that name.
    fn def_name(&mut self, shape: &'static Shape) -> String {
        let id = shape.id.get();
        let base = shape.type_identifier;
        let mut name = base.to_string();
        let mut n = 1;
        while let Some(&other) = self.names.get(&name) {
            if other == id {
                return name;
            }
            n += 1;
            name = format!("{base}{n}");
        }
        self.names.insert(name.clone(), id);
        name
    }

    fn structure(&mut self, shape: &'static Shape, st: &StructType) -> Value {
        let mut schema = match st.kind {
            StructKind::Unit => json!({ "type": "null" }),
            StructKind::TupleStruct | StructKind::Tuple if st.fields.len() == 1 => {
                return self.field(&st.fields[0]);
            }
            StructKind::TupleStruct | StructKind::Tuple => {
                let items: Vec<_> = st.fields.iter().map(|f| self.field(f)).collect();
                json!({ "type": "array", "prefixItems": items, "items": false })
            }
            StructKind::Struct => self.object(st.fields),
        };
        describe(&mut schema, shape.type_identifier, shape.doc);
        schema
    }

    /// `{ type: object, properties, required }`. A field is optional if it's
    /// an `Option` or has a default; a flattened field's own fields are
    /// merged in.
    fn object(&mut self, fields: &'static [Field]) -> Value {
        let mut properties = Map::new();
        let mut required = Vec::new();
        self.collect(fields, &mut properties, &mut required);
        object_of(properties, required)
    }

    fn collect(
        &mut self,
        fields: &'static [Field],
        properties: &mut Map<String, Value>,
        required: &mut Vec<Value>,
    ) {
        for field in fields
            .iter()
            .filter(|f| !f.should_skip_serializing_unconditional())
        {
            if field.is_flattened()
                && let Type::User(UserType::Struct(st)) = field.shape().ty
            {
                self.collect(st.fields, properties, required);
                continue;
            }
            let name = field.effective_name();
            let mut schema = self.field(field);
            if let Some(doc) = doc_text(field.doc)
                && let Value::Object(schema) = &mut schema
            {
                schema.insert("description".into(), doc.into());
            }
            properties.insert(name.into(), schema);
            if !matches!(field.shape().def, Def::Option(_)) && field.default.is_none() {
                required.push(Value::from(name));
            }
        }
    }

    fn field(&mut self, field: &'static Field) -> Value {
        match field.proxy {
            Some(proxy) => self.schema(proxy.shape),
            None => self.schema(field.shape()),
        }
    }

    /// Unit-only enums are a string enum; others are `anyOf` their variants:
    /// internally tagged (`{ "<tag>": "name", ..fields }`), untagged (the
    /// variant's own shape), or externally tagged (`{ "name": inner }`).
    fn enumeration(&mut self, shape: &'static Shape, en: &EnumType) -> Value {
        let unit = |v: &facet::Variant| v.data.kind == StructKind::Unit;
        let mut schema = if en.variants.iter().all(unit) {
            let names: Vec<_> = en.variants.iter().map(|v| v.effective_name()).collect();
            json!({ "type": "string", "enum": names })
        } else {
            let variants: Vec<_> = en
                .variants
                .iter()
                .map(|variant| match shape.tag {
                    Some(tag) => self.tagged_variant(tag, variant),
                    None if shape.is_untagged() => self.variant_data(variant),
                    None => self.externally_tagged(variant),
                })
                .collect();
            json!({ "anyOf": variants })
        };
        describe(&mut schema, shape.type_identifier, shape.doc);
        schema
    }

    /// A variant's payload: its name for a unit, a lone field's schema, a
    /// tuple, or an object.
    fn variant_data(&mut self, variant: &facet::Variant) -> Value {
        let data = &variant.data;
        match data.kind {
            StructKind::Unit => json!({ "const": variant.effective_name() }),
            StructKind::TupleStruct | StructKind::Tuple if data.fields.len() == 1 => {
                self.field(&data.fields[0])
            }
            StructKind::TupleStruct | StructKind::Tuple => {
                let items: Vec<_> = data.fields.iter().map(|f| self.field(f)).collect();
                json!({ "type": "array", "prefixItems": items, "items": false })
            }
            StructKind::Struct => self.object(data.fields),
        }
    }

    /// `{ "<name>": inner }`, or the bare name for a unit variant.
    fn externally_tagged(&mut self, variant: &facet::Variant) -> Value {
        let name = variant.effective_name();
        if variant.data.kind == StructKind::Unit {
            return json!({ "const": name });
        }
        let inner = self.variant_data(variant);
        json!({
            "type": "object",
            "properties": { name: inner },
            "required": [name],
            "additionalProperties": false,
        })
    }

    /// `{ "<tag>": "<name>", ..fields }`: a struct variant's fields, or a
    /// newtype variant's inner struct's, with the tag first.
    fn tagged_variant(&mut self, tag: &str, variant: &facet::Variant) -> Value {
        let name = variant.effective_name();
        let mut properties = Map::new();
        properties.insert(tag.into(), json!({ "const": name }));
        let mut required = vec![Value::from(tag)];
        let data = &variant.data;
        let fields = match data.kind {
            StructKind::Unit => &[][..],
            StructKind::Struct => data.fields,
            StructKind::TupleStruct | StructKind::Tuple => match data.fields[0].shape().ty {
                Type::User(UserType::Struct(st)) if data.fields.len() == 1 => st.fields,
                _ => unreachable!("a tagged newtype variant holds a struct"),
            },
        };
        self.collect(fields, &mut properties, &mut required);
        object_of(properties, required)
    }
}

fn object_of(properties: Map<String, Value>, required: Vec<Value>) -> Value {
    let mut schema = json!({ "type": "object", "properties": properties });
    if !required.is_empty() {
        schema["required"] = Value::Array(required);
    }
    schema["additionalProperties"] = false.into();
    schema
}

fn reference(name: &str) -> Value {
    json!({ "$ref": format!("#/$defs/{name}") })
}

fn scalar(type_identifier: &str) -> Value {
    match type_identifier {
        "String" | "str" | "&str" | "Cow" | "char" => json!({ "type": "string" }),
        "bool" => json!({ "type": "boolean" }),
        "u8" | "u16" | "u32" | "u64" | "u128" | "usize" => {
            json!({ "type": "integer", "minimum": 0 })
        }
        "i8" | "i16" | "i32" | "i64" | "i128" | "isize" => json!({ "type": "integer" }),
        "f32" | "f64" => json!({ "type": "number" }),
        _ => json!({}),
    }
}

/// Add a type's `title` and doc `description`.
fn describe(schema: &mut Value, title: &str, docs: &[&str]) {
    let Value::Object(map) = schema else { return };
    map.insert("title".into(), title.into());
    if let Some(doc) = doc_text(docs) {
        map.insert("description".into(), doc.into());
    }
}
