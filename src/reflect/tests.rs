//! The checked-in files generated from the config / IR types are current,
//! and the facet view of the config (which writes them) agrees with serde
//! (which reads configs). Run with `UPDATE_GOLDEN=1` to rewrite them.
use std::path::Path;

use super::*;
use crate::Config;

fn generated(path: &str, actual: &str) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(path);
    if std::env::var_os("UPDATE_GOLDEN").is_some() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, actual).unwrap();
        return;
    }
    let expected = std::fs::read_to_string(&path)
        .unwrap_or_else(|_| panic!("missing {}; run with UPDATE_GOLDEN=1", path.display()));
    assert_eq!(
        actual,
        expected,
        "{} is stale; run with UPDATE_GOLDEN=1",
        path.display()
    );
}

#[test]
fn default_configs() {
    generated("config.default.yaml", &example_config().unwrap());
    generated("config.default.json", &example_config_json().unwrap());
}

#[test]
fn json_schemas() {
    generated("config.schema.json", &config_schema().unwrap());
    generated("ir.schema.json", &ir_schema().unwrap());
}

#[test]
fn typescript_files() {
    generated("ts/src/generated/config.ts", &config_typescript());
    generated("ts/src/generated/ir.ts", &ir_typescript());
}

/// Writing a config out with facet and reading it back with serde is
/// lossless, for the defaults and for every fixture config.
#[test]
fn yaml_round_trips() {
    let fixtures = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures");
    let mut configs = vec![("default".to_string(), Config::default())];
    for entry in std::fs::read_dir(fixtures).unwrap() {
        let path = entry.unwrap().path();
        let name = path.file_name().unwrap().to_string_lossy();
        // `<fixture>.<variant>.yaml` / `.json`; `<fixture>.yaml` is a spec and
        // `<fixture>.ir.json` an IR golden.
        let config = name.matches('.').count() == 2
            && (name.ends_with(".yaml") || name.ends_with(".json"))
            && !name.ends_with(".ir.json");
        if config {
            configs.push((
                path.display().to_string(),
                Config::from_file(&path).unwrap(),
            ));
        }
    }
    assert!(configs.len() > 3, "found the fixture configs");
    for (name, config) in configs {
        let yaml = to_yaml(&config).unwrap();
        let reparsed = Config::from_yaml(&yaml)
            .unwrap_or_else(|e| panic!("{name}: {e:#}\n--- generated ---\n{yaml}"));
        assert_eq!(reparsed, config, "{name} changed on round trip:\n{yaml}");
    }
}
