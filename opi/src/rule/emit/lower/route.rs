//! A route's `{request}` and `{response}`:
//!
//! ```ts
//! request:
//!   | { body: Component; contentType: "application/json"; params: { name: string }; query?: never; ... }
//!   | { body?: never; contentType?: never; params: { name: string }; query?: never; ... }
//! response:
//!   | { status: 200; contentType: "application/json"; body: Component }
//!   | { status: 204; contentType: null; body?: never }
//! ```
//!
//! Every variant has the same keys (`?: never` where one doesn't apply) so
//! the unions narrow on `contentType` / `status`.

use oxc_ast::ast::{TSSignature, TSType};

use super::super::docs::DocLines;
use super::Lower;
use crate::ast::{Param, Params, Route, Status};

/// The keys of a request variant after `body` / `contentType`, in order.
const PARAM_KEYS: [&str; 4] = ["params", "query", "headers", "cookies"];

/// One of [`PARAM_KEYS`]: its type, whether it's required, its docs.
type Group<'b> = (TSType<'b>, bool, Vec<String>);

impl<'b> Lower<'_, '_, 'b> {
    /// One variant per body media type, plus one without a body when it's
    /// optional or there is none.
    pub(super) fn request(&mut self, route: &Route) -> TSType<'b> {
        let tb = self.tb;
        let mut variants = Vec::with_capacity(route.request.len() + 1);
        for request in &route.request {
            let body = match &request.body {
                Some(schema) => self.schema(schema),
                None => tb.unknown(),
            };
            let docs = DocLines::default()
                .tag("description", request.description.as_deref())
                .flag("stream", request.stream)
                .done();
            let mut members = vec![
                tb.with_doc(tb.prop("body", body), &docs),
                tb.prop(
                    "contentType",
                    tb.string_literal(tb.str(&request.content_type)),
                ),
            ];
            members.extend(self.params(&route.params));
            variants.push(tb.type_literal(members));
        }
        if !route.body_required || route.request.is_empty() {
            let mut members = vec![
                tb.optional_prop("body", tb.never()),
                tb.optional_prop("contentType", tb.never()),
            ];
            members.extend(self.params(&route.params));
            variants.push(tb.type_literal(members));
        }
        tb.union(variants)
    }

    /// `{ status; contentType; body }` per status and media type.
    pub(super) fn response(&mut self, route: &Route) -> TSType<'b> {
        let tb = self.tb;
        let mut variants = Vec::with_capacity(route.response.len());
        for response in &route.response {
            let status = match response.status {
                Status::Code(code) => tb.number_literal(code.into()),
                status => tb.string_literal(tb.str(&status.to_string())),
            };
            let docs = DocLines::default()
                .tag("description", response.description.as_deref())
                .done();
            let status = tb.with_doc(tb.prop("status", status), &docs);
            let (content_type, body) = match &response.content_type {
                None => (tb.null(), tb.optional_prop("body", tb.never())),
                Some(content_type) => {
                    let body = match &response.schema {
                        Some(schema) => self.schema(schema),
                        None => tb.unknown(),
                    };
                    let docs = DocLines::default().flag("stream", response.stream).done();
                    let body = tb.with_doc(tb.prop("body", body), &docs);
                    (tb.string_literal(tb.str(content_type)), body)
                }
            };
            variants.push(tb.type_literal([status, tb.prop("contentType", content_type), body]));
        }
        tb.union(variants)
    }

    /// `params`, `query`, `headers`, `cookies`: each `key: {...}` (`key?:`
    /// when none of its params is required), or `key?: never`.
    fn params(&mut self, params: &Params) -> Vec<TSSignature<'b>> {
        let tb = self.tb;
        let query = match &params.querystring {
            // A 3.2 `querystring` param is the whole query.
            Some(param) => Some(self.whole_param(param)),
            None => self.param_group(&params.query),
        };
        let groups = [
            self.param_group(&params.path),
            query,
            self.param_group(&params.header),
            self.param_group(&params.cookie),
        ];
        PARAM_KEYS
            .into_iter()
            .zip(groups)
            .map(|(key, group)| match group {
                None => tb.optional_prop(key, tb.never()),
                Some((ty, required, docs)) => tb.with_doc(tb.field(key, ty, required), &docs),
            })
            .collect()
    }

    fn whole_param(&mut self, param: &Param) -> Group<'b> {
        let ty = self.schema(&param.schema);
        (ty, param.required, DocLines::param(param).done())
    }

    /// `{ a: string; b?: number }`, required if any param in it is.
    fn param_group(&mut self, params: &[Param]) -> Option<Group<'b>> {
        if params.is_empty() {
            return None;
        }
        let tb = self.tb;
        let mut fields = Vec::with_capacity(params.len());
        for param in params {
            let ty = self.schema(&param.schema);
            let field = tb.field(tb.str(&param.name), ty, param.required);
            fields.push(tb.with_doc(field, &DocLines::param(param).done()));
        }
        let required = params.iter().any(|p| p.required);
        Some((tb.type_literal(fields), required, Vec::new()))
    }
}
