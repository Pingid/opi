use oxc_ast::ast::*;

use super::{TsMembers, TypeBuilder};

// ---------------------------------------------------------------------------
// Schema-shaped types (`{ type: "string" }` etc.)
// ---------------------------------------------------------------------------
pub struct JsonTypeBuilder<'a> {
    builder: &'a TypeBuilder<'a>,
}

impl<'a> JsonTypeBuilder<'a> {
    pub fn new(builder: &'a TypeBuilder<'a>) -> Self {
        Self { builder }
    }

    pub fn json_type(&self, type_name: &'a str) -> TsMembers<'a> {
        self.builder
            .member_builder()
            .prop("type", self.builder.string_literal(type_name))
    }

    pub fn string(&self) -> TSType<'a> {
        self.json_type("string").into_literal()
    }

    pub fn number(&self) -> TSType<'a> {
        self.json_type("number").into_literal()
    }

    pub fn integer(&self) -> TSType<'a> {
        self.json_type("integer").into_literal()
    }

    pub fn boolean(&self) -> TSType<'a> {
        self.json_type("boolean").into_literal()
    }

    pub fn null(&self) -> TSType<'a> {
        self.json_type("null").into_literal()
    }
}

impl<'a> std::ops::Deref for JsonTypeBuilder<'a> {
    type Target = TypeBuilder<'a>;

    fn deref(&self) -> &Self::Target {
        self.builder
    }
}
