use std::cell::{Cell, RefCell};

use oxc_allocator::{Allocator, ArenaBox, ArenaVec};
use oxc_ast::ast::*;
use oxc_ast::builder::AstBuilder;
use oxc_codegen::{Codegen, CodegenOptions};
use oxc_span::{ContentEq, GetSpanMut, SPAN, Span};

pub struct TypeBuilder<'a> {
    pub ast: AstBuilder<'a>,
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

// ---------------------------------------------------------------------------
// Keywords
//
// JSON Schema -> TS:
//   "string"            -> string
//   "number"/"integer"  -> number
//   "boolean"           -> boolean
//   "null"              -> null
//   {} / true           -> unknown
//   false               -> never
// ---------------------------------------------------------------------------
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

    pub fn any(&self) -> TSType<'a> {
        TSType::new_ts_any_keyword(SPAN, &self.ast)
    }

    pub fn never(&self) -> TSType<'a> {
        TSType::new_ts_never_keyword(SPAN, &self.ast)
    }

    pub fn undefined(&self) -> TSType<'a> {
        TSType::new_ts_undefined_keyword(SPAN, &self.ast)
    }
}

// ---------------------------------------------------------------------------
// Literals (`const` / `enum`)
// ---------------------------------------------------------------------------
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

    /// `enum: ["a", "b"]` -> `"a" | "b"`
    pub fn string_enum(&self, values: impl IntoIterator<Item = &'a str>) -> TSType<'a> {
        self.union(values.into_iter().map(|v| self.string_literal(v)))
    }
}

// ---------------------------------------------------------------------------
// Composites
// ---------------------------------------------------------------------------
impl<'a> TypeBuilder<'a> {
    /// `items: T` -> `T[]`
    pub fn array(&self, element: TSType<'a>) -> TSType<'a> {
        TSType::new_ts_array_type(SPAN, element, &self.ast)
    }

    /// `prefixItems: [A, B], items: false` -> `[A, B]`
    pub fn tuple(&self, elements: impl IntoIterator<Item = TSType<'a>>) -> TSType<'a> {
        self.tuple_with_rest(elements, None)
    }

    /// `prefixItems: [A, B], items: C` -> `[A, B, ...C[]]`
    pub fn tuple_with_rest(
        &self,
        elements: impl IntoIterator<Item = TSType<'a>>,
        rest: Option<TSType<'a>>,
    ) -> TSType<'a> {
        let elements = elements
            .into_iter()
            .map(TSTupleElement::from)
            .chain(rest.map(|r| TSTupleElement::new_ts_rest_type(SPAN, self.array(r), &self.ast)));
        TSType::new_ts_tuple_type(SPAN, ArenaVec::from_iter_in(elements, &self.ast), &self.ast)
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

    /// `type: "object", additionalProperties: T` (no `properties`) -> `Record<string, T>`
    pub fn record(&self, value: TSType<'a>) -> TSType<'a> {
        self.generic("Record", [self.string(), value])
    }
}

// ---------------------------------------------------------------------------
// Object members
// ---------------------------------------------------------------------------
impl<'a> TypeBuilder<'a> {
    pub fn annotation(&self, type_: TSType<'a>) -> ArenaBox<'a, TSTypeAnnotation<'a>> {
        TSTypeAnnotation::boxed(SPAN, type_, &self.ast)
    }

    /// `foo` if it's a valid identifier, `200` if it's a canonical integer,
    /// otherwise `"content-type"`.
    pub fn property_key(&self, key: &'a str) -> PropertyKey<'a> {
        if is_identifier(key) {
            PropertyKey::new_static_identifier(SPAN, key, &self.ast)
        } else if let Some(n) = canonical_integer(key) {
            PropertyKey::new_numeric_literal(SPAN, n, None, NumberBase::Decimal, &self.ast)
        } else {
            PropertyKey::new_string_literal(SPAN, Str::from_str_in(key, &self.ast), None, &self.ast)
        }
    }

    pub fn property(
        &self,
        key: &'a str,
        value: ArenaBox<'a, TSTypeAnnotation<'a>>,
        computed: bool,
        optional: bool,
        readonly: bool,
    ) -> TSSignature<'a> {
        let key = if computed {
            // `[Foo]: T` - key is an expression
            PropertyKey::new_identifier(SPAN, key, &self.ast)
        } else {
            self.property_key(key)
        };
        TSSignature::new_ts_property_signature(
            SPAN,
            computed,
            optional,
            readonly,
            key,
            Some(value),
            &self.ast,
        )
    }

    /// Key listed in `required` -> `key: T`
    pub fn prop(&self, key: &'a str, type_: TSType<'a>) -> TSSignature<'a> {
        self.property(key, self.annotation(type_), false, false, false)
    }

    /// Key not in `required` -> `key?: T`
    pub fn optional_prop(&self, key: &'a str, type_: TSType<'a>) -> TSSignature<'a> {
        self.property(key, self.annotation(type_), false, true, false)
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
            ArenaVec::<TSSignature<'a>>::from_iter_in(members.into_iter(), &self.ast),
            &self.ast,
        )
    }

    pub fn member_builder(&'a self) -> TsMembers<'a> {
        TsMembers::new(self)
    }
}

// ---------------------------------------------------------------------------
// Parsing
// ---------------------------------------------------------------------------
impl<'a> TypeBuilder<'a> {
    /// Parse a TS type from source, e.g. `GetFoo['request']['body']`. For
    /// user-supplied snippets that are easier to write than to build.
    pub fn parse_type(&self, source: &str) -> anyhow::Result<TSType<'a>> {
        let wrapped = self.str(&format!("type __T = {source};"));
        let ret = oxc_parser::Parser::new(self.allocator, wrapped, SourceType::ts()).parse();
        if ret.diagnostics.has_errors() || ret.fatal_error {
            let errors: Vec<_> = ret.diagnostics.errors().map(|e| e.to_string()).collect();
            anyhow::bail!("invalid TS type `{source}`: {}", errors.join("; "));
        }
        let mut body = ret.program.body.into_iter();
        match (body.next(), body.next()) {
            (Some(Statement::TSTypeAliasDeclaration(alias)), None) => {
                Ok(alias.unbox().type_annotation)
            }
            _ => anyhow::bail!("invalid TS type `{source}`: expected a single type"),
        }
    }
}

// ---------------------------------------------------------------------------
// Declarations
// ---------------------------------------------------------------------------
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

    /// `interface Name { ... }`
    pub fn interface(
        &self,
        name: &'a str,
        members: impl IntoIterator<Item = TSSignature<'a>>,
    ) -> Declaration<'a> {
        Declaration::new_ts_interface_declaration(
            SPAN,
            BindingIdentifier::new(SPAN, name, &self.ast),
            None,
            ArenaVec::new_in(&self.ast),
            TSInterfaceBody::boxed(SPAN, ArenaVec::from_iter_in(members, &self.ast), &self.ast),
            false,
            &self.ast,
        )
    }

    /// `export <declaration>`
    pub fn export(&self, declaration: Declaration<'a>) -> Statement<'a> {
        ModuleDeclaration::new_export_declaration(SPAN, declaration, &self.ast).into()
    }
}

// ---------------------------------------------------------------------------
// Member builder
// ---------------------------------------------------------------------------
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

    pub fn include<T: Into<TSSignature<'a>>>(
        &mut self,
        member: impl IntoIterator<Item = T>,
    ) -> &mut Self {
        self.members.extend(member.into_iter().map(|m| m.into()));
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

    /// Convenience for walking `properties` with a `required` list.
    pub fn field(self, key: &'a str, type_: TSType<'a>, required: bool) -> Self {
        if required {
            self.prop(key, type_)
        } else {
            self.optional_prop(key, type_)
        }
    }

    pub fn index_signature(self, value: TSType<'a>) -> Self {
        let member = self.builder.index_signature(value);
        self.with(member)
    }

    pub fn is_empty(&self) -> bool {
        self.members.is_empty()
    }

    pub fn into_literal(self) -> TSType<'a> {
        self.builder.type_literal(self.members)
    }

    pub fn into_interface(self, name: &'a str) -> Declaration<'a> {
        self.builder.interface(name, self.members)
    }
}

// ---------------------------------------------------------------------------
// Doc comments
//
// oxc's codegen prints comments by looking them up by the `span.start` of the
// node they're attached to, and slicing their text out of the program's source
// text. Since generated nodes have no source, we fake both: every documented
// node gets a unique synthetic span start, and the comment text is appended to
// a buffer that becomes the program's `source_text` at codegen time.
// ---------------------------------------------------------------------------
#[derive(Default)]
struct Docs {
    source: RefCell<String>,
    comments: RefCell<Vec<Comment>>,
    /// Offset from [`ANCHOR_BASE`] of the next anchor.
    next_anchor: Cell<u32>,
}

/// Synthetic anchors live far above any real offset, so they can't collide
/// with span 0 (every built node) or with nodes from [`TypeBuilder::parse_type`]
/// (whose spans are offsets into short snippets).
const ANCHOR_BASE: u32 = 1 << 31;

impl<'a> TypeBuilder<'a> {
    /// Attach a `/** ... */` comment to a node. `lines` are the raw lines of
    /// the comment body (e.g. `["@description The component"]`).
    pub fn with_doc<N: GetSpanMut>(&self, mut node: N, lines: &[String]) -> N {
        if lines.is_empty() {
            return node;
        }
        let offset = self.docs.next_anchor.get();
        self.docs.next_anchor.set(offset + 1);
        let anchor = ANCHOR_BASE + offset;
        *node.span_mut() = Span::new(anchor, anchor);

        let text = jsdoc_text(lines);
        let mut source = self.docs.source.borrow_mut();
        let start = source.len() as u32;
        source.push_str(&text);
        let end = source.len() as u32;

        let mut comment = Comment::new(
            start,
            end,
            if lines.len() > 1 {
                CommentKind::MultiLineBlock
            } else {
                CommentKind::SingleLineBlock
            },
        );
        comment.attached_to = anchor;
        comment.position = CommentPosition::Leading;
        comment.content = CommentContent::Jsdoc;
        comment.newlines = CommentNewlines::Leading | CommentNewlines::Trailing;
        self.docs.comments.borrow_mut().push(comment);
        node
    }
}

fn jsdoc_text(lines: &[String]) -> String {
    let escape = |l: &String| l.replace("*/", "*\\/");
    match lines {
        [line] => format!("/** {} */", escape(line)),
        lines => {
            let mut out = String::from("/**\n");
            for line in lines {
                out.push_str(" * ");
                out.push_str(&escape(line));
                out.push('\n');
            }
            out.push_str(" */");
            out
        }
    }
}

// ---------------------------------------------------------------------------
// Codegen
// ---------------------------------------------------------------------------
impl<'a> TypeBuilder<'a> {
    pub fn code_gen<I: Into<Statement<'a>>>(&self, body: impl IntoIterator<Item = I>) -> String {
        self.code_gen_with(CodegenOptions::default(), body)
    }

    pub fn code_gen_with<I: Into<Statement<'a>>>(
        &self,
        options: CodegenOptions,
        body: impl IntoIterator<Item = I>,
    ) -> String {
        let source_text = self.str(&self.docs.source.borrow());
        let comments = self.docs.comments.borrow();
        let program = Program::new(
            SPAN,
            SourceType::ts(),
            source_text,
            ArenaVec::from_iter_in(comments.iter().copied(), &self.ast),
            None,
            ArenaVec::new_in(&self.ast),
            ArenaVec::from_iter_in(body.into_iter().map(|i| i.into()), &self.ast),
            &self.ast,
        );

        Codegen::new().with_options(options).build(&program).code
    }
}

/// ASCII-only identifier check. Non-ASCII keys get quoted, which is always
/// valid TS, so there's no need to pull in `oxc_syntax` for full Unicode rules.
fn is_identifier(s: &str) -> bool {
    let mut chars = s.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_alphabetic() || c == '_' || c == '$')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$')
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
