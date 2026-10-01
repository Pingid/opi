//! Build-only ways to write a [`Config`] out.

use anyhow::{Result, anyhow};
use facet::Facet;
use facet_json_schema::{AdditionalProperties, JsonSchema, SchemaType};

use super::config::Config;
use super::fmt;

impl Config {
    /// This config as TOML, with doc comments from the Rust docs. Loading the
    /// output gives back an equal config.
    pub fn to_toml(&self) -> Result<String> {
        fmt::to_string(self, &fmt::SerializeOptions::default()).map_err(|e| anyhow!("{e}"))
    }

    /// This config as JSON, pointing editors at `schema` (e.g.
    /// `"./schema.json"`) with a leading `"$schema"` key. Unset options are
    /// written as `null` rather than left out, so every key is listed.
    pub fn to_json(&self, schema: &str) -> Result<String> {
        let json =
            facet_json::to_string_with_options(self, &json_options()).map_err(|e| anyhow!("{e}"))?;
        let body = json.strip_prefix("{\n").ok_or_else(|| anyhow!("config JSON isn't an object"))?;
        let schema = facet_json::to_string(schema).map_err(|e| anyhow!("{e}"))?;
        Ok(format!("{{\n  \"$schema\": {schema},\n{body}"))
    }

    /// JSON Schema for config files, e.g. for editor completion.
    pub fn json_schema() -> Result<String> {
        let mut schema = facet_json_schema::schema_for::<Self>();
        fix_schema(&mut schema);
        // Every key is optional (container `#[facet(default)]`), but the
        // generator only looks at field-level defaults.
        if Self::SHAPE.has_default_attr() {
            schema.required = None;
        }
        schema.schema = Some("https://json-schema.org/draft/2020-12/schema".into());
        // Config files may point at the schema themselves; without this,
        // `additionalProperties: false` would reject that key.
        schema.properties.get_or_insert_default().insert(
            "$schema".into(),
            JsonSchema {
                type_: Some(SchemaType::String.into()),
                description: Some("Path or URL of this schema, for editors.".into()),
                ..JsonSchema::default()
            },
        );
        facet_json::to_string_with_options(&schema, &json_options()).map_err(|e| anyhow!("{e}"))
    }
}

fn json_options() -> facet_json::SerializeOptions {
    facet_json::SerializeOptions::default().indent("  ")
}

/// facet-json-schema ignores `proxy` and `untagged`, which the config types
/// use to read as plain strings / lists. Rewrites those nodes to describe
/// what config files actually contain.
fn fix_schema(schema: &mut JsonSchema) {
    // `#[facet(proxy = String)]`: its source string.
    if schema.title.as_deref() == Some("Template") {
        *schema = JsonSchema {
            type_: Some(SchemaType::String.into()),
            title: schema.title.take(),
            description: schema.description.take(),
            ..JsonSchema::default()
        };
        return;
    }

    // Children first, so the rewrites below see fixed children and aren't
    // revisited.
    let children = schema.properties.iter_mut().flat_map(|p| p.values_mut());
    let children = children.chain(schema.defs.iter_mut().flat_map(|d| d.values_mut()));
    let children = children.chain(schema.items.as_deref_mut());
    let children = children.chain(schema.any_of.iter_mut().flatten());
    let children = children.chain(schema.one_of.iter_mut().flatten());
    let children = children.chain(schema.all_of.iter_mut().flatten());
    let children = children.chain(match &mut schema.additional_properties {
        Some(AdditionalProperties::Schema(s)) => Some(s.as_mut()),
        _ => None,
    });
    for child in children {
        fix_schema(child);
    }

    // `#[facet(untagged)]`: `x` or `[x, ..]`, not `{"One": x}` / `{"Many": [..]}`.
    if schema.title.as_deref() == Some("List")
        && let Some(variants) = schema.one_of.take()
    {
        schema.any_of = Some(variants.into_iter().flat_map(untag).collect());
    }

    // `Templates` (proxied through `List<String>`) comes out as an array of
    // `Template`; a single string is fine too.
    if schema.items.as_ref().is_some_and(|i| i.title.as_deref() == Some("Template")) {
        let description = schema.description.take();
        let mut array = std::mem::take(schema);
        if let Some(items) = &mut array.items {
            items.description = None; // the Rust type's docs, not the field's
        }
        *schema = JsonSchema {
            any_of: Some(vec![
                JsonSchema { type_: Some(SchemaType::String.into()), ..JsonSchema::default() },
                array,
            ]),
            description,
            ..JsonSchema::default()
        };
    }
}

/// `{"Variant": x}` -> `x`.
fn untag(variant: JsonSchema) -> Option<JsonSchema> {
    let mut properties = variant.properties?;
    let (_, inner) = properties.pop_first()?;
    properties.is_empty().then_some(inner)
}
