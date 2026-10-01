//! Object members: property signatures and the [`TsMembers`] accumulator.

use oxc_allocator::{ArenaBox, ArenaVec};
use oxc_ast::ast::*;
use oxc_span::SPAN;

use super::TypeBuilder;
use crate::case::Case;

impl<'a> TypeBuilder<'a> {
    pub fn annotation(&self, type_: TSType<'a>) -> ArenaBox<'a, TSTypeAnnotation<'a>> {
        TSTypeAnnotation::boxed(SPAN, type_, &self.ast)
    }

    /// `foo` if it's a valid identifier, `200` if it's a canonical integer,
    /// otherwise `"content-type"`.
    pub fn property_key(&self, key: &'a str) -> PropertyKey<'a> {
        if Case::is_identifier(&key) {
            PropertyKey::new_static_identifier(SPAN, key, &self.ast)
        } else if let Some(n) = canonical_integer(key) {
            PropertyKey::new_numeric_literal(SPAN, n, None, NumberBase::Decimal, &self.ast)
        } else {
            PropertyKey::new_string_literal(SPAN, Str::from_str_in(key, &self.ast), None, &self.ast)
        }
    }
    /// `key: T`, `key?: T`, `readonly key: T`.
    pub fn property(
        &self,
        key: &'a str,
        type_: TSType<'a>,
        optional: bool,
        readonly: bool,
    ) -> TSSignature<'a> {
        TSSignature::new_ts_property_signature(
            SPAN,
            false,
            optional,
            readonly,
            self.property_key(key),
            Some(self.annotation(type_)),
            &self.ast,
        )
    }

    /// Key listed in `required` -> `key: T`
    pub fn prop(&self, key: &'a str, type_: TSType<'a>) -> TSSignature<'a> {
        self.property(key, type_, false, false)
    }

    /// Key not in `required` -> `key?: T`
    pub fn optional_prop(&self, key: &'a str, type_: TSType<'a>) -> TSSignature<'a> {
        self.property(key, type_, true, false)
    }

    /// `key: T` when required, else `key?: T`.
    pub fn field(&self, key: &'a str, type_: TSType<'a>, required: bool) -> TSSignature<'a> {
        self.property(key, type_, !required, false)
    }

    /// `additionalProperties: T` alongside `properties` -> `[key: string]: T`
    pub fn index_signature(&self, value: TSType<'a>) -> TSSignature<'a> {
        TSSignature::new_ts_index_signature(
            SPAN,
            TSIndexSignatureName::new(SPAN, "key", self.annotation(self.string()), &self.ast),
            self.annotation(value),
            false,
            false,
            &self.ast,
        )
    }

    pub fn type_literal(&self, members: impl IntoIterator<Item = TSSignature<'a>>) -> TSType<'a> {
        TSType::new_ts_type_literal(
            SPAN,
            ArenaVec::<TSSignature<'a>>::from_iter_in(members, &self.ast),
            &self.ast,
        )
    }

    pub fn member_builder(&'a self) -> TsMembers<'a> {
        TsMembers::new(self)
    }
}

pub struct TsMembers<'a> {
    builder: &'a TypeBuilder<'a>,
    members: Vec<TSSignature<'a>>,
}

impl<'a> TsMembers<'a> {
    pub fn new(builder: &'a TypeBuilder<'a>) -> Self {
        Self {
            builder,
            members: Vec::new(),
        }
    }

    pub fn with(mut self, member: impl Into<TSSignature<'a>>) -> Self {
        self.members.push(member.into());
        self
    }

    pub fn prop(self, key: &'a str, type_: TSType<'a>) -> Self {
        let member = self.builder.prop(key, type_);
        self.with(member)
    }

    pub fn optional_prop(self, key: &'a str, type_: TSType<'a>) -> Self {
        let member = self.builder.optional_prop(key, type_);
        self.with(member)
    }

    pub fn index_signature(self, value: TSType<'a>) -> Self {
        let member = self.builder.index_signature(value);
        self.with(member)
    }

    pub fn into_literal(self) -> TSType<'a> {
        self.builder.type_literal(self.members)
    }
}
/// `"200"` -> `Some(200.0)`, but not `"0200"` or `"2XX"`, which must stay quoted
/// to round-trip as the same key.
fn canonical_integer(s: &str) -> Option<f64> {
    let canonical = !s.is_empty()
        && s.len() <= 15
        && s.bytes().all(|b| b.is_ascii_digit())
        && (s == "0" || !s.starts_with('0'));
    canonical.then(|| s.parse().ok()).flatten()
}
