use super::shape::RefTarget;
use super::*;
use crate::template::Var;

/// The checked-in `config.default.yaml` loads back to the defaults.
#[test]
fn default_yaml_matches_default() {
    assert_eq!(
        Config::from_yaml(crate::DEFAULT_CONFIG_YAML).unwrap(),
        Config::default()
    );
}

#[test]
fn empty_is_default() {
    assert_eq!(Config::from_yaml("").unwrap(), Config::default());
    assert_eq!(Config::from_yaml("# nothing\n").unwrap(), Config::default());
    assert_eq!(Config::from_json("{}").unwrap(), Config::default());
}

#[test]
fn partial_sections_keep_defaults() {
    let config = Config::from_yaml("jsdoc: false\noperation:\n  overrides: []").unwrap();
    let default = Config::default();
    assert!(!config.jsdoc);
    assert_eq!(config.header, default.header);
    assert_eq!(config.formats, default.formats);
    assert_eq!(config.emit, default.emit);
    assert_eq!(config.operation.name, default.operation.name);
}

#[test]
fn json() {
    let config = Config::from_json(
        r#"{
          "$schema": "./config.schema.json",
          "filter": { "method": ["GET", "PUT"] },
          "emit": [{ "name": "All", "shape": "{type_name}" }]
        }"#,
    )
    .unwrap();
    assert_eq!(
        config.filter.method.as_ref().map(|m| m.as_slice().len()),
        Some(2)
    );
    assert_eq!(
        config.emit[0].shape,
        Shape::Value(Value::Template(template("{type_name}")))
    );
}

#[test]
fn values() {
    let value = |source: &str| {
        let config = Config::from_yaml(&format!("emit:\n- name: X\n  shape: {source}")).unwrap();
        let Shape::Value(value) = &config.emit[0].shape else {
            panic!("{source} isn't a value")
        };
        match value.resolve(Var::TEXT, "test").unwrap() {
            Resolved::Ref { target, .. } => Some((target.root, target.key)),
            Resolved::Literal(_) => None,
        }
    };
    assert_eq!(value("'{type_name}'"), Some((Root::Op, None)));
    assert_eq!(value("'{method}'"), Some((Root::Op, Some("method"))));
    assert_eq!(value("'{request}'"), Some((Root::Request, None)));
    assert_eq!(
        value("'{request.content_type}'"),
        Some((Root::Request, Some("contentType")))
    );
    assert_eq!(
        value("'{response.body}'"),
        Some((Root::Response, Some("body")))
    );
    assert_eq!(
        value("{ ref: '{response.status}', where: { content_type: '*' } }"),
        Some((Root::Response, Some("status")))
    );
    // Anything else is text.
    assert_eq!(value("'{METHOD}'"), None);
    assert_eq!(value("'{tags}'"), None);
    assert_eq!(value("'{id|pascal}'"), None);
    assert_eq!(value("'v1 {path}'"), None);
}

#[test]
fn validation() {
    let err = |s: &str| format!("{:#}", Config::from_yaml(s).unwrap_err());
    let emit = |shape: &str| err(&format!("emit:\n- name: X\n  shape: {shape}"));
    assert!(err("operation:\n  name: '{type_name}'").contains("isn't available"));
    assert!(err("operation:\n  name: '{request}'").contains("isn't available"));
    assert!(err("operation:\n  name: '{nope}'").contains("unknown variable"));
    assert!(err("operation:\n  name: '{operationId}'").contains("it's `{id}` now"));
    assert!(err("jsdocs: true").contains("unknown field"));
    assert!(err("filter: { method: GTE }").contains("unknown method"));
    assert!(err("filter: { status: 200 }").contains("`status` isn't available"));
    assert!(err("filter: { has_body: true }").contains("unknown field"));
    assert!(err("schemas: { keep: { tag: x } }").contains("schemas.keep"));
    let grouped = |by: &str, name: &str| {
        err(&format!(
            "emit:\n- name: '{name}'\n  group_by: {by}\n  shape: '{{type_name}}'"
        ))
    };
    assert!(grouped("tags", "X").contains("same name"));
    assert!(grouped("request", "{Tags}").contains("isn't a variable to group on"));
    assert!(grouped("type_name", "{Type_name}").contains("isn't a variable to group on"));
    assert!(err("emit:\n- name: '{Tags}'\n  shape: '{type_name}'").contains("isn't available"));
    assert!(err("emit:\n- name: X\n  kind: map\n  shape: '{type_name}'").contains("unknown field"));
    assert!(emit("'{request}[\"body\"]'").contains("can only be the whole value"));
    assert!(emit("'{op.request}'").contains("it's `{request}` now"));
    assert!(emit("{ ref: '{type_name}', where: { status: 200 } }").contains("isn't part of one"));
    assert!(emit("{ ref: '{request}', pick: [status] }").contains("can't pick `status`"));
    assert!(emit("{ ref: '{request.body}', pick: [a] }").contains("`pick` takes keys of"));
    assert!(emit("{ ref: '{response}', where: { tag: x } }").contains("isn't available"));
    assert!(emit("{ ref: '{tags}' }").contains("isn't available"));
    assert!(emit("{ ref: '{type_name}', wher: {} }").contains("unknown field `wher`"));
    assert!(emit("{ a: 1 }").contains("found a number"));
    assert!(emit("{}").contains("empty map"));
    assert!(emit("{ '{request}': '{type_name}' }").contains("isn't available"));
    assert!(emit("{ '{path}': '{type}' }").contains("it's `{type_name}` now"));
}

#[test]
fn other_formats_are_rejected() {
    for ext in ["toml", "json5"] {
        let path = std::env::temp_dir().join(format!("opi-config-test.{ext}"));
        std::fs::write(&path, "").unwrap();
        let err = format!("{:#}", Config::from_file(&path).unwrap_err());
        assert!(err.contains("aren't supported"), "{err}");
    }
}

/// Exactly the `REF` variables stand for part of the operation's type.
#[test]
fn ref_vars_have_targets() {
    for var in Var::ALL {
        assert_eq!(
            RefTarget::from_var(var).is_some(),
            Var::REF.contains(&var),
            "{var}"
        );
    }
}
