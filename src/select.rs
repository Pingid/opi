//! Operation selectors: the `match = { ... }` tables in the config.
//!
//! Fields are ANDed; a list within a field is ORed; `not` negates a nested
//! selector. An empty selector matches everything.
//!
//! ```toml
//! match = { method = ["GET", "HEAD"], path = "/shop/**", not = { deprecated = true } }
//! ```

use anyhow::{Result, bail};
use facet::Facet;

use crate::ir::Method;

#[derive(Debug, Clone, Default, PartialEq, Facet)]
#[facet(default, deny_unknown_fields)]
pub struct Selector {
    /// `"GET"` / `["GET", "HEAD"]`, case-insensitive.
    pub method: Option<List<String>>,
    /// Path globs: `*` matches within one segment, `**` any number of
    /// segments. `/shop/**`, `**/items/*`, `/v*/users`.
    pub path: Option<List<String>>,
    /// Matches if the operation has any of these tags.
    pub tag: Option<List<String>>,
    #[facet(alias = "operationId")]
    pub operation_id: Option<List<String>>,
    pub deprecated: Option<bool>,
    pub has_body: Option<bool>,
    #[facet(recursive_type)]
    pub not: Option<Box<Selector>>,
}

impl Selector {
    /// Matches everything.
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }

    /// Catch typos that would otherwise silently match nothing.
    pub fn validate(&self) -> Result<()> {
        for method in self.method.iter().flat_map(List::as_slice) {
            if Method::parse(method).is_none() {
                bail!("unknown method {method:?} in match");
            }
        }
        if let Some(not) = &self.not {
            not.validate()?;
        }
        Ok(())
    }
}

/// A value that can be written as `x` or `[x, y]`.
#[derive(Debug, Clone, Facet)]
#[facet(untagged)]
#[repr(u8)]
pub enum List<T> {
    One(T),
    Many(Vec<T>),
}

impl<T> List<T> {
    pub fn as_slice(&self) -> &[T] {
        match self {
            List::One(v) => std::slice::from_ref(v),
            List::Many(v) => v,
        }
    }
}

impl<T> Default for List<T> {
    fn default() -> Self {
        List::Many(Vec::new())
    }
}

/// `"x"` and `["x"]` are the same list.
impl<T: PartialEq> PartialEq for List<T> {
    fn eq(&self, other: &Self) -> bool {
        self.as_slice() == other.as_slice()
    }
}

impl<T> From<Vec<T>> for List<T> {
    fn from(v: Vec<T>) -> Self {
        List::Many(v)
    }
}

pub fn path_matches(glob: &str, path: &str) -> bool {
    let glob: Vec<_> = glob.split('/').collect();
    let path: Vec<_> = path.split('/').collect();
    segments_match(&glob, &path)
}

fn segments_match(glob: &[&str], path: &[&str]) -> bool {
    match glob.split_first() {
        None => path.is_empty(),
        Some((&"**", rest)) => (0..=path.len()).any(|i| segments_match(rest, &path[i..])),
        Some((segment, rest)) => match path.split_first() {
            Some((first, path)) => wildcard(segment, first) && segments_match(rest, path),
            None => false,
        },
    }
}

/// `*` matches any run of characters within a segment.
pub fn wildcard(pattern: &str, s: &str) -> bool {
    match pattern.split_once('*') {
        None => pattern == s,
        Some((prefix, rest)) => {
            let Some(s) = s.strip_prefix(prefix) else {
                return false;
            };
            (0..=s.len())
                .filter(|&i| s.is_char_boundary(i))
                .any(|i| wildcard(rest, &s[i..]))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::path_matches;

    #[test]
    fn globs() {
        assert!(path_matches("/shop/**", "/shop/items/{id}"));
        assert!(path_matches("/shop/**", "/shop"));
        assert!(path_matches("**/shop/**", "/api/shop/items"));
        assert!(path_matches("/shop/*", "/shop/items"));
        assert!(!path_matches("/shop/*", "/shop/items/{id}"));
        assert!(path_matches("/v*/users", "/v2/users"));
        assert!(path_matches("/items/{id}", "/items/{id}"));
        assert!(!path_matches("/shop/**", "/shopping"));
    }
}
