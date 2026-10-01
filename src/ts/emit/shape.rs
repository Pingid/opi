//! Phase 2: a compiled `shape`, rendered per operation with the variables
//! bound for it.

use oxc_ast::ast::*;

use super::plan::Group;
use crate::config::{Resolved, Root, Shape};
use crate::ir::Operation;
use crate::template::{Template, Var};
use crate::ts::Generator;
use crate::ts::naming::op_values;

/// A [`Shape`] with its key templates parsed and values decoded.
pub(super) enum Compiled<'c> {
    Map(Vec<Key<'c>>),
    Value(Resolved<'c>),
}

pub(super) struct Key<'c> {
    template: Template,
    /// The key varies per operation (uses a variable).
    templated: bool,
    shape: Compiled<'c>,
}

impl<'c> Compiled<'c> {
    pub(super) fn new(shape: &'c Shape, vars: &[Var]) -> Self {
        match shape {
            Shape::Value(value) => Compiled::Value(
                value
                    .resolve(vars, "shape")
                    .expect("validated with the config"),
            ),
            Shape::Map(map) => Compiled::Map(
                map.iter()
                    .map(|(key, shape)| {
                        let template = Template::parse(key).expect("validated with the config");
                        let templated = template.vars().next().is_some();
                        Key {
                            templated,
                            template,
                            shape: Compiled::new(shape, vars),
                        }
                    })
                    .collect(),
            ),
        }
    }
}

/// One operation's contribution, before merging.
pub(super) enum Rendered<'a> {
    Map(Vec<(String, bool, Rendered<'a>)>),
    Value(TSType<'a>),
}

impl<'a> Rendered<'a> {
    pub(super) fn into_value(self) -> TSType<'a> {
        match self {
            Rendered::Value(ty) => ty,
            Rendered::Map(_) => unreachable!("a value shape renders a value"),
        }
    }
}

/// The variables for rendering one operation: its own values, with any a
/// group or an enclosing key fixed bound to one.
#[derive(Clone)]
pub(super) struct Env<'o> {
    pub(super) op: &'o Operation,
    pub(super) type_name: &'o str,
    pub(super) bound: Vec<(Var, String)>,
}

impl<'o> Env<'o> {
    pub(super) fn new(op: &'o Operation, type_name: &'o str, group: &Group) -> Self {
        Self {
            op,
            type_name,
            bound: group.iter().cloned().collect(),
        }
    }
}

impl Env<'_> {
    /// The value a group or an enclosing key fixed `var` to, if any.
    pub(super) fn bound(&self, var: Var) -> Option<&str> {
        self.bound
            .iter()
            .find(|(v, _)| *v == var)
            .map(|(_, value)| value.as_str())
    }

    fn values(&self, var: Var) -> Vec<String> {
        match self.bound(var) {
            Some(value) => vec![value.to_string()],
            None => op_values(self.op, var, Some(self.type_name)),
        }
    }

    /// One environment per combination of values of the variables
    /// `template` uses, each bound: a key with `{tags}` is repeated per tag.
    /// `Err` is a variable with no value.
    fn expand(&self, template: &Template) -> Result<Vec<Self>, Var> {
        let mut envs = vec![self.clone()];
        let mut seen = Vec::new();
        for var in template.vars() {
            if seen.contains(&var) {
                continue;
            }
            seen.push(var);
            let values = self.values(var);
            if values.is_empty() {
                return Err(var);
            }
            envs = envs
                .into_iter()
                .flat_map(|env| {
                    values.iter().map(move |value| {
                        let mut env = env.clone();
                        env.bound.push((var, value.clone()));
                        env
                    })
                })
                .collect();
        }
        Ok(envs)
    }

    /// Render with every variable bound (see [`Env::expand`]).
    fn render(&self, template: &Template) -> String {
        template
            .render(|var| self.values(var).into_iter().next())
            .expect("expanded: every variable has one value")
    }
}

impl<'a> Generator<'a> {
    /// `Err` is the variable an operation had no value for.
    pub(super) fn render(&self, shape: &Compiled, env: &Env) -> Result<Rendered<'a>, Var> {
        Ok(match shape {
            Compiled::Value(value) => Rendered::Value(self.value(value, env)?),
            Compiled::Map(keys) => {
                let mut entries = Vec::new();
                for key in keys {
                    for env in env.expand(&key.template)? {
                        let name = env.render(&key.template);
                        entries.push((name, key.templated, self.render(&key.shape, &env)?));
                    }
                }
                Rendered::Map(entries)
            }
        })
    }

    fn value(&self, value: &Resolved, env: &Env) -> Result<TSType<'a>, Var> {
        let b = self.b;
        let (target, filter, pick) = match *value {
            // One literal per value: `"{tags}"` -> `"shop" | "admin"`.
            Resolved::Literal(template) => {
                let envs = env.expand(template)?;
                let literals = envs
                    .iter()
                    .map(|e| b.string_literal(b.str(&e.render(template))));
                return Ok(b.union(literals));
            }
            Resolved::Ref {
                target,
                filter,
                pick,
            } => (target, filter, pick),
        };
        let name = b.reference(b.str(env.type_name));
        let ty = match target.root {
            Root::Op => name,
            Root::Request => b.indexed_access(name, "request"),
            Root::Response => b.indexed_access(name, "response"),
        };
        let ty = self.narrow(ty, target.root, filter, env);
        let ty = match target.key {
            Some(key) => b.indexed_access(ty, key),
            None => ty,
        };
        Ok(match pick {
            [] => ty,
            keys => b.generic(
                "Pick",
                [ty, b.union(keys.iter().map(|k| b.string_literal(b.str(k))))],
            ),
        })
    }
}
