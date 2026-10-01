//! Golden-file tests: `tests/fixtures/<name>.yaml` (+ optional
//! `<name>.<variant>.yaml` / `.json` config) -> `tests/fixtures/<name>[.<variant>].ts`,
//! and the IR / explain outputs. Run with `UPDATE_GOLDEN=1` to rewrite the
//! expected output.

use std::path::{Path, PathBuf};

use opi::{Config, Spec};

fn dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn spec(fixture: &str) -> Spec {
    Spec::from_file(&dir().join(format!("{fixture}.yaml"))).unwrap()
}

fn config(fixture: &str, variant: Option<&str>) -> Config {
    let Some(variant) = variant else {
        return Config::default();
    };
    ["yaml", "json"]
        .iter()
        .map(|ext| dir().join(format!("{fixture}.{variant}.{ext}")))
        .find(|path| path.exists())
        .map(|path| Config::from_file(&path).unwrap())
        .unwrap_or_else(|| panic!("no config for {fixture}.{variant}"))
}

fn golden(name: &str, actual: &str) {
    let golden = dir().join(name);
    if std::env::var_os("UPDATE_GOLDEN").is_some() {
        std::fs::write(&golden, actual).unwrap();
        return;
    }
    let expected = std::fs::read_to_string(&golden)
        .unwrap_or_else(|_| panic!("missing {}; run with UPDATE_GOLDEN=1", golden.display()));
    assert_eq!(actual, expected, "output differs from {}", golden.display());
}

fn check(fixture: &str, variant: Option<&str>) {
    let actual = opi::generate(&spec(fixture), &config(fixture, variant)).unwrap();
    let name = match variant {
        Some(v) => format!("{fixture}.{v}.ts"),
        None => format!("{fixture}.ts"),
    };
    golden(&name, &actual);
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

/// Only shop operations, so `Error` (used only by the component endpoints)
/// is dropped, while `keep` retains the unreferenced `Mixed`. The config is
/// JSON.
#[test]
fn components_schema_pruning() {
    check("components", Some("schemas"));
}

#[test]
fn openapi_32_features() {
    check("openapi32", None);
}

#[test]
fn components_ir() {
    let ir = opi::generate_ir(&spec("components"), &Config::default()).unwrap();
    golden("components.ir.json", &ir);
}

#[test]
fn components_explain() {
    let report = opi::explain(&spec("components"), &config("components", Some("emit"))).unwrap();
    golden("components.emit.explain.txt", &report);
}
