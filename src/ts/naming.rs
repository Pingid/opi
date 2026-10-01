//! Turning IR names into unique, valid TS identifiers.

use std::collections::{HashMap, HashSet};

pub use crate::case::{pascal_case, path_pascal_case};
use crate::config::DEFAULT_OPERATION_NAME;
use crate::ir::Operation;
use crate::template::Template;

/// Globals that generated types reference (or that users will expect to
/// still work), so a schema called `Error` becomes `Error2` rather than
/// shadowing them.
const RESERVED: &[&str] = &[
    "Array", "Blob", "Boolean", "Date", "Error", "File", "Map", "Number", "Object", "Omit",
    "Partial", "Pick", "Promise", "Readonly", "Record", "Set", "String",
];

/// All top-level names in the output, allocated up front so that schema refs
/// can be emitted before (or regardless of whether) the target is emitted.
#[derive(Debug, Default)]
pub struct Names {
    used: HashSet<String>,
    /// Raw `components.schemas` key -> TS identifier.
    schemas: HashMap<String, String>,
    /// Index into the generated operations -> TS identifier.
    operations: Vec<String>,
}

impl Names {
    /// `reserved` names are claimed first, verbatim: they're names the config
    /// asked for explicitly (`[[emit]]` names, format types), so schemas and
    /// operations get suffixed rather than them.
    /// Only `schemas` (raw names) get names, so schemas that aren't
    /// generated can't claim one and push a generated one to `Foo2`.
    pub fn new(schemas: &[&str], ops: &[&Operation], template: &Template, reserved: &[&str]) -> Self {
        let mut names = Self::default();
        names.used.extend(RESERVED.iter().chain(reserved).map(|s| s.to_string()));
        for &raw in schemas {
            let name = names.reserve(pascal_case(raw));
            names.schemas.insert(raw.to_string(), name);
        }
        let fallback = Template::parse(DEFAULT_OPERATION_NAME).expect("valid");
        for op in ops {
            let name = operation_name(op, template)
                .or_else(|| operation_name(op, &fallback))
                .expect("{Method}{Path} always renders");
            let name = names.reserve(name);
            names.operations.push(name);
        }
        names
    }

    pub fn schema(&self, raw: &str) -> Option<&str> {
        self.schemas.get(raw).map(String::as_str)
    }

    pub fn operation(&self, index: usize) -> &str {
        &self.operations[index]
    }

    /// Claim `name`, suffixing `2`, `3`, ... on collision.
    fn reserve(&mut self, name: String) -> String {
        let mut candidate = name.clone();
        let mut n = 1;
        while !self.used.insert(candidate.clone()) {
            n += 1;
            candidate = format!("{name}{n}");
        }
        candidate
    }
}

/// Render an operation's type name, `None` if the template uses a variable
/// the operation doesn't have.
pub fn operation_name(op: &Operation, template: &Template) -> Option<String> {
    template
        .render(|var| op_var(op, var, op.tags.first().map(String::as_str), None))
        .map(|name| identifier(&name))
}

/// Template variables for one operation. `tag` is the group's tag when
/// grouping by tag, otherwise the operation's first tag; `type_name` is only
/// known once names are allocated.
pub fn op_var(op: &Operation, var: &str, tag: Option<&str>, type_name: Option<&str>) -> Option<String> {
    match var {
        "method" => Some(op.method.as_str().to_ascii_lowercase()),
        "path" => Some(op.path.clone()),
        "operationId" => op.id.clone(),
        "tag" => tag.map(str::to_string),
        "type" => type_name.map(str::to_string),
        _ => None,
    }
}

/// Force a rendered name into a valid identifier: invalid characters become
/// `_`, and a leading digit gets a `_` prefix.
pub fn identifier(s: &str) -> String {
    let mut out: String = s
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '_' || c == '$' { c } else { '_' })
        .collect();
    if out.is_empty() || out.starts_with(|c: char| c.is_ascii_digit()) {
        out.insert(0, '_');
    }
    out
}
