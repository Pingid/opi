//! [`model::Operation`](super::model::Operation) -> [`ir::Operation`].

use anyhow::{Context, Result, bail};
use indexmap::IndexMap;

use super::model::{self, Components, Location, MediaType, Parameter};
use super::refs::resolve;
use super::schema;
use crate::ir::{self, Content, Method, Param, Params, RequestBody, Response, Status};

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
        body: op
            .request_body
            .as_ref()
            .map(|body| lower_body(components, body))
            .transpose()?,
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
    let mut merged: IndexMap<(Location, &str), &Parameter> = IndexMap::new();
    for param in path_params.iter().chain(op_params) {
        let param = resolve(components, param).context("resolving parameter")?;
        let Some(location) = param.location.filter(|&l| l != Location::Unknown) else {
            bail!("parameter {:?} has no known `in`", param.name);
        };
        merged.insert((location, &param.name), param);
    }

    let mut params = Params::default();
    for ((location, _), param) in merged {
        let lowered = Param {
            name: param.name.clone(),
            // Path params are always required, whatever the spec says.
            required: param.required || location == Location::Path,
            description: param.description.clone(),
            deprecated: param.deprecated,
            schema: match &param.schema {
                Some(s) => schema::lower(s),
                None => lower_content(&param.content)
                    .into_values()
                    .next()
                    .map_or_else(ir::Schema::unknown, |c| c.schema),
            },
        };
        match location {
            Location::Path => params.path.push(lowered),
            Location::Query => params.query.push(lowered),
            Location::Querystring => params.querystring = Some(lowered),
            Location::Header => params.header.push(lowered),
            Location::Cookie => params.cookie.push(lowered),
            Location::Unknown => unreachable!("filtered above"),
        }
    }
    Ok(params)
}

fn lower_body(components: &Components, body: &model::RequestBody) -> Result<RequestBody> {
    let body = resolve(components, body).context("resolving request body")?;
    Ok(RequestBody {
        required: body.required,
        description: body.description.clone(),
        content: lower_content(&body.content),
    })
}

fn lower_responses(
    components: &Components,
    responses: &IndexMap<String, model::Response>,
) -> Result<Vec<Response>> {
    responses
        .iter()
        .map(|(status, response)| {
            let status = parse_status(status)?;
            let response = resolve(components, response)
                .with_context(|| format!("resolving {status} response"))?;
            Ok(Response {
                status,
                description: response.description.clone(),
                content: lower_content(&response.content),
            })
        })
        .collect()
}

/// `200`, `2XX` (any case) or `default`.
fn parse_status(status: &str) -> Result<Status> {
    if status == "default" {
        return Ok(Status::Default);
    }
    if let Ok(code) = status.parse() {
        return Ok(Status::Code(code));
    }
    match status.as_bytes() {
        [digit @ b'1'..=b'5', x, y] if x.eq_ignore_ascii_case(&b'X') && y.eq_ignore_ascii_case(&b'X') => {
            Ok(Status::Range(u16::from(digit - b'0')))
        }
        _ => bail!("invalid response status {status:?}"),
    }
}

/// A streaming media type's `itemSchema` wins over its `schema`: it's the
/// type a client actually handles.
fn lower_content(content: &IndexMap<String, MediaType>) -> IndexMap<String, Content> {
    content
        .iter()
        .map(|(media_type, media)| {
            let content = match &media.item_schema {
                Some(item) => Content {
                    schema: schema::lower(item),
                    stream: true,
                },
                None => Content {
                    schema: schema::lower_opt(media.schema.as_ref()),
                    stream: false,
                },
            };
            (media_type.clone(), content)
        })
        .collect()
}
