use super::*;
use crate::ir::{Method, Operation, Params, Request, Response, Schema, Status};

#[test]
fn globs() {
    assert!(path_matches("/shop/**", "/shop/items/{id}"));
    assert!(path_matches("/shop/**", "/shop"));
    assert!(path_matches("**/shop/**", "/api/shop/items"));
    assert!(path_matches("/shop/*", "/shop/items"));
    assert!(!path_matches("/shop/*", "/shop/items/{id}"));
    assert!(path_matches("/v*/users", "/v2/users"));
    assert!(path_matches("/items/{id}", "/items/{id}"));
    assert!(!path_matches("/shop/**", "/shopping"));
}

fn with_body(content_type: &str) -> Request {
    Request {
        content_type: Some(content_type.into()),
        body: Some(Schema::unknown()),
        stream: false,
        description: None,
    }
}

fn op() -> Operation {
    Operation {
        id: Some("createItem".into()),
        method: Method::Post,
        path: "/shop/items".into(),
        summary: None,
        description: None,
        deprecated: true,
        tags: vec!["shop".into(), "admin".into()],
        params: Params::default(),
        // An optional JSON body: with it, and without.
        requests: vec![with_body("application/json"), Request::bodiless()],
        responses: vec![
            Response::bodiless(Status::Code(201), None),
            Response {
                status: Status::Range(4),
                content_type: Some("application/problem+json".into()),
                body: Some(Schema::unknown()),
                stream: false,
                description: None,
            },
        ],
    }
}

fn parse(yaml: &str) -> Where {
    let w: Where = serde_yaml::from_str(yaml).unwrap();
    w.validate(Scope::Operation, "test").unwrap();
    w
}

#[test]
fn matching() {
    let op = op();
    let yes = |y: &str| assert!(parse(y).matches(&op), "{y} should match");
    let no = |y: &str| assert!(!parse(y).matches(&op), "{y} shouldn't match");
    yes("{}");
    yes("{ method: [get, POST], tag: sh*, operationId: create* }");
    yes("{ request: { content_type: application/json } }");
    yes("{ request: { not: { content_type: '*' } } }"); // optional body
    yes("{ response: { status: 2xx } }");
    yes("{ response: { status: 4xx, content_type: '*json' } }");
    no("{ response: { status: 404 } }");
    no("{ response: { status: [200, default] } }");
    yes("{ any: [{ tag: billing }, { path: '/shop/**' }] }");
    no("{ any: [{ tag: billing }, { path: '/admin/**' }] }");
    no("{ not: { deprecated: true } }");
    no("{ method: GET, deprecated: true }");
}

#[test]
fn mismatch_names_the_field() {
    let op = op();
    let field = |y: &str| parse(y).mismatch(&op);
    assert_eq!(
        field("{ deprecated: false }").as_deref(),
        Some("deprecated")
    );
    assert_eq!(field("{ not: { tag: shop } }").as_deref(), Some("not.tag"));
    assert_eq!(
        field("{ request: { content_type: text/plain } }").as_deref(),
        Some("request.content_type")
    );
    assert_eq!(field("{ path: '/shop/*' }"), None);
    // `not` is blamed before the nested selectors, whatever the struct order.
    assert_eq!(
        field("{ not: { tag: shop }, request: { content_type: text/plain } }").as_deref(),
        Some("not.tag")
    );
}

#[test]
fn validation() {
    let err = |y: &str, scope| {
        let w: Where = serde_yaml::from_str(y).unwrap();
        format!("{:#}", w.validate(scope, "filter").unwrap_err())
    };
    assert!(err("{ status: 200 }", Scope::Operation).contains("`status` isn't available"));
    assert!(err("{ method: GTE }", Scope::Operation).contains("unknown method"));
    assert!(err("{ response: { status: 2xxx } }", Scope::Operation).contains("unknown status"));
    assert!(err("{ request: { status: 200 } }", Scope::Operation).contains("filter.request"));
    assert!(err("{ any: [{ path: /a }] }", Scope::Schema).contains("filter.any[0]"));
    assert!(serde_yaml::from_str::<Where>("{ has_body: true }").is_err());
}
