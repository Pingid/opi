use std::{
    marker::PhantomData,
    path::{Path, PathBuf},
};

use anyhow::{Context, bail};
use serde::de::DeserializeOwned;

/// How long a fetch may take, connecting through to the last byte.
const FETCH_TIMEOUT_SECS: u64 = 60;

pub struct JsonOrYamlFile<T> {
    pub path: PathBuf,
    _marker: PhantomData<T>,
}

impl<T: DeserializeOwned> JsonOrYamlFile<T> {
    /// A local `.json` / `.yaml` / `.yml` file, or an `http(s)://` URL.
    pub fn from_file(path: &Path) -> anyhow::Result<T> {
        if let Some(url) = url(path) {
            return Self::from_url(url);
        }
        let content = Self::read_file(path)?;
        match path.extension() {
            Some(ext) if ext == "json" => Self::from_json(&content),
            Some(ext) if ext == "yaml" || ext == "yml" => Self::from_yaml(&content),
            _ => anyhow::bail!("File must be a JSON or YAML file"),
        }
    }

    /// JSON or YAML by the URL's extension, else its `Content-Type`, else
    /// its content.
    pub fn from_url(url: &str) -> anyhow::Result<T> {
        let (content, content_type) = fetch(url)?;
        let format = Format::from_extension(url)
            .or_else(|| content_type.as_deref().and_then(Format::from_content_type))
            .unwrap_or_else(|| Format::sniff(&content));
        let parsed = match format {
            Format::Json => Self::from_json(&content),
            Format::Yaml => Self::from_yaml(&content),
        };
        parsed.with_context(|| format!("in {url}"))
    }

    pub fn from_json(string: &str) -> anyhow::Result<T> {
        serde_json::from_str(string).context("Failed to parse file as JSON")
    }

    pub fn from_yaml(string: &str) -> anyhow::Result<T> {
        serde_yaml::from_str(string).context("Failed to parse file as YAML")
    }

    fn read_file(path: &Path) -> anyhow::Result<String> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read file: {}", path.display()))?;
        Ok(content)
    }
}

/// `path` as a URL, if it's an `http://` or `https://` one.
pub fn url(path: &Path) -> Option<&str> {
    let s = path.to_str()?;
    let scheme = s.split_once("://")?.0;
    ["http", "https"]
        .iter()
        .any(|known| scheme.eq_ignore_ascii_case(known))
        .then_some(s)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Format {
    Json,
    Yaml,
}

impl Format {
    /// From the last path segment, ignoring any query or fragment.
    fn from_extension(url: &str) -> Option<Self> {
        let path = url.split(['?', '#']).next()?;
        let segment = path.rsplit('/').next()?;
        match Path::new(segment).extension()?.to_str()? {
            "json" => Some(Format::Json),
            "yaml" | "yml" => Some(Format::Yaml),
            _ => None,
        }
    }

    /// `application/json`, `application/vnd.oai.openapi+json`, `text/yaml`,
    /// `application/x-yaml`, ...; not `text/plain`.
    fn from_content_type(content_type: &str) -> Option<Self> {
        let essence = content_type.split(';').next()?.trim().to_ascii_lowercase();
        if essence.ends_with("json") {
            Some(Format::Json)
        } else if essence.ends_with("yaml") || essence.ends_with("yml") {
            Some(Format::Yaml)
        } else {
            None
        }
    }

    /// A JSON document is an object or array; anything else is YAML.
    fn sniff(content: &str) -> Self {
        match content.trim_start().starts_with(['{', '[']) {
            true => Format::Json,
            false => Format::Yaml,
        }
    }
}

/// The body of a 2xx response to a GET, and its `Content-Type`.
/// Redirects are followed.
fn fetch(url: &str) -> anyhow::Result<(String, Option<String>)> {
    let response = minreq::get(url)
        .with_header("User-Agent", concat!("opi/", env!("CARGO_PKG_VERSION")))
        .with_header(
            "Accept",
            "application/json, application/yaml, text/yaml;q=0.9, */*;q=0.8",
        )
        .with_timeout(FETCH_TIMEOUT_SECS)
        .send()
        .with_context(|| format!("Failed to fetch {url}"))?;
    if !(200..300).contains(&response.status_code) {
        bail!(
            "Failed to fetch {url}: {} {}",
            response.status_code,
            response.reason_phrase
        );
    }
    let content_type = response
        .headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("content-type"))
        .map(|(_, value)| value.clone());
    let content = response
        .as_str()
        .with_context(|| format!("{url} isn't UTF-8 text"))?
        .to_string();
    Ok((content, content_type))
}

pub struct JsonOrYaml<T> {
    _marker: PhantomData<T>,
}

impl<T: DeserializeOwned> JsonOrYaml<T> {
    pub fn deserialize(string: &str) -> anyhow::Result<T> {
        match serde_json::from_str(string) {
            Ok(value) => Ok(value),
            Err(_) => match serde_yaml::from_str(string) {
                Ok(value) => Ok(value),
                Err(_) => anyhow::bail!("Failed to deserialize as JSON or YAML"),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::{BufRead, BufReader, Write};
    use std::net::TcpListener;

    use serde_json::{Value, json};

    use super::*;

    /// Serves `routes` (path, status, content type, body) on a local port
    /// until the test process exits; returns its base URL.
    fn serve(routes: &'static [(&str, u16, Option<&str>, &str)]) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
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
                let (_, status, content_type, body) = routes
                    .iter()
                    .find(|(path, ..)| *path == target)
                    .copied()
                    .unwrap_or(("", 404, None, ""));
                let content_type = content_type
                    .map(|ct| format!("Content-Type: {ct}\r\n"))
                    .unwrap_or_default();
                let response = format!(
                    "HTTP/1.1 {status} Status\r\n{content_type}Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                stream.write_all(response.as_bytes()).unwrap();
            }
        });
        base
    }

    fn get(url: &str) -> anyhow::Result<Value> {
        JsonOrYamlFile::<Value>::from_file(Path::new(url))
    }

    #[test]
    fn urls() {
        assert_eq!(url(Path::new("https://a/b.json")), Some("https://a/b.json"));
        assert_eq!(url(Path::new("HTTP://a")), Some("HTTP://a"));
        assert_eq!(url(Path::new("./https.yaml")), None);
        assert_eq!(url(Path::new("ftp://a/b.json")), None);
        assert_eq!(url(Path::new("C:/specs/a.json")), None);
    }

    #[test]
    fn format_detection() {
        assert_eq!(
            Format::from_extension("https://a/b.yml?ref=main#x"),
            Some(Format::Yaml)
        );
        assert_eq!(Format::from_extension("https://a/v3/api-docs"), None);
        assert_eq!(Format::from_extension("https://a.json/docs"), None);
        let ct = Format::from_content_type;
        assert_eq!(
            ct("application/vnd.oai.openapi+json;version=3.1"),
            Some(Format::Json)
        );
        assert_eq!(ct("application/x-yaml; charset=utf-8"), Some(Format::Yaml));
        assert_eq!(ct("text/plain"), None);
        assert_eq!(Format::sniff("\n  {\"a\": 1}"), Format::Json);
        assert_eq!(Format::sniff("a: 1"), Format::Yaml);
    }

    #[test]
    fn fetches_json_and_yaml() -> anyhow::Result<()> {
        static ROUTES: &[(&str, u16, Option<&str>, &str)] = &[
            // Raw file hosts serve everything as text/plain: the extension wins.
            ("/spec.yaml?ref=main", 200, Some("text/plain"), "a: 1"),
            ("/v3/api-docs", 200, Some("application/json"), "{\"a\": 1}"),
            ("/docs.yaml", 200, Some("application/yaml"), "a: 1"),
            ("/plain", 200, None, "a: 1"),
            ("/plain-json", 200, Some("text/plain"), " {\"a\": 1}"),
            ("/broken.json", 200, None, "a: 1"),
        ];
        let base = serve(ROUTES);
        for path in [
            "/spec.yaml?ref=main",
            "/v3/api-docs",
            "/docs.yaml",
            "/plain",
            "/plain-json",
        ] {
            assert_eq!(get(&format!("{base}{path}"))?, json!({ "a": 1 }), "{path}");
        }
        // The real parse error, not a generic one.
        let err = format!("{:#}", get(&format!("{base}/broken.json")).unwrap_err());
        assert!(
            err.contains("as JSON") && err.contains("/broken.json"),
            "{err}"
        );
        Ok(())
    }

    #[test]
    fn http_errors_name_the_status() {
        static ROUTES: &[(&str, u16, Option<&str>, &str)] = &[];
        let base = serve(ROUTES);
        let err = format!("{:#}", get(&format!("{base}/missing.json")).unwrap_err());
        assert!(err.contains("/missing.json: 404"), "{err}");

        // Nothing listening.
        let port = TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port();
        let err = format!(
            "{:#}",
            get(&format!("http://127.0.0.1:{port}/a.json")).unwrap_err()
        );
        assert!(err.contains("Failed to fetch"), "{err}");
    }
}
