//! Golden-file tests: `tests/fixtures/<name>.yaml` (+ optional
//! `<name>.<variant>.toml` config) -> `tests/fixtures/<name>[.<variant>].ts`.
//! Run with `UPDATE_GOLDEN=1` to rewrite the expected output.

use std::path::Path;

use opi::{Config, openapi};

fn check(fixture: &str, variant: Option<&str>) {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let spec = openapi::Spec::from_file(&dir.join(format!("{fixture}.yaml"))).unwrap();
    let (config, golden) = match variant {
        Some(v) => (
            Config::from_file(&dir.join(format!("{fixture}.{v}.toml"))).unwrap(),
            dir.join(format!("{fixture}.{v}.ts")),
        ),
        None => (Config::default(), dir.join(format!("{fixture}.ts"))),
    };
    let actual = opi::generate(&spec, &config).unwrap();

    if std::env::var_os("UPDATE_GOLDEN").is_some() {
        std::fs::write(&golden, &actual).unwrap();
        return;
    }
    let expected = std::fs::read_to_string(&golden)
        .unwrap_or_else(|_| panic!("missing {}; run with UPDATE_GOLDEN=1", golden.display()));
    assert_eq!(actual, expected, "output differs from {}", golden.display());
}

#[test]
fn components_default() {
    check("components", None);
}

#[test]
fn components_alternate_layouts() {
    check("components", Some("alt"));
}

#[test]
fn components_emits() {
    check("components", Some("emit"));
}

#[test]
fn components_schema_pruning() {
    check("components", Some("schemas"));
}

#[test]
fn openapi_32_features() {
    check("openapi32", None);
}
