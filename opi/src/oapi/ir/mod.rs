use anyhow::{Context, Result, bail};
use indexmap::IndexMap;

use crate::ast;
use crate::oapi;

mod schema;

pub fn lower(document: oapi::Document) -> Result<ast::Doc> {
    let oapi::Document {
        paths, components, ..
    } = document;
    let routes = collect_routes(&paths, &components)?;
    let schemas = components
        .schemas
        .iter()
        .map(|(name, s)| ast::Component {
            name: name.clone(),
            schema: schema::lower(s),
        })
        .collect();
    Ok(ast::Doc { routes, schemas })
}

fn collect_routes(
    paths: &IndexMap<String, oapi::PathItem>,
    components: &oapi::Components,
) -> Result<Vec<ast::Route>> {
    let mut routes = vec![];
    for (path, item) in paths {
        let item = oapi::resolve(components, item).with_context(|| format!("resolving {path}"))?;
        for (method, operation) in item.operations() {
            let route = lower_route(components, path, &method, &item.parameters, operation)
                .with_context(|| format!("{} {path}", method.to_uppercase()))?;
            routes.push(route);
        }
    }
    Ok(routes)
}

fn lower_route(
    components: &oapi::Components,
    path: &str,
    method: &str,
    path_params: &[oapi::Parameter],
    operation: &oapi::Operation,
) -> Result<ast::Route> {
    let body = match &operation.request_body {
        Some(body) => Some(oapi::resolve(components, body).context("resolving request body")?),
        None => None,
    };
    Ok(ast::Route {
        path: path.to_string(),
        method: method.into(),
        summary: operation.summary.clone(),
        description: operation.description.clone(),
        deprecated: Some(operation.deprecated),
        params: lower_params(components, path_params, &operation.parameters)?,
        body_required: body.is_some_and(|b| b.required),
        request: body.map(lower_requests).unwrap_or_default(),
        response: lower_responses(components, &operation.responses)?,
    })
}

/// Operation-level parameters override path-level ones with the same
/// `(in, name)`, in the path-level one's position.
fn lower_params(
    components: &oapi::Components,
    path_params: &[oapi::Parameter],
    op_params: &[oapi::Parameter],
) -> Result<ast::Params> {
    use oapi::Location;

    let mut merged = IndexMap::new();
    for param in path_params.iter().chain(op_params) {
        let param = oapi::resolve(components, param).context("resolving parameter")?;
        let location = match param.location {
            Some(Location::Unknown) | None => {
                bail!("parameter {:?} has no known `in`", param.name)
            }
            Some(location) => location,
        };
        merged.insert((location, param.name.as_str()), param);
    }

    let mut params = ast::Params::default();
    for ((location, _), param) in merged {
        let lowered = ast::Param {
            name: param.name.clone(),
            // Path params are always required, whatever the spec says.
            required: param.required || location == Location::Path,
            description: param.description.clone(),
            deprecated: param.deprecated,
            schema: param_schema(param),
        };
        match location {
            Location::Path => params.path.push(lowered),
            Location::Query => params.query.push(lowered),
            Location::Querystring => params.querystring = Some(lowered),
            Location::Header => params.header.push(lowered),
            Location::Cookie => params.cookie.push(lowered),
            Location::Unknown => unreachable!("rejected above"),
        }
    }
    Ok(params)
}

/// `schema`, else the first `content` media type's, else unknown.
fn param_schema(param: &oapi::Parameter) -> ast::Schema {
    if let Some(schema) = &param.schema {
        return schema::lower(schema);
    }
    param
        .content
        .values()
        .next()
        .and_then(|media| media_schema(media).0)
        .unwrap_or_else(ast::Schema::unknown)
}

/// `itemSchema` (3.2 streams) wins over `schema`: it's the type a client
/// handles, one item at a time. The flag says which it was.
fn media_schema(media: &oapi::MediaType) -> (Option<ast::Schema>, bool) {
    match &media.item_schema {
        Some(item) => (Some(schema::lower(item)), true),
        None => (media.schema.as_ref().map(schema::lower), false),
    }
}

fn lower_requests(body: &oapi::RequestBody) -> Vec<ast::Request> {
    body.content
        .iter()
        .map(|(content_type, media)| {
            let (schema, stream) = media_schema(media);
            ast::Request {
                content_type: content_type.clone(),
                deprecated: None,
                body: schema,
                stream,
                description: body.description.clone(),
            }
        })
        .collect()
}

/// One response per status and media type, in spec order; a status without
/// content (e.g. 204) is one response without a content type.
fn lower_responses(
    components: &oapi::Components,
    responses: &IndexMap<String, oapi::Response>,
) -> Result<Vec<ast::Response>> {
    let mut out = vec![];
    for (status, response) in responses {
        let status: ast::Status = status.parse()?;
        let response = oapi::resolve(components, response)
            .with_context(|| format!("resolving {status} response"))?;
        if response.content.is_empty() {
            out.push(ast::Response {
                status,
                content_type: None,
                schema: None,
                stream: false,
                description: response.description.clone(),
                deprecated: None,
            });
        }
        for (content_type, media) in &response.content {
            let (schema, stream) = media_schema(media);
            out.push(ast::Response {
                status,
                content_type: Some(content_type.clone()),
                schema,
                stream,
                description: response.description.clone(),
                deprecated: None,
            });
        }
    }
    Ok(out)
}
