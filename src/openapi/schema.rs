//! [`model::Schema`](super::model::Schema) -> [`ir::Schema`].
//!
//! Lowering is total: anything we don't understand becomes
//! [`SchemaKind::Unknown`] rather than an error, so a single odd schema never
//! blocks generation for the whole spec.
//!
//! Handles 3.0 and 3.1+ schemas alike: `nullable: true` and
//! `type: [.., "null"]` both become [`Meta::nullable`], and keywords that
//! 3.1+ allows side by side (`$ref` + `properties`, `type` + `oneOf`, ...)
//! are intersected rather than all but one being dropped.

use serde_json::Value;

use super::model::{Schema, SchemaOrBool};
use super::refs::{SCHEMA_PREFIX, unescape};
use crate::ir::{self, Literal, Meta, Object, Property, SchemaKind};

pub fn lower(schema: &SchemaOrBool) -> ir::Schema {
    match schema {
        SchemaOrBool::Bool(true) => ir::Schema::unknown(),
        SchemaOrBool::Bool(false) => ir::Schema::new(SchemaKind::Never),
        SchemaOrBool::Schema(schema) => lower_schema(schema),
    }
}

pub fn lower_opt(schema: Option<&SchemaOrBool>) -> ir::Schema {
    schema.map_or_else(ir::Schema::unknown, lower)
}

fn lower_schema(s: &Schema) -> ir::Schema {
    if let Some(decoded) = decoded_json(s) {
        return decoded;
    }
    let null_type = s.types.0.iter().any(|t| t == "null");
    let types: Vec<&str> = s.types.0.iter().map(String::as_str).filter(|&t| t != "null").collect();

    // Each keyword group that constrains the shape. A lone group is used as
    // is; several are intersected.
    let reference = s.reference.as_deref().map(lower_ref);
    let all_of = (!s.all_of.is_empty()).then(|| s.all_of.iter().map(lower).collect::<Vec<_>>());
    let union = s.one_of.iter().chain(&s.any_of).map(lower).collect::<Vec<_>>();
    let union = (!union.is_empty()).then(|| ir::Schema::new(SchemaKind::Union(union)));
    // A bare `type: object` next to `oneOf` etc. adds nothing the members
    // don't already say; don't intersect it in.
    let composed = reference.is_some() || all_of.is_some() || union.is_some();
    let shape = shape(s, &types)
        .filter(|_| !(composed && is_bare(s)))
        .map(ir::Schema::new);

    let groups = [reference.is_some(), shape.is_some(), all_of.is_some(), union.is_some()];
    let kind = match groups.iter().filter(|&&g| g).count() {
        // `type: "null"`, or nothing we can use (e.g. only `not`).
        0 if null_type => SchemaKind::Null,
        0 => SchemaKind::Unknown,
        1 => match (reference, shape, all_of, union) {
            (Some(one), ..) | (_, Some(one), ..) | (.., Some(one)) => one.kind,
            (_, _, Some(all_of), _) => SchemaKind::Intersection(all_of),
            _ => unreachable!("exactly one group is set"),
        },
        _ => {
            let mut members: Vec<_> = reference.into_iter().chain(shape).collect();
            members.extend(all_of.into_iter().flatten());
            members.extend(union);
            // `X & unknown` is `X`. Validation-only members (e.g.
            // `anyOf: [{ required: [a] }, ..]`) lower to `unknown`.
            members.retain(|m| !is_unknown(m));
            match members.len() {
                0 => SchemaKind::Unknown,
                1 if members[0].meta == Meta::default() => members.remove(0).kind,
                _ => SchemaKind::Intersection(members),
            }
        }
    };

    let meta = Meta {
        title: s.title.clone(),
        description: s.description.clone(),
        deprecated: s.deprecated,
        // `type: ["null"]` alone is `Null`, not "nullable nothing".
        nullable: s.nullable || (null_type && kind != SchemaKind::Null),
        read_only: s.read_only,
        write_only: s.write_only,
        encoded: None,
    };
    ir::Schema::new(kind).with_meta(meta)
}

/// A JSON string with a `contentSchema` (e.g. a server-sent event's `data`):
/// typed as the decoded value, flagged with [`Meta::encoded`] so the docs say
/// it arrives as a string. Other media types, or an extra `contentEncoding`
/// (base64 JSON), stay plain strings.
fn decoded_json(s: &Schema) -> Option<ir::Schema> {
    let media_type = s.content_media_type.as_deref()?;
    let content_schema = s.content_schema.as_ref()?;
    let is_string = s.types.0.iter().all(|t| t == "string" || t == "null");
    if !is_string || s.content_encoding.is_some() || !is_json(media_type) {
        return None;
    }
    let mut decoded = lower(content_schema);
    let meta = &mut decoded.meta;
    meta.encoded = Some(media_type.to_string());
    meta.nullable |= s.nullable || s.types.0.iter().any(|t| t == "null");
    meta.deprecated |= s.deprecated;
    meta.read_only |= s.read_only;
    meta.write_only |= s.write_only;
    // The property's own docs describe it better than the decoded type's.
    if s.description.is_some() {
        meta.description = s.description.clone();
    }
    if s.title.is_some() {
        meta.title = s.title.clone();
    }
    Some(decoded)
}

/// `application/json`, or a `+json` type like `application/geo+json`.
fn is_json(media_type: &str) -> bool {
    let essence = media_type.split(';').next().unwrap_or_default().trim().to_ascii_lowercase();
    essence == "application/json" || essence.ends_with("+json")
}

/// No constraint beyond the type itself.
fn is_bare(s: &Schema) -> bool {
    s.const_value.is_none()
        && s.enum_values.is_none()
        && s.properties.is_empty()
        && s.additional_properties.is_none()
        && s.items.is_none()
}

/// `unknown`, or a union with an `unknown` member (which is `unknown` too).
fn is_unknown(schema: &ir::Schema) -> bool {
    match &schema.kind {
        SchemaKind::Unknown => !schema.meta.nullable,
        SchemaKind::Union(members) => !schema.meta.nullable && members.iter().any(is_unknown),
        _ => false,
    }
}

fn lower_ref(reference: &str) -> ir::Schema {
    match reference.strip_prefix(SCHEMA_PREFIX) {
        Some(name) => ir::Schema::new(SchemaKind::Ref(unescape(name))),
        // TODO: external / non-component refs.
        None => ir::Schema::unknown(),
    }
}

/// The value-level constraints: `const` / `enum`, else `type` (or the type
/// implied by `properties` / `items` when `type` is omitted).
fn shape(s: &Schema, types: &[&str]) -> Option<SchemaKind> {
    if let Some(value) = &s.const_value {
        return Some(SchemaKind::Enum(json_literal(value).into_iter().collect()));
    }
    if let Some(values) = &s.enum_values {
        return Some(SchemaKind::Enum(values.iter().filter_map(json_literal).collect()));
    }
    match types {
        [] if !s.properties.is_empty() || s.additional_properties.is_some() => {
            Some(SchemaKind::Object(lower_object(s)))
        }
        [] if s.items.is_some() => Some(type_kind("array", s)),
        [] => None,
        [ty] => Some(type_kind(ty, s)),
        types => Some(SchemaKind::Union(
            types.iter().map(|ty| ir::Schema::new(type_kind(ty, s))).collect(),
        )),
    }
}

fn type_kind(ty: &str, s: &Schema) -> SchemaKind {
    let format = s.format.clone();
    match ty {
        "string" => SchemaKind::String { format },
        "number" => SchemaKind::Number { format },
        "integer" => SchemaKind::Integer { format },
        "boolean" => SchemaKind::Boolean,
        "null" => SchemaKind::Null,
        "array" => SchemaKind::Array(Box::new(lower_opt(s.items.as_ref()))),
        "object" => SchemaKind::Object(lower_object(s)),
        _ => SchemaKind::Unknown,
    }
}

fn lower_object(s: &Schema) -> Object {
    let properties = s
        .properties
        .iter()
        .map(|(name, schema)| {
            let property = Property {
                schema: lower(schema),
                required: s.required.contains(name),
            };
            (name.clone(), property)
        })
        .collect();

    let additional = match &s.additional_properties {
        None | Some(SchemaOrBool::Bool(false)) => None,
        Some(schema) => Some(Box::new(lower(schema))),
    };

    Object {
        properties,
        additional,
    }
}

fn json_literal(value: &Value) -> Option<Literal> {
    Some(match value {
        Value::Null => Literal::Null,
        Value::Bool(b) => Literal::Bool(*b),
        Value::Number(n) => Literal::Number(n.as_f64()?),
        Value::String(s) => Literal::String(s.clone()),
        _ => return None,
    })
}
