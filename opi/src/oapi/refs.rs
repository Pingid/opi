//! `$ref` resolution for the non-schema component kinds.
//!
//! Schema refs are *not* resolved here: they're kept as
//! [`crate::ir::SchemaKind::Ref`] so they can be emitted as named types.
//! Everything else (parameters, request bodies, responses, path items) is
//! inlined, since the TS output has no use for them as standalone names.

use anyhow::{Result, bail};
use indexmap::IndexMap;

use super::model::{Components, Parameter, PathItem, RequestBody, Response};

/// Refs are allowed to chain (`A -> B -> C`); cap it so a cycle can't hang us.
const MAX_DEPTH: usize = 32;

pub const SCHEMA_PREFIX: &str = "#/components/schemas/";

/// A component kind that can be `$ref`erenced.
pub trait Referable: Sized {
    const PREFIX: &'static str;
    fn reference(&self) -> Option<&str>;
    fn components(components: &Components) -> &IndexMap<String, Self>;
}

macro_rules! referable {
    ($ty:ty, $prefix:literal, $field:ident) => {
        impl Referable for $ty {
            const PREFIX: &'static str = $prefix;
            fn reference(&self) -> Option<&str> {
                self.reference.as_deref()
            }
            fn components(components: &Components) -> &IndexMap<String, Self> {
                &components.$field
            }
        }
    };
}

referable!(Parameter, "#/components/parameters/", parameters);
referable!(RequestBody, "#/components/requestBodies/", request_bodies);
referable!(Response, "#/components/responses/", responses);
referable!(PathItem, "#/components/pathItems/", path_items);

/// Follows `item`'s `$ref` chain to the object it names. Siblings of a
/// `$ref` (e.g. 3.1's `summary` / `description` overrides) are ignored.
pub fn resolve<'s, T: Referable>(components: &'s Components, mut item: &'s T) -> Result<&'s T> {
    for _ in 0..MAX_DEPTH {
        let Some(reference) = item.reference() else {
            return Ok(item);
        };
        let Some(name) = component_name(reference, T::PREFIX) else {
            bail!("unsupported ref {reference} (expected {}*)", T::PREFIX);
        };
        let Some(next) = T::components(components).get(&name) else {
            bail!("dangling ref {reference}");
        };
        item = next;
    }
    bail!("ref chain too deep (cycle?) under {}", T::PREFIX)
}

/// The component a `<prefix><name>` ref names, unescaped; `None` for any
/// other ref.
pub fn component_name(reference: &str, prefix: &str) -> Option<String> {
    reference.strip_prefix(prefix).map(unescape)
}

/// JSON Pointer escapes in a ref segment: `~1` -> `/`, `~0` -> `~`.
fn unescape(segment: &str) -> String {
    segment.replace("~1", "/").replace("~0", "~")
}
