use crate::ts::generate;
use crate::{Config, Spec};

const SPEC: &str = r#"
openapi: 3.1.0
info: { title: t, version: "1" }
paths:
  /a:
    post:
      requestBody:
        content:
          application/json: { schema: { type: string } }
          text/plain: { schema: { type: string } }
      responses:
        "200":
          content:
            application/json: { schema: { type: string } }
            application/xml: { schema: { type: string } }
        "404":
          content:
            application/json: { schema: { type: number } }
        "204": { description: none }
"#;

/// The `X` member of `emit: [{ name: T, shape: { X: <value> } }]`.
fn value(value: &str) -> String {
    let config = Config::from_yaml(&format!(
        "header: ''\nemit:\n- name: T\n  shape:\n    X: {value}"
    ))
    .unwrap();
    let api = Spec::from_yaml(SPEC).unwrap().lower().unwrap();
    let code = generate(&api, &config).unwrap();
    let start = code.find("export type T = {").unwrap();
    let member = code[start..].split_once("X: ").unwrap().1;
    let member = member.rsplit_once(";\n}").unwrap().0;
    member.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[test]
fn narrowing() {
    let response = |w: &str| value(&format!("{{ ref: '{{response}}', where: {w} }}"));
    let request = |w: &str| value(&format!("{{ ref: '{{request}}', where: {w} }}"));

    assert_eq!(response("{}"), r#"PostA["response"]"#);
    assert_eq!(response("{ status: 500 }"), "never");
    assert_eq!(
        response("{ content_type: '*json' }"),
        r#"Extract<PostA["response"], { contentType: "application/json"; }>"#
    );
    assert_eq!(
        response("{ status: 2xx }"),
        r#"Extract<PostA["response"], { status: 200 | 204; }>"#
    );
    assert_eq!(
        response("{ status: 200, content_type: '*json' }"),
        r#"Extract<PostA["response"], { status: 200; contentType: "application/json"; }>"#
    );
    assert_eq!(
        response("{ not: { content_type: '*' } }"),
        r#"Extract<PostA["response"], { contentType: null; }>"#
    );
    // No field set picks out exactly these two: one discriminant each.
    assert_eq!(
        response("{ any: [{ status: 200, content_type: '*xml' }, { status: 404 }] }"),
        r#"Extract<PostA["response"], { status: 200; contentType: "application/xml"; } | { status: 404; contentType: "application/json"; }>"#
    );

    assert_eq!(
        request("{ content_type: text/plain }"),
        r#"Extract<PostA["request"], { contentType: "text/plain"; }>"#
    );
    // The optional body's "no body" variant.
    assert_eq!(
        request("{ not: { content_type: text/* } }"),
        r#"Extract<PostA["request"], { contentType?: "application/json" | undefined; }>"#
    );
}

/// The whole `T` type for `shape:` `shape`.
fn object(shape: &str) -> String {
    let config =
        Config::from_yaml(&format!("header: ''\nemit:\n- name: T\n  shape: {shape}")).unwrap();
    let api = Spec::from_yaml(SPEC).unwrap().lower().unwrap();
    let code = generate(&api, &config).unwrap();
    let start = code.find("export type T = {").unwrap();
    code[start..]
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

#[test]
fn references() {
    assert_eq!(value("'{type_name}'"), "PostA");
    assert_eq!(value("'{method}'"), r#"PostA["method"]"#);
    assert_eq!(value("'{request.body}'"), r#"PostA["request"]["body"]"#);
    assert_eq!(
        value("'{response.status}'"),
        r#"PostA["response"]["status"]"#
    );
    assert_eq!(
        value("{ ref: '{response.body}', where: { status: 404 } }"),
        r#"Extract<PostA["response"], { status: 404; }>["body"]"#
    );
    // Text, not a reference.
    assert_eq!(value("'{METHOD} {path}'"), r#""POST /a""#);
    assert_eq!(value("'{type_name}Op'"), r#""PostAOp""#);
    assert_eq!(value("'{response.status}!'"), r#""200!" | "404!" | "204!""#);
}

/// A key with a variable that has several values is repeated per value,
/// with the variable bound to it below.
#[test]
fn keys_fan_out() {
    assert_eq!(
        object("{ '{response.status}': { status: '{response.status}!', body: '{response.body}' } }"),
        [
            r#"export type T = {"#,
            r#"200: { status: "200!"; body: Extract<PostA["response"], { status: 200; }>["body"]; };"#,
            r#"404: { status: "404!"; body: Extract<PostA["response"], { status: 404; }>["body"]; };"#,
            r#"204: { status: "204!"; body: Extract<PostA["response"], { status: 204; }>["body"]; }; };"#,
        ]
        .join(" ")
    );
    assert_eq!(
        object("{ '{request.content_type}': '{request.body}' }"),
        [
            r#"export type T = {"#,
            r#""application/json": Extract<PostA["request"], { contentType: "application/json"; }>["body"];"#,
            r#""text/plain": Extract<PostA["request"], { contentType: "text/plain"; }>["body"]; };"#,
        ]
        .join(" ")
    );
}
