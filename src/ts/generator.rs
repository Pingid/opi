//! The generator: what the config selects from the API, the names everything
//! gets, and the statements built from them.

use anyhow::Result;
use oxc_allocator::Allocator;
use oxc_ast::ast::*;

use super::naming::Names;
use super::selection::Selection;
use crate::builder::TypeBuilder;
use crate::config::Config;
use crate::ir;

pub struct Generator<'a> {
    pub(super) b: &'a TypeBuilder<'a>,
    pub(super) api: &'a ir::Api,
    pub(super) config: &'a Config,
    pub(super) sel: Selection<'a>,
    pub(super) names: Names,
}

impl<'a> Generator<'a> {
    pub fn new(b: &'a TypeBuilder<'a>, api: &'a ir::Api, config: &'a Config) -> Result<Self> {
        let sel = Selection::new(api, config)?;
        let formats: Vec<&str> = config.formats.values().map(String::as_str).collect();
        let emits: Vec<(&str, String)> = sel
            .plans
            .iter()
            .map(|p| (p.name.as_str(), p.owner()))
            .collect();
        let names = Names::new(&sel.schemas, &sel.ops, &config.operation, &formats, &emits)?;
        Ok(Self {
            b,
            api,
            config,
            sel,
            names,
        })
    }

    /// Validate `config`, build a generator over a fresh arena and hand it to
    /// `f`. The arena dies with the call, so `f` returns owned data.
    pub(crate) fn run<T>(
        api: &ir::Api,
        config: &Config,
        f: impl for<'g> FnOnce(&Generator<'g>) -> Result<T>,
    ) -> Result<T> {
        config.validate()?;
        let allocator = Allocator::default();
        let builder = TypeBuilder::new(&allocator);
        let generator = Generator::new(&builder, api, config)?;
        f(&generator)
    }

    /// Named schemas, then one type per operation, then `emit` types.
    pub fn statements(&self) -> Result<Vec<Statement<'a>>> {
        let schemas = self
            .sel
            .schemas
            .iter()
            .map(|&raw| self.named_schema(raw, &self.api.schemas[raw]));
        let operations = self
            .sel
            .ops
            .iter()
            .enumerate()
            .map(|(i, op)| self.operation(self.names.operation(i), op));
        let mut statements: Vec<_> = schemas.chain(operations).collect();
        for plan in &self.sel.plans {
            statements.push(self.emit(plan)?);
        }
        Ok(statements)
    }

    /// `export type Name = T;`, with docs.
    pub(super) fn export_type(
        &self,
        name: &str,
        ty: TSType<'a>,
        docs: Vec<String>,
    ) -> Statement<'a> {
        let statement = self.b.export(self.b.type_alias(self.b.str(name), ty));
        self.doc(statement, docs)
    }

    pub(super) fn doc<N: oxc_span::GetSpanMut>(&self, node: N, lines: Vec<String>) -> N {
        if self.config.jsdoc {
            self.b.with_doc(node, &lines)
        } else {
            node
        }
    }
}
