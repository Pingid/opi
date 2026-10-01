//! OpenAPI 3.0 / 3.1 / 3.2 frontend: load a spec and lower it into the
//! [`crate::ir`].

mod model;
mod operation;
mod refs;
mod schema;

use std::path::Path;

use anyhow::{Context, Result, bail};

pub use model::Document;

use crate::ir::{self, Method};

#[derive(Debug)]
pub struct Spec {
    document: Document,
}

impl Spec {
    /// Rejects anything that isn't OpenAPI 3.x.
    pub fn new(document: Document) -> Result<Self> {
        match (&document.openapi, &document.swagger) {
            (Some(v), _) if v.starts_with("3.") => Ok(Self { document }),
            (Some(v), _) => bail!("unsupported OpenAPI version {v} (expected 3.x)"),
            (None, Some(v)) => bail!("Swagger {v} isn't supported; convert it to OpenAPI 3 first"),
            (None, None) => bail!("not an OpenAPI document (no `openapi` version)"),
        }
    }

    pub fn from_json(json: &str) -> Result<Self> {
        Self::new(serde_json::from_str(json)?)
    }

    pub fn from_yaml(yaml: &str) -> Result<Self> {
        Self::new(serde_yaml::from_str(yaml)?)
    }

    pub fn from_file(path: &Path) -> Result<Self> {
        let text =
            std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        let ext = path.extension().unwrap_or_default();
        let spec = if ext == "json" {
            Self::from_json(&text)
        } else if ext == "yaml" || ext == "yml" {
            Self::from_yaml(&text)
        } else {
            bail!("Unsupported file extension: {}", ext.to_string_lossy());
        };
        spec.with_context(|| format!("in {}", path.display()))
    }

    pub fn document(&self) -> &Document {
        &self.document
    }

    /// Lower the spec into the intermediate representation.
    pub fn lower(&self) -> Result<ir::Api> {
        let components = &self.document.components;

        let schemas = components
            .schemas
            .iter()
            .map(|(name, schema)| (name.clone(), schema::lower(schema)))
            .collect();

        let mut operations = Vec::new();
        for (path, item) in &self.document.paths {
            let item = refs::resolve(components, item)
                .with_context(|| format!("resolving path item {path}"))?;
            for (method, op) in operations_of(item) {
                let context = || format!("lowering {} {path}", method.as_str());
                let op = operation::lower(components, path, method.clone(), &item.parameters, op)
                    .with_context(context)?;
                operations.push(op);
            }
        }

        Ok(ir::Api {
            schemas,
            operations,
        })
    }
}

/// The fixed methods in a stable order (the one `openapiv3` used), then 3.2's
/// `additionalOperations` in spec order.
fn operations_of(item: &model::PathItem) -> impl Iterator<Item = (Method, &model::Operation)> {
    let fixed = [
        (Method::Get, &item.get),
        (Method::Put, &item.put),
        (Method::Post, &item.post),
        (Method::Delete, &item.delete),
        (Method::Options, &item.options),
        (Method::Head, &item.head),
        (Method::Patch, &item.patch),
        (Method::Trace, &item.trace),
        (Method::Query, &item.query),
    ];
    let fixed = fixed.into_iter().filter_map(|(method, op)| Some((method, op.as_ref()?)));
    let additional = item
        .additional_operations
        .iter()
        .map(|(name, op)| (Method::parse(name).unwrap_or_else(|| Method::Other(name.clone())), op));
    fixed.chain(additional)
}
