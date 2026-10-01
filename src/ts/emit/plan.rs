//! Phase 1: which `emit` types exist, their names and which operations
//! each covers.

use std::collections::HashSet;

use indexmap::IndexMap;

use crate::case::Case;
use crate::config::Emit;
use crate::ir::Operation;
use crate::select::Where;
use crate::template::Var;
use crate::ts::naming::op_values;

/// The group variable and a group's value of it, when grouped.
pub(super) type Group = Option<(Var, String)>;

/// One type to emit. A grouped `emit` produces one plan per group.
#[derive(Debug)]
pub(crate) struct Plan<'c> {
    pub(super) emit: &'c Emit,
    /// Index of the `emit` entry, and of its report.
    pub(super) report: usize,
    pub(crate) name: String,
    pub(super) group: Group,
    /// Indexes into the selected operations.
    pub(super) ops: Vec<usize>,
}

impl Plan<'_> {
    /// `emit[1] ("{Tags}Routes", group "shop")`, for collision errors.
    pub(crate) fn owner(&self) -> String {
        let source = self.emit.name.source();
        match &self.group {
            Some((_, group)) => format!("emit[{}] ({source:?}, group {group:?})", self.report),
            None => format!("emit[{}] ({source:?})", self.report),
        }
    }
}

/// What became of the operations an `emit` was given, for `--explain`.
#[derive(Debug, Default)]
pub(crate) struct EmitReport {
    /// The name template.
    pub label: String,
    pub grouped: bool,
    /// Left out by `where`, by the selector field that didn't match.
    pub dropped: IndexMap<String, usize>,
    /// Each emitted type and how many operations it covers.
    pub types: Vec<(String, usize)>,
    /// Variable -> operations skipped for having no value of it (no tags to
    /// group on, no `operationId` for a key).
    pub skipped: IndexMap<Var, HashSet<usize>>,
}

impl EmitReport {
    fn new(emit: &Emit) -> Self {
        Self {
            label: emit.name.source().to_string(),
            grouped: emit.group_by.is_some(),
            ..Self::default()
        }
    }

    /// One emitted type: how many operations it covers, and the ones skipped
    /// for lacking a variable the shape uses.
    pub(super) fn record(
        &mut self,
        name: &str,
        entries: usize,
        skipped: IndexMap<Var, HashSet<usize>>,
    ) {
        self.types.push((name.to_string(), entries));
        for (var, ops) in skipped {
            self.skipped.entry(var).or_default().extend(ops);
        }
    }
}

/// One plan per `emit` (per group when grouped), and one report per `emit`.
/// Name collisions are caught when the names are claimed.
pub(crate) fn plan<'c>(emits: &'c [Emit], ops: &[&Operation]) -> (Vec<Plan<'c>>, Vec<EmitReport>) {
    let mut plans = Vec::new();
    let mut reports = Vec::new();
    for emit in emits {
        let mut report = EmitReport::new(emit);
        let matching = select(emit.filter.as_ref(), ops, &mut report.dropped);
        for (group, ops) in group(emit.group_var(), ops, matching, &mut report.skipped) {
            let name = plan_name(emit, &group);
            plans.push(Plan {
                emit,
                report: reports.len(),
                name,
                group,
                ops,
            });
        }
        reports.push(report);
    }
    (plans, reports)
}

/// Indexes of the operations `filter` keeps; the rest are tallied in
/// `dropped` by the field that didn't match.
fn select(
    filter: Option<&Where>,
    ops: &[&Operation],
    dropped: &mut IndexMap<String, usize>,
) -> Vec<usize> {
    let mut matching = Vec::new();
    for (i, op) in ops.iter().enumerate() {
        match filter.and_then(|w| w.mismatch(*op)) {
            None => matching.push(i),
            Some(field) => *dropped.entry(field).or_default() += 1,
        }
    }
    matching
}

/// One group per distinct value of `var`, in first-seen order: an operation
/// with several values is in each, one with none is `skipped`. Ungrouped,
/// the one group of everything.
fn group(
    var: Option<Var>,
    ops: &[&Operation],
    matching: Vec<usize>,
    skipped: &mut IndexMap<Var, HashSet<usize>>,
) -> Vec<(Group, Vec<usize>)> {
    let Some(var) = var else {
        return vec![(None, matching)];
    };
    let mut groups: IndexMap<String, Vec<usize>> = IndexMap::new();
    for i in matching {
        let values = op_values(ops[i], var, None);
        if values.is_empty() {
            skipped.entry(var).or_default().insert(i);
        }
        for value in values {
            groups.entry(value).or_default().push(i);
        }
    }
    groups
        .into_iter()
        .map(|(value, ops)| (Some((var, value)), ops))
        .collect()
}

/// The name template with the group variable bound, as an identifier.
fn plan_name(emit: &Emit, group: &Group) -> String {
    let bound = |var| {
        group
            .as_ref()
            .filter(|(v, _)| *v == var)
            .map(|(_, value)| value.clone())
    };
    let name = emit
        .name
        .render(bound)
        .expect("validated: name only uses the group variable");
    Case::identifier(&name)
}
