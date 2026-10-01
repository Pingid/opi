//! Merging every operation's rendering into one object type.

use anyhow::{Result, anyhow};
use indexmap::IndexMap;
use oxc_ast::ast::*;

use super::shape::Rendered;
use crate::ir::Operation;
use crate::ts::{DocLines, Generator};

/// The type being built: each key is a branch, the leaves hold every
/// operation that landed on that key (unioned if more than one).
pub(super) enum Node<'a, 'o> {
    Branch {
        children: IndexMap<String, Node<'a, 'o>>,
        templated: bool,
        ops: Vec<&'o Operation>,
    },
    Leaf {
        values: Vec<TSType<'a>>,
        templated: bool,
        ops: Vec<&'o Operation>,
    },
}

impl<'a> Generator<'a> {
    /// An operation's docs go on the outermost key that varies per operation
    /// and holds only it (`get` under a `'/items'` that has `post` too);
    /// `documented` is whether an enclosing key already has them.
    pub(super) fn members(
        &self,
        nodes: IndexMap<String, Node<'a, '_>>,
        documented: bool,
    ) -> Vec<TSSignature<'a>> {
        let b = self.b;
        nodes
            .into_iter()
            .map(|(key, node)| {
                let key = b.str(&key);
                let (templated, ops) = match &node {
                    Node::Branch { templated, ops, .. } | Node::Leaf { templated, ops, .. } => {
                        (*templated, ops.clone())
                    }
                };
                let op = match ops.as_slice() {
                    [op] if templated && !documented => Some(*op),
                    _ => None,
                };
                let member = match node {
                    Node::Branch { children, .. } => {
                        let members = self.members(children, documented || op.is_some());
                        b.prop(key, b.type_literal(members))
                    }
                    Node::Leaf { values, .. } => b.prop(key, b.union(values)),
                };
                match op {
                    Some(op) => self.doc(member, DocLines::summary(op).done()),
                    None => member,
                }
            })
            .collect()
    }
}

/// Every operation's map rendering merged into one tree. The error is a key
/// that's an object for one operation and a value for another.
pub(super) fn merge<'a, 'o>(
    rendered: Vec<(Rendered<'a>, &'o Operation)>,
    type_name: &str,
) -> Result<IndexMap<String, Node<'a, 'o>>> {
    let mut root = IndexMap::new();
    for (r, op) in rendered {
        let Rendered::Map(entries) = r else {
            unreachable!("a map shape renders a map")
        };
        insert(&mut root, entries, op).map_err(|key| {
            anyhow!("emit `{type_name}`: key {key:?} is an object for one operation and a value for another")
        })?;
    }
    Ok(root)
}

/// Merge one operation's rendering into the tree. `Err` is a key that's a
/// value in one place and an object in another.
fn insert<'a, 'o>(
    nodes: &mut IndexMap<String, Node<'a, 'o>>,
    entries: Vec<(String, bool, Rendered<'a>)>,
    op: &'o Operation,
) -> Result<(), String> {
    for (key, templated, rendered) in entries {
        match rendered {
            Rendered::Value(ty) => {
                let node = nodes.entry(key.clone()).or_insert_with(|| Node::Leaf {
                    values: Vec::new(),
                    templated,
                    ops: Vec::new(),
                });
                let Node::Leaf { values, ops, .. } = node else {
                    return Err(key);
                };
                values.push(ty);
                add(ops, op);
            }
            Rendered::Map(entries) => {
                let node = nodes.entry(key.clone()).or_insert_with(|| Node::Branch {
                    children: IndexMap::new(),
                    templated,
                    ops: Vec::new(),
                });
                let Node::Branch { children, ops, .. } = node else {
                    return Err(key);
                };
                add(ops, op);
                insert(children, entries, op)?;
            }
        }
    }
    Ok(())
}

fn add<'o>(ops: &mut Vec<&'o Operation>, op: &'o Operation) {
    if !ops.iter().any(|o| std::ptr::eq(*o, op)) {
        ops.push(op);
    }
}
