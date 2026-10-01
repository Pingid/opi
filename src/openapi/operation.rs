//! [`model::Operation`](super::model::Operation) -> [`ir::Operation`].

use anyhow::{Context, Result, bail};
use indexmap::IndexMap;

use super::model::{self, Components, MediaType, Parameter};
use super::refs::resolve;
use super::schema;
use crate::ir::{self, Location, Method, Param, Params, Request, Response, Status};

pub fn lower(
    components: &Components,
    path: &str,
    method: Method,
    path_params: &[Parameter],
    op: &model::Operation,
) -> Result<ir::Operation> {
    Ok(ir::Operation {
        id: op.operation_id.clone(),
        method,
        path: path.to_string(),
        summary: op.summary.clone(),
        description: op.description.clone(),
        deprecated: op.deprecated,
        tags: op.tags.clone(),
        params: lower_params(components, path_params, &op.parameters)?,
        requests: lower_requests(components, op.request_body.as_ref())?,
        responses: lower_responses(components, &op.responses)?,
    })
}

/// Operation-level parameters override path-level ones with the same
/// `(name, in)`.
fn lower_params(
    components: &Components,
    path_params: &[Parameter],
    op_params: &[Parameter],
) -> Result<Params> {
    let mut params = Params::default();
    for ((location, _), param) in merge_params(components, path_params, op_params)? {
        params.push(location, lower_param(param, location));
    }
    Ok(params)
}

/// Resolved and keyed by `(in, name)`: a later (operation-level) parameter
/// replaces an earlier (path-level) one in place.
fn merge_params<'s>(
    components: &'s Components,
    path_params: &'s [Parameter],
    op_params: &'s [Parameter],
) -> Result<IndexMap<(Location, &'s str), &'s Parameter>> {
    let mut merged = IndexMap::new();
    for param in path_params.iter().chain(op_params) {
        let param = resolve(components, param).context("resolving parameter")?;
        let Some(location) = param.location.and_then(model::Location::lower) else {
            bail!("parameter {:?} has no known `in`", param.name);
        };
        merged.insert((location, param.name.as_str()), param);
    }
    Ok(merged)
}

/// Path params are always required, whatever the spec says.
fn lower_param(param: &Parameter, location: Location) -> Param {
    Param {
        name: param.name.clone(),
        required: param.required || location == Location::Path,
        description: param.description.clone(),
        deprecated: param.deprecated,
        schema: param_schema(param),
    }
}

/// `schema`, else the first `content` media type's, else unknown.
fn param_schema(param: &Parameter) -> ir::Schema {
    match &param.schema {
        Some(schema) => schema::lower(schema),
        None => param
            .content
            .values()
            .next()
            .map_or_else(ir::Schema::unknown, |media| lower_media(media).0),
    }
}

/// One request per body media type, plus a bodiless one when the body is
/// missing or optional. Never empty.
fn lower_requests(
    components: &Components,
    body: Option<&model::RequestBody>,
) -> Result<Vec<Request>> {
    let Some(body) = body else {
        return Ok(vec![Request::bodiless()]);
    };
    let body = resolve(components, body).context("resolving request body")?;
    let mut requests: Vec<_> = body
        .content
        .iter()
        .map(|(media_type, media)| {
            let (schema, stream) = lower_media(media);
            Request {
                content_type: Some(media_type.clone()),
                body: Some(schema),
                stream,
                description: body.description.clone(),
            }
        })
        .collect();
    if !body.required || requests.is_empty() {
        requests.push(Request::bodiless());
    }
    Ok(requests)
}

/// One response per status and media type, in spec order; a status without
/// content (e.g. 204) is one bodiless response.
fn lower_responses(
    components: &Components,
    responses: &IndexMap<String, model::Response>,
) -> Result<Vec<Response>> {
    let mut out = Vec::new();
    for (status, response) in responses {
        let status: Status = status.parse()?;
        let response = resolve(components, response)
            .with_context(|| format!("resolving {status} response"))?;
        if response.content.is_empty() {
            out.push(Response::bodiless(status, response.description.clone()));
        }
        for (media_type, media) in &response.content {
            let (schema, stream) = lower_media(media);
            out.push(Response {
                status,
                content_type: Some(media_type.clone()),
                body: Some(schema),
                stream,
                description: response.description.clone(),
            });
        }
    }
    Ok(out)
}

/// A streaming media type's `itemSchema` wins over its `schema`: it's the
/// type a client actually handles.
fn lower_media(media: &MediaType) -> (ir::Schema, bool) {
    match &media.item_schema {
        Some(item) => (schema::lower(item), true),
        None => (schema::lower_opt(media.schema.as_ref()), false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn param(name: &str, location: model::Location, required: bool) -> Parameter {
        Parameter {
            name: name.into(),
            location: Some(location),
            required,
            ..Parameter::default()
        }
    }

    /// An operation-level param keeps the path-level one's position but
    /// takes its own value; path params are always required.
    #[test]
    fn operation_params_override_path_params() {
        use model::Location::{Header, Path, Query};
        let components = Components::default();
        let path = [param("a", Query, false), param("id", Path, false)];
        let op = [param("a", Query, true), param("x-h", Header, false)];
        let merged = merge_params(&components, &path, &op).unwrap();
        let keys: Vec<_> = merged.keys().map(|(_, name)| *name).collect();
        assert_eq!(keys, ["a", "id", "x-h"]);
        assert!(merged[&(Location::Query, "a")].required);

        let params = lower_params(&components, &path, &op).unwrap();
        let lowered: Vec<_> = params
            .iter()
            .map(|p| (p.name.as_str(), p.required))
            .collect();
        assert_eq!(lowered, [("id", true), ("a", true), ("x-h", false)]);
    }
}
