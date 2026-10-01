//! `emit` entries -> aggregate unions / object types over operations.
//!
//! Runs in two phases because names depend on each other:
//! 1. [`plan`] (before naming): decide which types exist, their names and
//!    which operations each covers. Emit names are then reserved so
//!    schemas / operations can't take them.
//! 2. [`Generator::emit`] (after naming): render the `shape` per operation,
//!    whose values reference the operation types by name.

mod narrow;
mod plan;
mod shape;
#[cfg(test)]
mod tests;
mod tree;

use std::collections::HashSet;

use anyhow::Result;
use indexmap::IndexMap;
use oxc_ast::ast::*;

pub(crate) use plan::{EmitReport, Plan, plan};
use shape::{Compiled, Env, Rendered};
use tree::merge;

use super::{DocLines, Generator};
use crate::ir::Operation;
use crate::template::Var;

impl<'a> Generator<'a> {
    /// `export type Name = …;`: a union for a value shape, an object type
    /// for a map.
    pub(super) fn emit(&self, plan: &Plan<'a>) -> Result<Statement<'a>> {
        let shape = Compiled::new(&plan.emit.shape, Var::TEXT);
        let rendered = self.render_all(plan, &shape);
        let docs = DocLines::description(plan.emit.description.as_deref()).done();
        let ty = match shape {
            Compiled::Value(_) => self
                .b
                .union(rendered.into_iter().map(|(r, _)| r.into_value())),
            Compiled::Map(_) => {
                let members = self.members(merge(rendered, &plan.name)?, false);
                self.b.type_literal(members)
            }
        };
        Ok(self.export_type(&plan.name, ty, docs))
    }

    /// Each operation's rendering. Operations missing a variable the shape
    /// uses are skipped, and the plan's report told.
    fn render_all(&self, plan: &Plan<'a>, shape: &Compiled) -> Vec<(Rendered<'a>, &'a Operation)> {
        let mut rendered = Vec::new();
        let mut skipped: IndexMap<Var, HashSet<usize>> = IndexMap::new();
        for &i in &plan.ops {
            let op = self.sel.ops[i];
            let env = Env::new(op, self.names.operation(i), &plan.group);
            match self.render(shape, &env) {
                Ok(r) => rendered.push((r, op)),
                Err(var) => {
                    skipped.entry(var).or_default().insert(i);
                }
            }
        }
        let mut reports = self.sel.reports.borrow_mut();
        reports[plan.report].record(&plan.name, rendered.len(), skipped);
        rendered
    }
}
