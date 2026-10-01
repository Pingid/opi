//! What the config selects from the API, before anything is named.

use std::cell::RefCell;

use anyhow::Result;
use indexmap::IndexMap;

use super::emit::{self, EmitReport, Plan};
use crate::config::{Config, SchemaEmit, SchemasConfig};
use crate::ir::{Api, Operation};
use crate::select::Where;

pub(super) struct Selection<'a> {
    /// Operations that passed `filter`, in spec order.
    pub ops: Vec<&'a Operation>,
    /// Operations `filter` left out, by the selector field that didn't match.
    pub excluded: IndexMap<String, usize>,
    /// Raw names of the schemas to generate, in spec order.
    pub schemas: Vec<&'a str>,
    pub plans: Vec<Plan<'a>>,
    /// Filled in by `Generator::emit`.
    pub reports: RefCell<Vec<EmitReport>>,
}

impl<'a> Selection<'a> {
    pub fn new(api: &'a Api, config: &'a Config) -> Result<Self> {
        let (ops, excluded) = filter_ops(&config.filter, &api.operations);
        let (plans, reports) = emit::plan(&config.emit, &ops);
        let schemas = select_schemas(api, &config.schemas, &ops);
        Ok(Self {
            ops,
            excluded,
            schemas,
            plans,
            reports: RefCell::new(reports),
        })
    }
}

/// The operations `filter` keeps, and a tally of why the rest were dropped.
fn filter_ops<'a>(
    filter: &Where,
    all: &'a [Operation],
) -> (Vec<&'a Operation>, IndexMap<String, usize>) {
    let mut ops = Vec::new();
    let mut excluded: IndexMap<String, usize> = IndexMap::new();
    for op in all {
        match filter.mismatch(op) {
            None => ops.push(op),
            Some(field) => *excluded.entry(field).or_default() += 1,
        }
    }
    (ops, excluded)
}

/// Every schema, or the ones `ops` (and `keep`) reach.
fn select_schemas<'a>(api: &'a Api, config: &SchemasConfig, ops: &[&'a Operation]) -> Vec<&'a str> {
    let names = api.schemas.keys().map(String::as_str);
    match config.emit {
        SchemaEmit::All => names.collect(),
        SchemaEmit::Referenced => {
            let keep = config
                .keep
                .iter()
                .flat_map(|keep| names.clone().filter(|name| keep.matches(*name)));
            api.reachable_schemas(ops, keep)
        }
    }
}
