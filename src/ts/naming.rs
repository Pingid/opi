//! Turning IR names into unique, valid TS identifiers.

use std::collections::HashMap;
use std::fmt;

use anyhow::{Result, bail};

use crate::case::Case;
use crate::config::{DEFAULT_OPERATION_NAME, OperationConfig};
use crate::ir::Operation;
use crate::list::distinct;
use crate::template::{Template, Var};

/// Globals that generated types reference (or that users will expect to
/// still work), so a schema called `Error` becomes `Error2` rather than
/// shadowing them.
const RESERVED: &[&str] = &[
    "Array", "Blob", "Boolean", "Date", "Error", "Exclude", "Extract", "File", "Map", "Number",
    "Object", "Omit", "Partial", "Pick", "Promise", "Readonly", "Record", "Set", "String",
];

/// Who a top-level name belongs to, for collision errors.
#[derive(Debug, Clone)]
enum Owner {
    Global,
    Format,
    /// With the `emit` entry (and group) it came from.
    Emit(String),
    Schema(String),
    Operation(String),
}

impl fmt::Display for Owner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Owner::Global => f.write_str("a TS global"),
            Owner::Format => f.write_str("a `formats` type"),
            Owner::Emit(_) => f.write_str("an `emit` type"),
            Owner::Schema(raw) => write!(f, "the schema {raw:?}"),
            Owner::Operation(op) => write!(f, "operation {op}"),
        }
    }
}

/// All top-level names in the output, allocated up front so that schema refs
/// can be emitted before (or regardless of whether) the target is emitted.
#[derive(Debug, Default)]
pub struct Names {
    owners: HashMap<String, Owner>,
    /// Raw `components.schemas` key -> TS identifier.
    schemas: HashMap<String, String>,
    /// Index into the generated operations -> TS identifier.
    operations: Vec<String>,
}

impl Names {
    /// `formats` and `emits` are names the config asks for explicitly, so
    /// they're claimed first. Schema names come from the spec and can't be
    /// renamed, so a colliding one is suffixed (`Error2`); operation names
    /// can (`operation.overrides`), so a colliding one is an error. Two
    /// `emit` types with one name are an error too.
    /// Only `schemas` (raw names) get names, so schemas that aren't
    /// generated can't claim one and push a generated one to `Foo2`.
    pub fn new(
        schemas: &[&str],
        ops: &[&Operation],
        config: &OperationConfig,
        formats: &[&str],
        emits: &[(&str, String)],
    ) -> Result<Self> {
        let mut names = Self::default();
        names.claim_fixed(formats, emits)?;
        names.claim_schemas(schemas);
        names.claim_operations(ops, config)?;
        Ok(names)
    }

    /// The globals, then `formats` types, then `emit` types, which may not
    /// shadow anything claimed before them.
    fn claim_fixed(&mut self, formats: &[&str], emits: &[(&str, String)]) -> Result<()> {
        for &global in RESERVED {
            self.owners.insert(global.to_string(), Owner::Global);
        }
        for &format in formats {
            self.owners
                .entry(format.to_string())
                .or_insert(Owner::Format);
        }
        for (emit, owner) in emits {
            match self.owners.get(*emit) {
                Some(Owner::Emit(other)) => bail!("{other} and {owner} are both named `{emit}`"),
                Some(existing) => bail!("`emit` type `{emit}` would shadow {existing}; rename it"),
                None => {}
            }
            self.owners
                .insert(emit.to_string(), Owner::Emit(owner.clone()));
        }
        Ok(())
    }

    /// PascalCase, suffixed on collision.
    fn claim_schemas(&mut self, schemas: &[&str]) {
        for &raw in schemas {
            let name = self.reserve(raw.pascal_case(), Owner::Schema(raw.to_string()));
            self.schemas.insert(raw.to_string(), name);
        }
    }

    /// From the config's templates; a collision is an error.
    fn claim_operations(&mut self, ops: &[&Operation], config: &OperationConfig) -> Result<()> {
        let fallback = Template::parse(DEFAULT_OPERATION_NAME).expect("valid");
        for op in ops {
            let name = operation_name_for(op, config, &fallback);
            match self.owners.get(&name) {
                Some(Owner::Operation(other)) => bail!(
                    "operations {other} and {} are both named `{name}`; rename one with \
                     `operation.overrides`",
                    describe(op)
                ),
                Some(owner) => bail!(
                    "operation {} is named `{name}`, which is already {owner}; rename it with \
                     `operation.overrides`",
                    describe(op)
                ),
                None => {}
            }
            self.owners
                .insert(name.clone(), Owner::Operation(describe(op)));
            self.operations.push(name);
        }
        Ok(())
    }

    pub fn schema(&self, raw: &str) -> Option<&str> {
        self.schemas.get(raw).map(String::as_str)
    }

    pub fn operation(&self, index: usize) -> &str {
        &self.operations[index]
    }

    /// Claim `name`, suffixing `2`, `3`, ... on collision.
    fn reserve(&mut self, name: String, owner: Owner) -> String {
        let mut candidate = name.clone();
        let mut n = 1;
        while self.owners.contains_key(&candidate) {
            n += 1;
            candidate = format!("{name}{n}");
        }
        self.owners.insert(candidate.clone(), owner);
        candidate
    }
}

/// The first override whose `where` matches, else `operation.name`, else
/// `fallback`; each is skipped when the operation lacks a variable it uses.
fn operation_name_for(op: &Operation, config: &OperationConfig, fallback: &Template) -> String {
    let overridden = config
        .overrides
        .iter()
        .find(|o| o.filter.matches(op))
        .and_then(|o| operation_name(op, &o.name));
    overridden
        .or_else(|| operation_name(op, &config.name))
        .or_else(|| operation_name(op, fallback))
        .expect("{Method}{Path} always renders")
}

/// `` `GET /items` (listItems) ``
pub fn describe(op: &Operation) -> String {
    let id = op
        .id
        .as_ref()
        .map(|id| format!(" ({id})"))
        .unwrap_or_default();
    format!("`{} {}`{id}", op.method, op.path)
}

/// Render an operation's type name, `None` if the template uses a variable
/// the operation doesn't have.
pub fn operation_name(op: &Operation, template: &Template) -> Option<String> {
    // A name is one string, so a variable with several values gives its
    // first.
    template
        .render(|var| op_values(op, var, None).into_iter().next())
        .map(|name| Case::identifier(&name))
}

/// The values of a text variable ([`Var::TEXT`]) for `op`: none (`id`
/// without an `operationId`), one, or several (`tags`, content types,
/// statuses), in spec order. `type_name` is only known once names are
/// allocated. Reference-only variables have no text value.
pub fn op_values(op: &Operation, var: Var, type_name: Option<&str>) -> Vec<String> {
    let one = |value: Option<&str>| value.map(str::to_string).into_iter().collect();
    match var {
        Var::TypeName => one(type_name),
        Var::Id => one(op.id.as_deref()),
        Var::Method => vec![op.method.as_str().to_ascii_lowercase()],
        Var::Path => vec![op.path.clone()],
        Var::Summary => one(op.summary.as_deref()),
        Var::Description => one(op.description.as_deref()),
        Var::Deprecated => vec![op.deprecated.to_string()],
        Var::Tags => op.tags.clone(),
        Var::RequestContentType => {
            distinct(op.requests.iter().filter_map(|r| r.content_type.clone()))
        }
        Var::ResponseStatus => distinct(op.responses.iter().map(|r| r.status.to_string())),
        Var::ResponseContentType => {
            distinct(op.responses.iter().filter_map(|r| r.content_type.clone()))
        }
        Var::Request
        | Var::RequestBody
        | Var::RequestParams
        | Var::RequestQuery
        | Var::RequestHeaders
        | Var::RequestCookies
        | Var::Response
        | Var::ResponseBody => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Override;
    use crate::ir::{Method, Params, Request};
    use crate::select::Where;

    fn op(method: Method, path: &str, id: &str) -> Operation {
        Operation {
            id: Some(id.into()),
            method,
            path: path.into(),
            summary: None,
            description: None,
            deprecated: false,
            tags: Vec::new(),
            params: Params::default(),
            requests: vec![Request::bodiless()],
            responses: Vec::new(),
        }
    }

    fn names(ops: &[Operation], config: &OperationConfig, schemas: &[&str]) -> Result<Names> {
        let ops: Vec<_> = ops.iter().collect();
        Names::new(
            schemas,
            &ops,
            config,
            &["Blob"],
            &[("Paths", "emit[0]".into())],
        )
    }

    #[test]
    fn collisions() {
        let ops = [
            op(Method::Get, "/a", "getA"),
            op(Method::Get, "/a/", "getASlash"),
        ];
        let err = names(&ops, &OperationConfig::default(), &[])
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("`GET /a` (getA) and `GET /a/` (getASlash)"),
            "{err}"
        );
        assert!(err.contains("both named `GetA`"), "{err}");

        let config = OperationConfig {
            overrides: vec![Override {
                filter: serde_yaml::from_str::<Where>("{ operation_id: getASlash }").unwrap(),
                name: Template::parse("GetATrailing").unwrap(),
            }],
            ..OperationConfig::default()
        };
        let names = names(&ops, &config, &[]).unwrap();
        assert_eq!(names.operation(0), "GetA");
        assert_eq!(names.operation(1), "GetATrailing");
    }

    #[test]
    fn schemas_are_suffixed_operations_are_not() {
        let ops = [op(Method::Get, "/paths", "x")];
        let config = OperationConfig::default();
        let names_ = names(&[], &config, &["blob", "error", "Error"]).unwrap();
        assert_eq!(names_.schema("blob"), Some("Blob2"));
        assert_eq!(names_.schema("error"), Some("Error2"));
        assert_eq!(names_.schema("Error"), Some("Error3"));

        let err = names(&ops, &config, &["GetPaths"]).unwrap_err().to_string();
        assert!(err.contains("already the schema \"GetPaths\""), "{err}");
        let renamed = OperationConfig {
            name: Template::parse("Paths").unwrap(),
            ..OperationConfig::default()
        };
        let err = names(&ops, &renamed, &[]).unwrap_err().to_string();
        assert!(err.contains("already an `emit` type"), "{err}");
    }
}
