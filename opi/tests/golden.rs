//! Golden-file tests. `tests/fixtures/<spec>.yaml` is an OpenAPI spec and
//! `<spec>.<variant>.yaml` a config; the expected output is
//! `<spec>[.<variant>].ts` beside them. The repo-root `config.schema.json`
//! is a golden too. Run with `UPDATE_GOLDEN=1` to rewrite them.

use std::path::{Path, PathBuf};
use std::process::Command;

use opi::Config;
use opi::oapi::Oapi;
use opi::rule::Builder;

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn update() -> bool {
    std::env::var_os("UPDATE_GOLDEN").is_some()
}

fn golden(path: &Path, actual: &str) {
    if update() {
        std::fs::write(path, actual).unwrap();
        return;
    }
    let expected = std::fs::read_to_string(path)
        .unwrap_or_else(|_| panic!("missing {}; run with UPDATE_GOLDEN=1", path.display()));
    assert_eq!(actual, expected, "output differs from {}", path.display());
}

fn generate(spec: &str, config: Config) -> String {
    let ir = Oapi::from_file(&fixtures().join(format!("{spec}.yaml")))
        .unwrap()
        .lower()
        .unwrap();
    let mut builder = Builder::new(&ir);
    builder.formats(config.formats);
    for rule in config.rules {
        builder.with(rule).unwrap();
    }
    builder.render().unwrap()
}

/// `None`: the built-in rules. `Some(v)`: the `rules` of `<spec>.<v>.yaml`.
fn check(spec: &str, variant: Option<&str>) {
    let (config, name) = match variant {
        None => (Config::default(), format!("{spec}.ts")),
        Some(v) => {
            let config = Config::from_file(&fixtures().join(format!("{spec}.{v}.yaml"))).unwrap();
            (config, format!("{spec}.{v}.ts"))
        }
    };
    golden(&fixtures().join(name), &generate(spec, config));
}

#[test]
fn components_default() {
    check("components", None);
}

#[test]
fn openapi32_default() {
    check("openapi32", None);
}

#[test]
fn components_emit() {
    check("components", Some("emit"));
}

// --- the binary ----------------------------------------------------------

fn opi() -> Command {
    Command::new(env!("CARGO_BIN_EXE_opi"))
}

/// Through the CLI, each `files` entry is written relative to the config.
#[test]
fn components_files() {
    let dir = std::env::temp_dir().join(format!("opi-golden-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let config = dir.join("components.files.yaml");
    std::fs::copy(fixtures().join("components.files.yaml"), &config).unwrap();

    let out = opi()
        .arg(fixtures().join("components.yaml"))
        .arg("-c")
        .arg(&config)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(out.stdout.is_empty(), "nothing on stdout without `rules`");

    for file in ["schemas", "routes"] {
        let actual = std::fs::read_to_string(dir.join(format!("{file}.ts"))).unwrap();
        golden(
            &fixtures().join(format!("components.files.{file}.ts")),
            &actual,
        );
    }
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn completions_do_not_need_a_spec() {
    let out = opi().args(["--completions", "zsh"]).output().unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(String::from_utf8_lossy(&out.stdout).contains("opi"));
}

#[test]
fn missing_spec_is_an_error_not_a_panic() {
    let out = opi().arg("nope.yaml").output().unwrap();
    assert_eq!(out.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("Failed to read file: nope.yaml"),
        "{stderr}"
    );
    assert!(!stderr.contains("panicked"), "{stderr}");
}

#[test]
fn init_prints_the_default_config() {
    let out = opi().arg("--init").output().unwrap();
    assert!(out.status.success());
    let expected = std::fs::read_to_string(root().join("config.default.yaml")).unwrap();
    assert_eq!(String::from_utf8_lossy(&out.stdout), expected);

    let out = opi().args(["--init", "json"]).output().unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let config: Config = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(config.files.len(), 1);
}

// --- config files ----------------------------------------------------------

#[test]
fn default_config_file_round_trips() {
    let config = Config::from_file(&root().join("config.default.yaml")).unwrap();
    assert_eq!(
        config.source.as_deref(),
        Some(Path::new("./schemas/openapi.yaml"))
    );
    assert!(
        config.rules.is_empty(),
        "no `rules` key means none, not the defaults"
    );
    assert_eq!(config.formats, Config::default().formats);
    let (path, rules) = config.files.iter().next().unwrap();
    assert_eq!(path, Path::new("./my.openapi.ts"));
    assert_eq!(rules.len(), 3);
}

/// Compared as JSON values so reformatting the file doesn't fail this.
#[test]
fn config_schema_is_current() {
    let actual = serde_json::to_value(Config::json_schema()).unwrap();
    let path = root().join("config.schema.json");
    if update() {
        let pretty = serde_json::to_string_pretty(&actual).unwrap();
        std::fs::write(&path, pretty + "\n").unwrap();
        return;
    }
    let expected: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(
        actual, expected,
        "config.schema.json is stale; run with UPDATE_GOLDEN=1"
    );
}
