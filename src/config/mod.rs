//! Generator configuration, loaded from YAML or JSON with serde. The doc
//! comments here are also the config file's documentation: with the `facet`
//! feature, the `reflect` tests write [`Config::default`] out as the commented
//! `config.default.yaml`, plus JSON Schema and TypeScript types for it.
//!
//! ```yaml
//! filter: { deprecated: false }   # which operations are generated at all
//! operation:
//!   name: "{Method}{Path}"        # the per-operation type
//! emit:                           # aggregate types over operations
//!   - name: Paths
//!     shape: { "{METHOD} {path}": { request: "{request}", response: "{response}" } }
//! ```
//!
//! Every field is optional and falls back to [`Config::default`]. Lists and
//! maps *replace* the default rather than merging into it, so a config with
//! any `emit` entries only gets those. (serde gets that from the container
//! `#[serde(default)]`; the field-level `facet(default)`s only mark the
//! fields optional in the generated schema and TS types.)

mod load;
mod shape;
#[cfg(test)]
mod tests;
mod validate;

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

pub use shape::{Resolved, Root, Shape, Value};

use crate::select::Where;
use crate::template::{Template, Var};

/// opi configuration: the built-in defaults, written out. Every key is
/// optional; copy only what you want to change. Lists and maps replace the
/// default rather than merging, so defining any `emit` drops the `Paths` one
/// below.
///
/// Templates (`name`s, `shape` keys and values): the operation's fields,
/// `.` for its request's / response's.
///   `{type_name}` `{id}` `{method}` `{path}` `{summary}` `{description}`
///   `{deprecated}` `{tags}` `{request.content_type}` `{response.status}`
///   `{response.content_type}`
///   `{method}` -> get, `{Method}` -> Get, `{METHOD}` -> GET.
///   `{Path}` on "/components/{name}" -> ComponentsByName.
///   Filters: `{tags|camel}`, `{id|pascal}`, `|lower`, `|upper`.
/// Some have several values (`tags`, the content types, `response.status`):
/// a `shape` key using one is repeated per value, a name uses the first.
///
/// Selectors (`filter`, `where`): fields AND, lists OR, `any` ORs selectors,
/// `not` negates one.
///   method: [GET, HEAD]    path: "/shop/**" (* one segment, ** any)
///   tag: shop    operation_id: "get*"    deprecated: false
///   request: { content_type: application/json }    response: { status: 2xx }
///   not: { ... }    any: [{ ... }, { ... }]
///
/// Shapes mirror the output: a map is an object type, a value a union.
///   shape: { "{METHOD} {path}": "{type_name}" }    -> type A = { ... }
///   shape: "{type_name}"                           -> type A = GetFoo | GetBar
/// Values reference the generated types; a variable as the whole value is
/// that part of the operation's type:
///   "{type_name}" -> GetFoo    "{method}" -> GetFoo['method']
///   "{request}" -> GetFoo['request']    "{request.body}" -> GetFoo['request']['body']
///   "{response}"    "{response.status}"    "{response.body}"    ...
///   { ref: "{response}", where: { content_type: application/json } }
///     -> Extract<GetFoo['response'], { contentType: 'application/json' }>
///   { ref: "{request}", pick: [body, query] }
///     -> Pick<GetFoo['request'], 'body' | 'query'>
///   any other template -> a string literal type ("{METHOD}" -> 'GET')
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
#[cfg_attr(
    feature = "facet",
    derive(facet::Facet),
    facet(default, deny_unknown_fields)
)]
#[allow(rustdoc::broken_intra_doc_links)] // `GetFoo['request']['body']` reads as a link
pub struct Config {
    /// Path or URL of this file's JSON Schema, for editors. Ignored.
    #[serde(rename = "$schema", skip_serializing_if = "Option::is_none")]
    #[cfg_attr(
        feature = "facet",
        facet(rename = "$schema", skip_serializing_if = Option::is_none)
    )]
    pub(crate) schema: Option<String>,
    /// Prepended verbatim to the output. `""` for none.
    #[cfg_attr(feature = "facet", facet(default = Config::default().header))]
    pub(crate) header: String,
    /// Emit `/** @description ... */` comments.
    #[cfg_attr(feature = "facet", facet(default = true))]
    pub(crate) jsdoc: bool,
    /// Emit `readonly` for `readOnly: true` properties.
    #[cfg_attr(feature = "facet", facet(default = true))]
    pub(crate) readonly: bool,
    /// `format` -> TS type to use instead of the base type, e.g.
    /// `binary: Blob`, `date-time: Date`. Applies to string/number/integer.
    #[cfg_attr(feature = "facet", facet(default = Config::default().formats))]
    pub(crate) formats: IndexMap<String, String>,
    /// Which operations are generated, as a selector (`{}` is all of them).
    /// Applied before everything else, so operations it leaves out don't get
    /// a type or appear in any `emit`.
    #[cfg_attr(feature = "facet", facet(default))]
    pub(crate) filter: Where,
    #[cfg_attr(feature = "facet", facet(default))]
    pub(crate) schemas: SchemasConfig,
    #[cfg_attr(feature = "facet", facet(default))]
    pub(crate) operation: OperationConfig,
    #[cfg_attr(feature = "facet", facet(default = Config::default().emit))]
    pub(crate) emit: Vec<Emit>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            schema: None,
            header: "// This file is generated by opi. Do not edit.".to_string(),
            jsdoc: true,
            readonly: true,
            formats: IndexMap::from([("binary".to_string(), "Blob".to_string())]),
            filter: Where::default(),
            schemas: SchemasConfig::default(),
            operation: OperationConfig::default(),
            emit: vec![Emit {
                name: template("Paths"),
                description: None,
                filter: None,
                group_by: None,
                shape: Shape::Map(IndexMap::from([(
                    "{METHOD} {path}".to_string(),
                    Shape::Map(IndexMap::from([
                        (
                            "request".to_string(),
                            Shape::Value(Value::Template(template("{request}"))),
                        ),
                        (
                            "response".to_string(),
                            Shape::Value(Value::Template(template("{response}"))),
                        ),
                    ])),
                )])),
            }],
        }
    }
}

/// Which named schemas (`components.schemas`) are generated.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
#[cfg_attr(
    feature = "facet",
    derive(facet::Facet),
    facet(default, deny_unknown_fields)
)]
pub struct SchemasConfig {
    /// `referenced`: only schemas the generated operations use, directly or
    /// through other schemas. `all`: every schema in the spec.
    #[cfg_attr(feature = "facet", facet(default = SchemaEmit::Referenced))]
    pub emit: SchemaEmit,
    /// Schemas to generate even when unreferenced, as a selector on `name`
    /// (`{ name: ["Mix*", Error] }`), e.g. types only used by client code.
    /// Whatever they reference is kept too.
    #[cfg_attr(feature = "facet", facet(default))]
    pub keep: Option<Where>,
}

impl Default for SchemasConfig {
    fn default() -> Self {
        Self {
            emit: SchemaEmit::Referenced,
            keep: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(
    feature = "facet",
    derive(facet::Facet),
    facet(rename_all = "snake_case")
)]
#[repr(u8)]
pub enum SchemaEmit {
    Referenced,
    All,
}

/// The `export type GetFoo = { method, path, request, response }` generated
/// for each operation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
#[cfg_attr(
    feature = "facet",
    derive(facet::Facet),
    facet(default, deny_unknown_fields)
)]
pub struct OperationConfig {
    /// Type name template, e.g. `"{operationId|pascal}"`. Operations missing
    /// a variable it uses (no `operationId`) fall back to `"{Method}{Path}"`.
    /// Two operations with the same name are an error; rename one with an
    /// override.
    #[cfg_attr(feature = "facet", facet(default = template(DEFAULT_OPERATION_NAME)))]
    pub name: Template,
    /// Names for particular operations, tried in order before `name`:
    /// `- { where: { operation_id: "repos/list-for-org" }, name: ListOrgRepos }`
    #[cfg_attr(feature = "facet", facet(default))]
    pub overrides: Vec<Override>,
}

impl Default for OperationConfig {
    fn default() -> Self {
        Self {
            name: template(DEFAULT_OPERATION_NAME),
            overrides: Vec::new(),
        }
    }
}

pub const DEFAULT_OPERATION_NAME: &str = "{Method}{Path}";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "facet", derive(facet::Facet), facet(deny_unknown_fields))]
pub struct Override {
    /// The operations this names.
    #[serde(rename = "where")]
    #[cfg_attr(feature = "facet", facet(rename = "where"))]
    pub filter: Where,
    /// Name template, as for `operation.name`.
    pub name: Template,
}

/// An aggregate type over operations: `shape` is its layout, with keys and
/// values rendered per operation.
///
///   - name: "{tags|pascal}Routes"          # one type per tag
///     group_by: tags
///     where: { method: GET }
///     shape: { "{path}": { request: "{request}", response: "{response}" } }
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "facet", derive(facet::Facet), facet(deny_unknown_fields))]
pub struct Emit {
    /// Type name. Uses the `group_by` variable when grouped.
    pub name: Template,
    /// JSDoc for the emitted type.
    #[serde(default)]
    #[cfg_attr(feature = "facet", facet(default))]
    pub description: Option<String>,
    /// Which operations this covers (a selector, as in `filter`). All of
    /// them if unset.
    #[serde(rename = "where", default)]
    #[cfg_attr(feature = "facet", facet(rename = "where", default))]
    pub filter: Option<Where>,
    /// A variable to emit one type per distinct value of: `tags`, `method`,
    /// `request.content_type`, ... An operation with several values (tags)
    /// appears in each of their groups; one with none is skipped.
    #[serde(default)]
    #[cfg_attr(feature = "facet", facet(default))]
    pub group_by: Option<String>,
    /// The type's layout: a map for an object type, a value for a union.
    pub shape: Shape,
}

impl Emit {
    /// The variable to group on, if any. Before validation, also `None` for
    /// a `group_by` that isn't a [`Var::NAME`].
    pub fn group_var(&self) -> Option<Var> {
        let name = self.group_by.as_deref()?;
        Var::NAME.iter().find(|v| v.name() == name).copied()
    }
}

fn template(source: &str) -> Template {
    Template::parse(source).expect("built-in template is valid")
}
