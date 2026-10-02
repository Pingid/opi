//! Types: keywords, literals and composites.

use oxc_allocator::ArenaVec;
use oxc_ast::ast::*;
use oxc_span::{ContentEq, SPAN};

use super::TypeBuilder;

// Keywords
impl<'a> TypeBuilder<'a> {
    pub fn string(&self) -> TSType<'a> {
        TSType::new_ts_string_keyword(SPAN, &self.ast)
    }

    pub fn number(&self) -> TSType<'a> {
        TSType::new_ts_number_keyword(SPAN, &self.ast)
    }

    pub fn boolean(&self) -> TSType<'a> {
        TSType::new_ts_boolean_keyword(SPAN, &self.ast)
    }

    pub fn null(&self) -> TSType<'a> {
        TSType::new_ts_null_keyword(SPAN, &self.ast)
    }

    pub fn unknown(&self) -> TSType<'a> {
        TSType::new_ts_unknown_keyword(SPAN, &self.ast)
    }

    pub fn never(&self) -> TSType<'a> {
        TSType::new_ts_never_keyword(SPAN, &self.ast)
    }

    pub fn undefined(&self) -> TSType<'a> {
        TSType::new_ts_undefined_keyword(SPAN, &self.ast)
    }
}

// Literals (`const` / `enum`)
impl<'a> TypeBuilder<'a> {
    pub fn string_literal(&self, value: &'a str) -> TSType<'a> {
        TSType::new_ts_literal_type(
            SPAN,
            TSLiteral::new_string_literal(
                SPAN,
                Str::from_str_in(value, &self.ast),
                None,
                &self.ast,
            ),
            &self.ast,
        )
    }

    /// Negative numbers are emitted as `-(n)` unary expressions, which is how
    /// the TS parser represents them in a literal type.
    pub fn number_literal(&self, value: f64) -> TSType<'a> {
        let literal = if value < 0.0 {
            TSLiteral::new_unary_expression(
                SPAN,
                UnaryOperator::UnaryNegation,
                Expression::new_numeric_literal(SPAN, -value, None, NumberBase::Decimal, &self.ast),
                &self.ast,
            )
        } else {
            TSLiteral::new_numeric_literal(SPAN, value, None, NumberBase::Decimal, &self.ast)
        };
        TSType::new_ts_literal_type(SPAN, literal, &self.ast)
    }

    pub fn boolean_literal(&self, value: bool) -> TSType<'a> {
        TSType::new_ts_literal_type(
            SPAN,
            TSLiteral::new_boolean_literal(SPAN, value, &self.ast),
            &self.ast,
        )
    }
}

// Composites
impl<'a> TypeBuilder<'a> {
    /// `items: T` -> `T[]`
    pub fn array(&self, element: TSType<'a>) -> TSType<'a> {
        TSType::new_ts_array_type(SPAN, element, &self.ast)
    }

    /// `anyOf` / `oneOf` / `type: [..]`.
    /// Zero members -> `never`, one member -> that member (no degenerate union).
    /// Nested unions are flattened (`A | (B | C)` -> `A | B | C`) and
    /// duplicate members dropped (`A | A` -> `A`).
    pub fn union(&self, types: impl IntoIterator<Item = TSType<'a>>) -> TSType<'a> {
        let mut members: Vec<TSType<'a>> = Vec::new();
        for t in types.into_iter().flat_map(|t| match t {
            TSType::TSUnionType(union) => union.unbox().types.into_iter().collect(),
            t => vec![t],
        }) {
            if !members.iter().any(|existing| existing.content_eq(&t)) {
                members.push(t);
            }
        }
        let mut types = members;
        match types.len() {
            0 => self.never(),
            1 => types.pop().unwrap(),
            _ => {
                TSType::new_ts_union_type(SPAN, ArenaVec::from_iter_in(types, &self.ast), &self.ast)
            }
        }
    }

    /// `allOf`. Zero members -> `unknown`, one member -> that member.
    pub fn intersection(&self, types: impl IntoIterator<Item = TSType<'a>>) -> TSType<'a> {
        let mut types: Vec<_> = types.into_iter().collect();
        match types.len() {
            0 => self.unknown(),
            1 => types.pop().unwrap(),
            _ => TSType::new_ts_intersection_type(
                SPAN,
                ArenaVec::from_iter_in(types, &self.ast),
                &self.ast,
            ),
        }
    }

    /// `type: ["T", "null"]` / `nullable: true` -> `T | null`
    pub fn nullable(&self, type_: TSType<'a>) -> TSType<'a> {
        self.union([type_, self.null()])
    }

    /// `$ref: "#/$defs/Foo"` -> `Foo`
    pub fn reference(&self, name: &'a str) -> TSType<'a> {
        TSType::new_ts_type_reference(
            SPAN,
            TSTypeName::new_identifier_reference(SPAN, name, &self.ast),
            None,
            &self.ast,
        )
    }

    /// `Name<A, B>`
    pub fn generic(&self, name: &'a str, args: impl IntoIterator<Item = TSType<'a>>) -> TSType<'a> {
        TSType::new_ts_type_reference(
            SPAN,
            TSTypeName::new_identifier_reference(SPAN, name, &self.ast),
            Some(TSTypeParameterInstantiation::boxed(
                SPAN,
                ArenaVec::from_iter_in(args, &self.ast),
                &self.ast,
            )),
            &self.ast,
        )
    }

    /// `T['key']`
    pub fn indexed_access(&self, object: TSType<'a>, key: &'a str) -> TSType<'a> {
        TSType::new_ts_indexed_access_type(SPAN, object, self.string_literal(key), &self.ast)
    }

    /// `type: "object", additionalProperties: T` (no `properties`) -> `Record<string, T>`
    pub fn record(&self, value: TSType<'a>) -> TSType<'a> {
        self.generic("Record", [self.string(), value])
    }
}
