use super::*;

fn render(t: &str) -> Option<String> {
    Template::parse(t).unwrap().render(|v| match v {
        Var::Method => Some("get".into()),
        Var::Path => Some("/components/{name}".into()),
        Var::Tags => Some("shop-api".into()),
        Var::RequestContentType => Some("application/json".into()),
        _ => None,
    })
}

#[test]
fn cases_and_filters() {
    assert_eq!(render("{Method}{Path}").unwrap(), "GetComponentsByName");
    assert_eq!(render("{METHOD} {path}").unwrap(), "GET /components/{name}");
    assert_eq!(render("{tags|camel}Routes").unwrap(), "shopApiRoutes");
    assert_eq!(render("{Tags}").unwrap(), "ShopApi");
    assert_eq!(render("{{literal}}").unwrap(), "{literal}");
    assert_eq!(render("{id}"), None);
    assert_eq!(render("{TYPE_NAME}"), None);
    assert_eq!(
        render("{request.content_type|pascal}Ops").unwrap(),
        "ApplicationJsonOps"
    );
    assert_eq!(
        render("{REQUEST.CONTENT_TYPE}").unwrap(),
        "APPLICATION/JSON"
    );
}

#[test]
fn single_var() {
    let var = |t: &str| Template::parse(t).unwrap().single_var();
    assert_eq!(var("{request.body}"), Some(Var::RequestBody));
    assert_eq!(var("{type_name}"), Some(Var::TypeName));
    assert_eq!(var("{METHOD}"), None);
    assert_eq!(var("{type_name}[]"), None);
    assert_eq!(var("{type_name|pascal}"), None);
}

#[test]
fn errors() {
    assert!(Template::parse("{nope}").is_err());
    assert!(Template::parse("{method|shout}").is_err());
    assert!(Template::parse("oops}").is_err());
    assert!(Template::parse("{method").is_err());
    let renamed = Template::parse("{op.request}").unwrap_err().to_string();
    assert!(renamed.contains("it's `{request}` now"), "{renamed}");
    let t = Template::parse("{tags}").unwrap();
    assert!(t.check(&[Var::Method], "test").is_err());
}
