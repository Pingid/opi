//! `ir::Operation` -> one exported type describing the whole endpoint:
//!
//! ```ts
//! export type GetComponentsByName = {
//!   method: "GET"
//!   path: "/components/{name}"
//!   request: { body?: never; contentType?: never; params: { name: string }; ... }
//!   response:
//!     | { status: 200; contentType: "application/json"; body: Component }
//!     | { status: 204; contentType: null; body?: never }
//! }
//! ```
//!
//! `request` and `response` are unions over the operation's
//! [`Request`](crate::ir::Request)s / [`Response`]s, every variant with the same
//! keys (`?: never` where one doesn't apply) so they can be narrowed on.

use oxc_ast::ast::*;

use super::{DocLines, Generator};
use crate::builder::TsMembers;
use crate::ir::{Operation, Param, Params, Response, Status};

/// The keys of a request variant after `body` / `contentType`, in order.
const PARAM_KEYS: [&str; 4] = ["params", "query", "headers", "cookies"];

struct ParamGroup<'a> {
    ty: TSType<'a>,
    required: bool,
    /// For the member, when the group is a single param (3.2 `querystring`).
    docs: Vec<String>,
}

impl<'a> Generator<'a> {
    pub(super) fn operation(&self, name: &str, op: &Operation) -> Statement<'a> {
        let b = self.b;
        let ty = b
            .member_builder()
            .prop("method", b.string_literal(b.str(op.method.as_str())))
            .prop("path", b.string_literal(b.str(&op.path)))
            .prop("request", self.request(op))
            .prop("response", self.response(op))
            .into_literal();

        self.export_type(name, ty, DocLines::operation(op).done())
    }

    fn request(&self, op: &Operation) -> TSType<'a> {
        let b = self.b;
        let variants = op.requests.iter().map(|variant| {
            let body = variant.body.as_ref().map(|schema| self.schema(schema));
            let members = match (&variant.content_type, body) {
                (Some(content_type), Some(body)) => {
                    let body = b.prop("body", body);
                    let docs = DocLines::description(variant.description.as_deref()).done();
                    b.member_builder()
                        .with(self.doc(body, docs))
                        .prop("contentType", b.string_literal(b.str(content_type)))
                }
                _ => b
                    .member_builder()
                    .optional_prop("body", b.never())
                    .optional_prop("contentType", b.never()),
            };
            PARAM_KEYS
                .into_iter()
                .zip(self.param_groups(&op.params))
                .fold(members, |members, (key, group)| {
                    self.param_member(members, key, group)
                })
                .into_literal()
        });
        b.union(variants)
    }

    /// One per [`PARAM_KEYS`] entry; `None` when there are no such params.
    fn param_groups(&self, params: &Params) -> [Option<ParamGroup<'a>>; 4] {
        let query = match &params.querystring {
            // A 3.2 `querystring` param is the whole query.
            Some(qs) => self.whole_param(qs),
            None => self.param_group(&params.query),
        };
        [
            self.param_group(&params.path),
            query,
            self.param_group(&params.header),
            self.param_group(&params.cookie),
        ]
    }

    /// `key: T` / `key?: T`, or `key?: never` if there are no params.
    fn param_member(
        &self,
        members: TsMembers<'a>,
        key: &'a str,
        group: Option<ParamGroup<'a>>,
    ) -> TsMembers<'a> {
        let b = self.b;
        match group {
            None => members.optional_prop(key, b.never()),
            Some(group) => {
                let member = b.field(key, group.ty, group.required);
                members.with(self.doc(member, group.docs))
            }
        }
    }

    /// A param that is the whole group.
    fn whole_param(&self, param: &Param) -> Option<ParamGroup<'a>> {
        Some(ParamGroup {
            ty: self.schema(&param.schema),
            required: param.required,
            docs: DocLines::param(param).done(),
        })
    }

    /// `{ a: string; b?: number }`, required if any param in it is.
    fn param_group(&self, params: &[Param]) -> Option<ParamGroup<'a>> {
        let b = self.b;
        if params.is_empty() {
            return None;
        }
        let fields = params.iter().map(|param| {
            let field = b.field(
                b.str(&param.name),
                self.schema(&param.schema),
                param.required,
            );
            self.doc(field, DocLines::param(param).done())
        });
        Some(ParamGroup {
            ty: b.type_literal(fields),
            required: params.iter().any(|p| p.required),
            docs: Vec::new(),
        })
    }

    fn response(&self, op: &Operation) -> TSType<'a> {
        self.b
            .union(op.responses.iter().map(|v| self.response_variant(v)))
    }

    /// `{ status: 200; contentType: "application/json"; body: T }`. `@stream`:
    /// `body` is the type of each item of a sequential body (OpenAPI 3.2
    /// `itemSchema`).
    fn response_variant(&self, variant: &Response) -> TSType<'a> {
        let b = self.b;
        let status = b.prop("status", self.status(variant.status));
        let docs = DocLines::description(variant.description.as_deref()).done();
        let content_type = match &variant.content_type {
            Some(content_type) => b.string_literal(b.str(content_type)),
            None => b.null(),
        };
        let members = b
            .member_builder()
            .with(self.doc(status, docs))
            .prop("contentType", content_type);
        match &variant.body {
            Some(schema) => {
                let body = b.prop("body", self.schema(schema));
                members.with(self.doc(
                    body,
                    DocLines::default().flag("stream", variant.stream).done(),
                ))
            }
            None => members.optional_prop("body", b.never()),
        }
        .into_literal()
    }

    /// `200`, `"2XX"`, `"default"`.
    pub(super) fn status(&self, status: Status) -> TSType<'a> {
        let b = self.b;
        match status {
            Status::Code(code) => b.number_literal(code.into()),
            status => b.string_literal(b.str(&status.to_string())),
        }
    }
}
