//! `where` selectors: the one selection language in the config, used by
//! `filter`, `emit[].where`, `operation.overrides[].where`,
//! `schemas.keep` and value-level `{ ref, where }`.
//!
//! Fields are ANDed; a list within a field is ORed; `any` is an explicit OR of
//! sub-selectors and `not` negates one. An empty selector matches everything.
//!
//! ```yaml
//! where:
//!   method: [GET, HEAD]
//!   path: "/repos/**"
//!   request: { content_type: application/json }
//!   response: { status: 2xx }
//!   not: { operation_id: getLegacy }
//!   any: [{ tag: billing }, { path: "/admin/**" }]
//! ```
//!
//! Which fields apply depends on what's being selected ([`Scope`]); config
//! validation rejects the rest.

mod field;
mod glob;
mod matching;
mod status;
#[cfg(test)]
mod tests;

use serde::{Deserialize, Serialize};

pub use crate::list::List;
pub use field::Scope;
pub use glob::{path_matches, wildcard};
pub use matching::Subject;
pub use status::StatusPattern;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
#[cfg_attr(
    feature = "facet",
    derive(facet::Facet),
    facet(default, deny_unknown_fields)
)]
pub struct Where {
    /// `"GET"` / `["GET", "HEAD"]`, case-insensitive.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "facet", facet(skip_serializing_if = Option::is_none))]
    pub method: Option<List<String>>,
    /// Path globs: `*` matches within one segment, `**` any number of
    /// segments. `/shop/**`, `**/items/*`, `/v*/users`.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "facet", facet(skip_serializing_if = Option::is_none))]
    pub path: Option<List<String>>,
    /// Matches if the operation has any of these tags (`*` wildcards).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "facet", facet(skip_serializing_if = Option::is_none))]
    pub tag: Option<List<String>>,
    /// `*` wildcards.
    #[serde(alias = "operationId", skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "facet", facet(skip_serializing_if = Option::is_none))]
    pub operation_id: Option<List<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "facet", facet(skip_serializing_if = Option::is_none))]
    pub deprecated: Option<bool>,
    /// Matches if any request variant (one per body content type, plus "no
    /// body" when it's optional) matches.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "facet", facet(recursive_type, skip_serializing_if = Option::is_none))]
    pub request: Option<Box<Where>>,
    /// Matches if any response (one per status and content type) matches.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "facet", facet(recursive_type, skip_serializing_if = Option::is_none))]
    pub response: Option<Box<Where>>,
    /// Request / response media type, `*` wildcards: `application/json`,
    /// `"*json*"`. `"*"` matches any body, so `request: { content_type: "*" }`
    /// selects operations that take one.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "facet", facet(skip_serializing_if = Option::is_none))]
    pub content_type: Option<List<String>>,
    /// Response status: `200`, `2xx`, `default`.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "facet", facet(skip_serializing_if = Option::is_none))]
    pub status: Option<List<StatusPattern>>,
    /// Schema name (`components.schemas` key), `*` wildcards.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "facet", facet(skip_serializing_if = Option::is_none))]
    pub name: Option<List<String>>,
    /// Matches what the nested selector doesn't.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "facet", facet(recursive_type, skip_serializing_if = Option::is_none))]
    pub not: Option<Box<Where>>,
    /// Matches what any of the nested selectors does.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "facet", facet(recursive_type, skip_serializing_if = Option::is_none))]
    pub any: Option<Vec<Where>>,
}

impl Where {
    pub fn matches(&self, subject: &(impl Subject + ?Sized)) -> bool {
        self.mismatch(subject).is_none()
    }
}
