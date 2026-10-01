//! `[[emit]]` entries -> aggregate unions / interfaces over operations.
//!
//! Runs in two phases because names depend on each other:
//! 1. [`plan`] (before naming): decide which types exist, their names and
//!    which operations each covers. Emit names are then reserved so
//!    schemas / operations can't take them.
//! 2. [`Generator::emit`] (after naming): render keys and values, which may
//!    reference operation type names via `{type}`.

use std::collections::HashSet;

use anyhow::{Context, Result, bail};
use indexmap::IndexMap;
use oxc_ast::ast::*;

use super::naming::{identifier, op_var};
use super::{DocLines, Generator};
use crate::config::{Emit, EmitKind, GroupBy};
use crate::ir::Operation;

/// One type to emit. A grouped `[[emit]]` produces one plan per group.
#[derive(Debug)]
pub(super) struct Plan<'c> {
    emit: &'c Emit,
    pub(super) name: String,
    /// The group's value (tag or lowercase method), when grouped.
    group: Option<String>,
    /// Indexes into [`Generator::ops`].
    ops: Vec<usize>,
}

pub(super) fn plan<'c>(emits: &'c [Emit], ops: &[&Operation]) -> Result<Vec<Plan<'c>>> {
    let mut plans = Vec::new();
    let mut seen = HashSet::new();
    for emit in emits {
        let matching = (0..ops.len()).filter(|&i| emit.selector.matches(ops[i]));

        let groups: Vec<(Option<String>, Vec<usize>)> = match emit.group_by {
            None => vec![(None, matching.collect())],
            Some(group_by) => {
                let mut groups: IndexMap<String, Vec<usize>> = IndexMap::new();
                for i in matching {
                    // Untagged operations belong to no tag group.
                    let values = match group_by {
                        GroupBy::Tag => ops[i].tags.clone(),
                        GroupBy::Method => vec![ops[i].method.as_str().to_ascii_lowercase()],
                    };
                    for value in values {
                        groups.entry(value).or_default().push(i);
                    }
                }
                groups.into_iter().map(|(k, v)| (Some(k), v)).collect()
            }
        };

        for (group, ops) in groups {
            let group_var = emit.group_by.map(GroupBy::var);
            let name = emit
                .name
                .render(|var| (Some(var) == group_var).then(|| group.clone()).flatten())
                .expect("validated: name only uses the group variable");
            let name = identifier(&name);
            if !seen.insert(name.clone()) {
                bail!("more than one [[emit]] type is named `{name}`");
            }
            plans.push(Plan {
                emit,
                name,
                group,
                ops,
            });
        }
    }
    Ok(plans)
}

/// A map being built: each key level is a branch, the last level holds every
/// operation that landed on that key (unioned if more than one).
enum Node<'a, 'o> {
    Branch(IndexMap<String, Node<'a, 'o>>),
    Leaf(Vec<(TSType<'a>, &'o Operation)>),
}

impl<'a> Generator<'a> {
    pub(super) fn emit(&self, plan: &Plan<'a>) -> Result<Statement<'a>> {
        let b = self.b;
        let emit = plan.emit;
        let context = || format!("[[emit]] {}", plan.name);

        // (keys, value type, operation), skipping operations missing a
        // variable the key or value uses.
        let mut entries = Vec::new();
        for &i in &plan.ops {
            let op = self.ops[i];
            let tag = match emit.group_by {
                Some(GroupBy::Tag) => plan.group.as_deref(),
                _ => op.tags.first().map(String::as_str),
            };
            let type_name = self.names.operation(i);
            let vars = |var: &str| op_var(op, var, tag, Some(type_name));

            let Some(keys) = emit.key.as_slice().iter().map(|k| k.render(vars)).collect::<Option<Vec<_>>>()
            else {
                continue;
            };
            let ty = match &emit.value {
                None => b.reference(b.str(type_name)),
                Some(value) => match value.render(vars) {
                    Some(source) => b.parse_type(&source).with_context(context)?,
                    None => continue,
                },
            };
            entries.push((keys, ty, op));
        }

        let docs = DocLines::default()
            .tag("description", emit.description.as_deref())
            .done();

        Ok(match emit.kind {
            EmitKind::Union => {
                self.export_type(&plan.name, b.union(entries.into_iter().map(|(_, ty, _)| ty)), docs)
            }
            EmitKind::Map => {
                let mut root = IndexMap::new();
                for (keys, ty, op) in entries {
                    insert(&mut root, &keys, ty, op);
                }
                let members = self.map_members(root);
                self.doc(b.export(b.interface(b.str(&plan.name), members)), docs)
            }
        })
    }

    fn map_members(&self, nodes: IndexMap<String, Node<'a, '_>>) -> Vec<TSSignature<'a>> {
        let b = self.b;
        nodes
            .into_iter()
            .map(|(key, node)| {
                let key = b.str(&key);
                match node {
                    Node::Branch(children) => b.prop(key, b.type_literal(self.map_members(children))),
                    Node::Leaf(mut values) if values.len() == 1 => {
                        let (ty, op) = values.pop().unwrap();
                        let docs = DocLines::default()
                            .tag("summary", op.summary.as_deref())
                            .flag("deprecated", op.deprecated)
                            .done();
                        self.doc(b.prop(key, ty), docs)
                    }
                    Node::Leaf(values) => b.prop(key, b.union(values.into_iter().map(|(ty, _)| ty))),
                }
            })
            .collect()
    }
}

fn insert<'a, 'o>(
    nodes: &mut IndexMap<String, Node<'a, 'o>>,
    keys: &[String],
    ty: TSType<'a>,
    op: &'o Operation,
) {
    let (key, rest) = keys.split_first().expect("map emits have at least one key");
    if rest.is_empty() {
        let node = nodes.entry(key.clone()).or_insert_with(|| Node::Leaf(Vec::new()));
        let Node::Leaf(values) = node else {
            unreachable!("every entry has the same number of keys")
        };
        values.push((ty, op));
    } else {
        let node = nodes
            .entry(key.clone())
            .or_insert_with(|| Node::Branch(IndexMap::new()));
        let Node::Branch(children) = node else {
            unreachable!("every entry has the same number of keys")
        };
        insert(children, rest, ty, op);
    }
}
