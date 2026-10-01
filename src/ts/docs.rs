//! JSDoc lines for the emitters.

use crate::ir::{Meta, Operation, Param, Schema};

/// Build JSDoc lines. Kept as plain strings so any emitter can add its own
/// tags (`@summary`, `@deprecated`, ...).
#[derive(Default)]
pub(super) struct DocLines(Vec<String>);

impl DocLines {
    /// `@description`.
    pub(super) fn description(text: Option<&str>) -> Self {
        Self::default().tag("description", text)
    }

    /// A schema's `@title`, then its annotations.
    pub(super) fn schema(schema: &Schema) -> Self {
        Self::default()
            .tag("title", schema.meta.title.as_deref())
            .with_meta(&schema.meta)
    }

    /// `@description`, `@contentMediaType`, `@deprecated`.
    pub(super) fn meta(meta: &Meta) -> Self {
        Self::default().with_meta(meta)
    }

    /// A parameter's `@description` and `@deprecated`.
    pub(super) fn param(param: &Param) -> Self {
        Self::default()
            .tag("description", param.description.as_deref())
            .flag("deprecated", param.deprecated)
    }

    /// An operation's `@summary`, `@description` and `@deprecated`.
    pub(super) fn operation(op: &Operation) -> Self {
        Self::default()
            .tag("summary", op.summary.as_deref())
            .tag("description", op.description.as_deref())
            .flag("deprecated", op.deprecated)
    }

    /// `@summary` and `@deprecated` only: for a member that stands for the
    /// operation.
    pub(super) fn summary(op: &Operation) -> Self {
        Self::default()
            .tag("summary", op.summary.as_deref())
            .flag("deprecated", op.deprecated)
    }

    pub(super) fn tag(mut self, tag: &str, text: Option<&str>) -> Self {
        let Some(text) = text.map(str::trim).filter(|t| !t.is_empty()) else {
            return self;
        };
        let mut lines = text.lines();
        if let Some(first) = lines.next() {
            self.0.push(format!("@{tag} {first}"));
        }
        self.0.extend(lines.map(|l| l.trim_end().to_string()));
        self
    }

    pub(super) fn flag(mut self, tag: &str, set: bool) -> Self {
        if set {
            self.0.push(format!("@{tag}"));
        }
        self
    }

    fn with_meta(self, meta: &Meta) -> Self {
        self.tag("description", meta.description.as_deref())
            .tag("contentMediaType", meta.encoded.as_deref())
            .flag("deprecated", meta.deprecated)
    }

    pub(super) fn done(self) -> Vec<String> {
        self.0
    }
}
