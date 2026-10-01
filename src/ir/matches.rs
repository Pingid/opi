//! Matching config selectors against operations. Kept out of `select.rs` /
//! `config` so those stay includable by `build.rs`.

use crate::config::Filter;
use crate::select::{List, Selector, path_matches};

use super::Operation;

impl Selector {
    pub fn matches(&self, op: &Operation) -> bool {
        any(&self.method, |m| m.eq_ignore_ascii_case(op.method.as_str()))
            && any(&self.path, |glob| path_matches(glob, &op.path))
            && any(&self.tag, |tag| op.tags.contains(tag))
            && any(&self.operation_id, |id| op.id.as_ref() == Some(id))
            && self.deprecated.is_none_or(|d| d == op.deprecated)
            && self.has_body.is_none_or(|b| b == op.body.is_some())
            && self.not.as_ref().is_none_or(|not| !not.matches(op))
    }
}

impl Filter {
    pub fn allows(&self, op: &Operation) -> bool {
        self.include.as_ref().is_none_or(|s| s.matches(op))
            && !self.exclude.as_ref().is_some_and(|s| s.matches(op))
    }
}

/// `None` (field not given) matches everything.
fn any<T>(list: &Option<List<T>>, f: impl Fn(&T) -> bool) -> bool {
    list.as_ref().is_none_or(|l| l.as_slice().iter().any(f))
}
