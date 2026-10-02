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
    assert_eq!(config.rules.len(), 3);
    assert!(config.files.is_empty());
}

// --- config files ----------------------------------------------------------

/// The checked-in default config: rules to stdout, no files, no source.
#[test]
fn default_config_file_round_trips() {
    let config = Config::from_file(&root().join("config.default.yaml")).unwrap();
    assert_eq!(config.source, None);
    assert_eq!(config.rules.len(), 3);
    assert!(config.files.is_empty(), "the default writes to stdout");
    assert_eq!(config.formats, Config::default().formats);
}

/// Without `-c`, the default rules go to stdout (or the `output` argument),
/// and nothing else is written.
#[test]
fn default_run_prints_to_stdout() {
    let dir = std::env::temp_dir().join(format!("opi-stdout-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let spec = fixtures().join("components.yaml");
    let expected = std::fs::read_to_string(fixtures().join("components.ts")).unwrap();

    let out = opi().arg(&spec).current_dir(&dir).output().unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&out.stdout), expected);
    assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 0, "wrote a file");

    let out = opi()
        .arg(&spec)
        .arg("out.ts")
        .current_dir(&dir)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(out.stdout.is_empty());
    assert_eq!(
        std::fs::read_to_string(dir.join("out.ts")).unwrap(),
        expected
    );
    std::fs::remove_dir_all(&dir).unwrap();
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

// --- remote specs ----------------------------------------------------------

/// Serves each `(path, content type, body)` on a local port until the test
/// process exits; returns its base URL.
fn serve(routes: Vec<(&'static str, &'static str, String)>) -> String {
    use std::io::{BufRead, BufReader, Write};

    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let mut stream = stream.unwrap();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut request = String::new();
            reader.read_line(&mut request).unwrap();
            let mut header = String::new();
            while reader.read_line(&mut header).unwrap() > 2 {
                header.clear();
            }
            let target = request.split(' ').nth(1).unwrap_or("/");
            let response = match routes.iter().find(|(path, ..)| *path == target) {
                Some((_, content_type, body)) => format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                ),
                None => "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                    .to_string(),
            };
            stream.write_all(response.as_bytes()).unwrap();
        }
    });
    base
}

const RULES: &str = r#"rules:
  - each: schema
    emit: { type: "{name}" }
  - each: route
    emit: { type: "{method}{path}" }
"#;

fn stdout(command: &mut Command) -> String {
    let out = command.output().unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap()
}

/// A URL as the `openapi` argument or as `source` reads the same as the
/// file. No extension here, so the `Content-Type` says it's YAML.
#[test]
fn specs_can_be_urls() {
    let spec = std::fs::read_to_string(fixtures().join("components.yaml")).unwrap();
    let base = serve(vec![("/specs/components", "application/yaml", spec)]);
    let url = format!("{base}/specs/components");

    let dir = std::env::temp_dir().join(format!("opi-remote-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let rules = dir.join("rules.yaml");
    std::fs::write(&rules, RULES).unwrap();
    let source = dir.join("source.yaml");
    std::fs::write(&source, format!("source: {url}\n{RULES}")).unwrap();

    let local = stdout(
        opi()
            .arg(fixtures().join("components.yaml"))
            .arg("-c")
            .arg(&rules),
    );
    assert!(local.contains("export type GetComponentsByName"), "{local}");
    assert_eq!(stdout(opi().arg(&url).arg("-c").arg(&rules)), local);
    assert_eq!(stdout(opi().arg("-c").arg(&source)), local);

    let out = opi()
        .arg(format!("{base}/nope.json"))
        .arg("-c")
        .arg(&rules)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("/nope.json: 404 Not Found"), "{stderr}");

    std::fs::remove_dir_all(&dir).unwrap();
}
