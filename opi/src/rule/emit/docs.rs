//! JSDoc lines: `@description`, `@deprecated`, ... as plain strings.

use crate::ast::{Meta, Param, Route};

#[derive(Default)]
pub(super) struct DocLines(Vec<String>);

impl DocLines {
    /// A schema's `@title`, `@description`, `@contentMediaType` and
    /// `@deprecated`.
    pub(super) fn schema(meta: &Meta) -> Self {
        Self::default()
            .tag("title", meta.title.as_deref())
            .tag("description", meta.description.as_deref())
            .tag("contentMediaType", meta.encoded.as_deref())
            .flag("deprecated", meta.deprecated)
    }

    /// A route's `@summary`, `@description` and `@deprecated`: for the
    /// declaration it's emitted as.
    pub(super) fn route(route: &Route) -> Self {
        Self::default()
            .tag("summary", route.summary.as_deref())
            .tag("description", route.description.as_deref())
            .flag("deprecated", route.deprecated())
    }

    /// `@summary` and `@deprecated` only: for a key that stands for the
    /// route inside a bigger type.
    pub(super) fn route_brief(route: &Route) -> Self {
        Self::default()
            .tag("summary", route.summary.as_deref())
            .flag("deprecated", route.deprecated())
    }

    /// A parameter's `@description` and `@deprecated`.
    pub(super) fn param(param: &Param) -> Self {
        Self::default()
            .tag("description", param.description.as_deref())
            .flag("deprecated", param.deprecated)
    }

    /// `@tag text`, continued over the text's later lines. Nothing for a
    /// missing or blank text.
    pub(super) fn tag(mut self, tag: &str, text: Option<&str>) -> Self {
        let Some(text) = text.map(str::trim).filter(|t| !t.is_empty()) else {
            return self;
        };
        let mut lines = text.lines();
        if let Some(first) = lines.next() {
            self.0.push(format!("@{tag} {}", first.trim_end()));
        }
        self.0.extend(lines.map(|l| l.trim_end().to_string()));
        self
    }

    /// `@tag` when `set`.
    pub(super) fn flag(mut self, tag: &str, set: bool) -> Self {
        if set {
            self.0.push(format!("@{tag}"));
        }
        self
    }

    pub(super) fn done(self) -> Vec<String> {
        self.0
    }
}
