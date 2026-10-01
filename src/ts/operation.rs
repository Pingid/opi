//! `ir::Operation` -> one exported type describing the whole endpoint:
//!
//! ```ts
//! export type GetComponentsByName = {
//!   method: 'GET'
//!   path: '/components/{name}'
//!   request: { body?: never; contentType?: never; params: { name: string }; ... }
//!   response: { 'application/json': { 200: Component; 404: Error } }
//! }
//! ```

use indexmap::IndexMap;
use oxc_ast::ast::*;

use super::{DocLines, Generator};
use crate::config::{ParamsLayout, ResponseLayout};
use crate::ir::{Operation, Param, Params, RequestBody, Response, Schema};
use crate::builder::TsMembers;

impl<'a> Generator<'a> {
    pub(super) fn operation(&self, name: &str, op: &Operation) -> Statement<'a> {
        let b = self.b;
        let ty = b
            .member_builder()
            .prop("method", b.string_literal(b.str(op.method.as_str())))
            .prop("path", b.string_literal(b.str(&op.path)))
            .prop("request", self.request(op))
            .prop("response", self.response(&op.responses))
            .into_literal();

        let docs = DocLines::default()
            .tag("summary", op.summary.as_deref())
            .tag("description", op.description.as_deref())
            .flag("deprecated", op.deprecated)
            .done();
        self.export_type(name, ty, docs)
    }

    /// One object per request content type, unioned. Keys that don't apply are
    /// present as `?: never` so every variant has the same shape to narrow on.
    fn request(&self, op: &Operation) -> TSType<'a> {
        let b = self.b;
        let variants = body_variants(op.body.as_ref()).into_iter().map(|variant| {
            let members = match variant {
                Some((content_type, schema)) => {
                    let description = op.body.as_ref().and_then(|b| b.description.as_deref());
                    let body = b.prop("body", self.schema(schema));
                    b.member_builder()
                        .with(self.doc(body, DocLines::default().tag("description", description).done()))
                        .prop("contentType", b.string_literal(b.str(content_type)))
                }
                None => b
                    .member_builder()
                    .optional_prop("body", b.never())
                    .optional_prop("contentType", b.never()),
            };
            self.param_groups(&op.params)
                .into_iter()
                .fold(members, |members, (key, group)| match group {
                    ParamGroup::Fields(params) => self.param_group(members, key, &params),
                    ParamGroup::Whole(param) => self.whole_param(members, key, param),
                })
                .into_literal()
        });
        b.union(variants)
    }

    fn param_groups<'p>(&self, params: &'p Params) -> Vec<(&'static str, ParamGroup<'p>)> {
        let path = params.path.iter();
        let query = params.query.iter();
        let mut groups = match (self.config.operation.params, &params.querystring) {
            // A 3.2 `querystring` param is the whole query, so it gets its own
            // key whatever the layout.
            (ParamsLayout::Split, Some(qs)) | (ParamsLayout::Merged, Some(qs)) => vec![
                ("params", ParamGroup::Fields(path.collect())),
                ("query", ParamGroup::Whole(qs)),
            ],
            (ParamsLayout::Split, None) => vec![
                ("params", ParamGroup::Fields(path.collect())),
                ("query", ParamGroup::Fields(query.collect())),
            ],
            (ParamsLayout::Merged, None) => {
                vec![("params", ParamGroup::Fields(path.chain(query).collect()))]
            }
        };
        groups.push(("headers", ParamGroup::Fields(params.header.iter().collect())));
        groups.push(("cookies", ParamGroup::Fields(params.cookie.iter().collect())));
        groups
    }

    /// `key: Schema`, for a param that is the whole group.
    fn whole_param(&self, members: TsMembers<'a>, key: &'a str, param: &Param) -> TsMembers<'a> {
        let docs = DocLines::default()
            .tag("description", param.description.as_deref())
            .flag("deprecated", param.deprecated)
            .done();
        let b = self.b;
        let ty = self.schema(&param.schema);
        let member = if param.required { b.prop(key, ty) } else { b.optional_prop(key, ty) };
        members.with(self.doc(member, docs))
    }

    /// `key: { a: string; b?: number }`, optional if no param in it is required,
    /// and `key?: never` if there are no params at all.
    fn param_group(&self, members: TsMembers<'a>, key: &'a str, params: &[&Param]) -> TsMembers<'a> {
        let b = self.b;
        if params.is_empty() {
            return members.optional_prop(key, b.never());
        }
        let fields = params.iter().map(|param| {
            let field = b.property(
                b.str(&param.name),
                b.annotation(self.schema(&param.schema)),
                false,
                !param.required,
                false,
            );
            let docs = DocLines::default()
                .tag("description", param.description.as_deref())
                .flag("deprecated", param.deprecated)
                .done();
            self.doc(field, docs)
        });
        let ty = b.type_literal(fields);
        members.field(key, ty, params.iter().any(|p| p.required))
    }

    fn response(&self, responses: &[Response]) -> TSType<'a> {
        match self.config.operation.response {
            ResponseLayout::ByContentType => self.response_by_content_type(responses),
            ResponseLayout::ByStatus => self.response_by_status(responses),
        }
    }

    /// `{ 'application/json': { 200: A; 404: B }, none: { 204: never } }`
    fn response_by_content_type(&self, responses: &[Response]) -> TSType<'a> {
        let b = self.b;
        let mut by_content_type: IndexMap<&str, Vec<TSSignature<'a>>> = IndexMap::new();
        for response in responses {
            if response.content.is_empty() {
                by_content_type
                    .entry(self.config.operation.no_content_key.as_str())
                    .or_default()
                    .push(self.status_member(response, b.never(), false));
            }
            for (content_type, content) in &response.content {
                let member = self.status_member(response, self.schema(&content.schema), content.stream);
                by_content_type.entry(content_type).or_default().push(member);
            }
        }
        b.type_literal(
            by_content_type
                .into_iter()
                .map(|(content_type, statuses)| b.prop(b.str(content_type), b.type_literal(statuses))),
        )
    }

    /// `{ 200: { 'application/json': A }, 204: {} }`
    fn response_by_status(&self, responses: &[Response]) -> TSType<'a> {
        let b = self.b;
        b.type_literal(responses.iter().map(|response| {
            let content = response.content.iter().map(|(content_type, content)| {
                let prop = b.prop(b.str(content_type), self.schema(&content.schema));
                self.doc(prop, DocLines::default().flag("stream", content.stream).done())
            });
            self.status_member(response, b.type_literal(content), false)
        }))
    }

    /// `stream`: `ty` is the type of each item of a sequential body (OpenAPI
    /// 3.2 `itemSchema`), flagged `@stream`.
    fn status_member(&self, response: &Response, ty: TSType<'a>, stream: bool) -> TSSignature<'a> {
        let b = self.b;
        let member = b.prop(b.str(&response.status.to_string()), ty);
        let docs = DocLines::default()
            .tag("description", response.description.as_deref())
            .flag("stream", stream)
            .done();
        self.doc(member, docs)
    }
}

/// A request key's params: one field per param, or (3.2 `querystring`) one
/// param typed as the whole value.
enum ParamGroup<'p> {
    Fields(Vec<&'p Param>),
    Whole(&'p Param),
}

/// `Some((content_type, schema))` per body media type, plus `None` for "no
/// body" when there's no body or it's optional.
fn body_variants(body: Option<&RequestBody>) -> Vec<Option<(&str, &Schema)>> {
    let Some(body) = body else {
        return vec![None];
    };
    let mut variants: Vec<_> = body
        .content
        .iter()
        .map(|(content_type, content)| Some((content_type.as_str(), &content.schema)))
        .collect();
    if !body.required || variants.is_empty() {
        variants.push(None);
    }
    variants
}
