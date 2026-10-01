//! The build-only TOML formatter (`build/fmt.rs`) against real configs.

#[path = "../build/fmt.rs"]
#[allow(dead_code)]
mod fmt;

use opi::Config;

/// Writing a config out and reading it back is lossless, for the
/// defaults and for every fixture config.
#[test]
fn to_toml_round_trips() {
    let fixtures = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures");
    let mut configs = vec![("default".to_string(), Config::default())];
    for entry in std::fs::read_dir(fixtures).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|e| e == "toml") {
            configs.push((
                path.display().to_string(),
                Config::from_file(&path).unwrap(),
            ));
        }
    }
    for (name, config) in configs {
        let toml = fmt::to_string(&config, &fmt::SerializeOptions::default()).unwrap();
        let reparsed = Config::from_toml(&toml)
            .unwrap_or_else(|e| panic!("{name}: {e:#}\n--- generated ---\n{toml}"));
        assert_eq!(reparsed, config, "{name} changed on round trip:\n{toml}");
    }
}
