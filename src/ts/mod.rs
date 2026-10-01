//! TypeScript backend: [`ir::Api`] -> `.ts` source.
//!
//! Split by what's being emitted:
//! - [`schema`]: `ir::Schema` -> `TSType` (named schemas + inline types)
//! - [`operation`]: `ir::Operation` -> `export type GetFoo = { method, path, request, response }`
//! - [`emit`]: `[[emit]]` config entries -> unions / interfaces over operations
//! - [`naming`]: identifier allocation shared by all of them
//!
//! All the oxc plumbing lives in [`crate::builder`]; this module only decides
//! *what* to build, driven by [`Config`].

mod emit;
mod naming;
mod operation;
mod schema;

use anyhow::Result;
use oxc_allocator::Allocator;
use oxc_ast::ast::*;
use oxc_codegen::CodegenOptions;

use crate::builder::TypeBuilder;
use crate::config::{Config, SchemaEmit};
use crate::select::wildcard;
use crate::ir::{self, Meta};

pub use naming::{identifier, operation_name, pascal_case, path_pascal_case};

pub fn generate(api: &ir::Api, config: &Config) -> Result<String> {
    config.validate()?;
    let allocator = Allocator::default();
    let builder = TypeBuilder::new(&allocator);
    let generator = Generator::new(&builder, api, config)?;

    let statements = generator.statements()?;
    let code = builder.code_gen_with(
        CodegenOptions {
            single_quote: config.single_quote,
            ..CodegenOptions::default()
        },
        statements,
    );

    Ok(match config.header.as_str() {
        "" => code,
        header => format!("{header}\n\n{code}"),
    })
}

pub struct Generator<'a> {
    b: &'a TypeBuilder<'a>,
    api: &'a ir::Api,
    config: &'a Config,
    /// Operations that passed `[filter]`, in spec order.
    ops: Vec<&'a ir::Operation>,
    /// Raw names of the schemas to generate, in spec order.
    schemas: Vec<&'a str>,
    emits: Vec<emit::Plan<'a>>,
    names: naming::Names,
}

impl<'a> Generator<'a> {
    pub fn new(b: &'a TypeBuilder<'a>, api: &'a ir::Api, config: &'a Config) -> Result<Self> {
        let ops: Vec<_> = api
            .operations
            .iter()
            .filter(|op| config.filter.allows(op))
            .collect();
        let emits = emit::plan(&config.emit, &ops)?;
        let schemas = match config.schemas.emit {
            SchemaEmit::All => api.schemas.keys().map(String::as_str).collect(),
            SchemaEmit::Referenced => {
                let keep = api.schemas.keys().map(String::as_str).filter(|name| {
                    config.schemas.keep.iter().any(|glob| wildcard(glob, name))
                });
                api.reachable_schemas(&ops, keep)
            }
        };

        // Names the config asks for explicitly win over generated ones.
        let reserved: Vec<&str> = config
            .formats
            .values()
            .map(String::as_str)
            .chain(emits.iter().map(|e| e.name.as_str()))
            .collect();
        let names = naming::Names::new(&schemas, &ops, &config.operation.name, &reserved);

        Ok(Self {
            b,
            api,
            config,
            ops,
            schemas,
            emits,
            names,
        })
    }

    /// Named schemas, then one type per operation, then `[[emit]]` types.
    pub fn statements(&self) -> Result<Vec<Statement<'a>>> {
        let schemas = self
            .schemas
            .iter()
            .map(|&raw| self.named_schema(raw, &self.api.schemas[raw]));
        let operations = self
            .ops
            .iter()
            .enumerate()
            .map(|(i, op)| self.operation(self.names.operation(i), op));
        let mut statements: Vec<_> = schemas.chain(operations).collect();
        for plan in &self.emits {
            statements.push(self.emit(plan)?);
        }
        Ok(statements)
    }

    /// `export type Name = T;`, with docs.
    fn export_type(&self, name: &str, ty: TSType<'a>, docs: Vec<String>) -> Statement<'a> {
        let statement = self.b.export(self.b.type_alias(self.b.str(name), ty));
        self.doc(statement, docs)
    }

    fn doc<N: oxc_span::GetSpanMut>(&self, node: N, lines: Vec<String>) -> N {
        if self.config.jsdoc {
            self.b.with_doc(node, &lines)
        } else {
            node
        }
    }
}

/// Build JSDoc lines. Kept as plain strings so any emitter can add its own
/// tags (`@summary`, `@deprecated`, ...).
#[derive(Default)]
struct DocLines(Vec<String>);

impl DocLines {
    fn tag(mut self, tag: &str, text: Option<&str>) -> Self {
        let Some(text) = text.map(str::trim).filter(|t| !t.is_empty()) else {
            return self;
        };
        let mut lines = text.lines();
        if let Some(first) = lines.next() {
            self.0.push(format!("@{tag} {first}"));
        }
        self.0.extend(lines.map(|l| l.trim_end().to_string()));
        self
    }

    fn flag(mut self, tag: &str, set: bool) -> Self {
        if set {
            self.0.push(format!("@{tag}"));
        }
        self
    }

    fn meta(self, meta: &Meta) -> Self {
        self.tag("description", meta.description.as_deref())
            .tag("contentMediaType", meta.encoded.as_deref())
            .flag("deprecated", meta.deprecated)
    }

    fn done(self) -> Vec<String> {
        self.0
    }
}
