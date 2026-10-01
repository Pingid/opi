//! Thin helpers over `oxc_ast` / `oxc_codegen`: [`TypeBuilder`] owns the
//! arena and the doc-comment side table; the submodules add constructors
//! by what they build (types, object members, docs) and the codegen.

mod codegen;
mod doc;
mod member;
mod types;

use oxc_allocator::Allocator;
use oxc_ast::ast::*;
use oxc_ast::builder::AstBuilder;
use oxc_span::SPAN;

use doc::Docs;
pub use member::TsMembers;

pub struct TypeBuilder<'a> {
    ast: AstBuilder<'a>,
    allocator: &'a Allocator,
    docs: Docs,
}

impl<'a> TypeBuilder<'a> {
    pub fn new(allocator: &'a Allocator) -> Self {
        Self {
            ast: AstBuilder::new(allocator),
            allocator,
            docs: Docs::default(),
        }
    }

    /// Copy a string into the arena so it can live in the AST.
    pub fn str(&self, s: &str) -> &'a str {
        self.allocator.alloc_str(s)
    }
}

// Declarations
impl<'a> TypeBuilder<'a> {
    /// `type Name = T;`
    pub fn type_alias(&self, name: &'a str, type_: TSType<'a>) -> Declaration<'a> {
        Declaration::new_ts_type_alias_declaration(
            SPAN,
            BindingIdentifier::new(SPAN, name, &self.ast),
            None,
            type_,
            false,
            &self.ast,
        )
    }

    /// `export <declaration>`
    pub fn export(&self, declaration: Declaration<'a>) -> Statement<'a> {
        ModuleDeclaration::new_export_declaration(SPAN, declaration, &self.ast).into()
    }
}
