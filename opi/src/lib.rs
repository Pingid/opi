pub mod ast;
pub mod code;
pub mod oapi;
pub mod rule;
pub mod util;

mod config;
pub use config::Config;
mod cli;
pub use cli::Cli;

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::util::JsonOrYaml;

    use super::oapi::Oapi;
    use super::rule::{Builder, Rule};

    const SPEC: &str = r##"{
      "openapi": "3.1.0",
      "paths": {
        "/users/{id}": {
          "get": {
            "responses": {
              "200": { "content": { "application/json": { "schema": { "$ref": "#/components/schemas/User" } } } },
              "404": { "content": { "application/json": { "schema": { "$ref": "#/components/schemas/Error" } } } }
            }
          },
          "put": {
            "deprecated": true,
            "requestBody": { "content": { "application/json": { "schema": { "$ref": "#/components/schemas/User" } } } },
            "responses": { "204": { "description": "ok" } }
          }
        }
      },
      "components": {
        "schemas": {
          "User": {
            "type": "object",
            "description": "A user.",
            "required": ["id"],
            "properties": {
              "id": { "type": "string" },
              "address": { "$ref": "#/components/schemas/Address" }
            }
          },
          "Address": { "type": "object", "properties": { "city": { "type": "string" } } },
          "Error": { "type": "object", "properties": { "message": { "type": "string" } } }
        }
      }
    }"##;

    fn rule(value: serde_json::Value) -> Rule {
        Rule::try_from(value).unwrap()
    }

    #[test]
    fn emits_the_grammar_examples() -> anyhow::Result<()> {
        let ir = JsonOrYaml::<Oapi>::deserialize(SPEC)?.lower()?;
        let mut builder = Builder::new(&ir);

        // `Address` is deliberately not emitted: refs to it get inlined.
        builder
            .with(rule(json!({
                "each": "schema",
                "where": { "deprecated": false },
                "emit": { "type": "{name}" }
            })))?
            .with(rule(json!({
                "each": "route",
                "emit": {
                    "type": "{method}{path}",
                    "fields": { "method": "{method}", "request": "{request}", "response": "{response}" }
                }
            })))?
            .with(rule(json!({
                "each": "route",
                "where": { "deprecated": false },
                "emit": {
                    "type": "RoutesMap",
                    "fields": { "{path}": { "{method}": {
                        "request": "{method}{path}['request']",
                        "response": "{method}{path}['response']"
                    } } }
                }
            })))?
            .with(rule(json!({
                "each": { "route": "response" },
                "emit": {
                    "type": "ContentTypes",
                    "fields": { "{content_type}": { "status": "{status}", "type": "{schema}" } }
                }
            })))?;

        let ts = builder.render()?;

        assert!(ts.contains("export type User ="));
        assert!(ts.contains("export type GetUsersById ="));
        assert!(ts.contains("export type PutUsersById ="));
        assert!(ts.contains("GetUsersById[\"request\"]"));
        // Deprecated `put` is filtered out of the map.
        assert!(!ts.contains("PutUsersById[\"request\"]"));
        // Two responses share a content type: their fields are unioned.
        assert!(ts.contains("200 | 404"));
        Ok(())
    }

    /// `{ a?: string; [key: string]: string }` doesn't compile; the index
    /// type must cover the named properties.
    #[test]
    fn index_signatures_cover_named_properties() -> anyhow::Result<()> {
        let spec = r#"{
          "openapi": "3.1.0",
          "components": { "schemas": { "Perms": {
            "type": "object",
            "properties": { "a": { "type": "string" }, "b": { "type": "integer" } },
            "required": ["b"],
            "additionalProperties": { "type": "string" }
          } } }
        }"#;
        let ir = JsonOrYaml::<Oapi>::deserialize(spec)?.lower()?;
        let mut builder = Builder::new(&ir);
        builder.with(rule(
            json!({ "each": "schema", "emit": { "type": "{name}" } }),
        ))?;
        let ts = builder.render()?;
        assert!(
            ts.contains("[key: string]: string | number | undefined;"),
            "{ts}"
        );
        Ok(())
    }

    const NAMES: &str = r##"{
      "openapi": "3.1.0",
      "paths": {
        "/blobs": {
          "post": {
            "summary": "Upload a blob",
            "description": "Stores it.",
            "requestBody": { "content": { "application/octet-stream": { "schema": { "type": "string", "format": "binary" } } } },
            "responses": { "201": { "content": { "application/json": { "schema": { "$ref": "#/components/schemas/Blob" } } } } }
          },
          "get": {
            "deprecated": true,
            "responses": { "500": { "content": { "application/json": { "schema": { "$ref": "#/components/schemas/Error" } } } } }
          }
        }
      },
      "components": {
        "schemas": {
          "Blob": { "type": "object", "properties": { "sha": { "type": "string" } } },
          "Error": { "type": "object", "properties": { "message": { "type": "string" } } }
        }
      }
    }"##;

    fn names_output(formats: &[(&str, &str)], extra: Option<Rule>) -> anyhow::Result<String> {
        let ir = JsonOrYaml::<Oapi>::deserialize(NAMES)?.lower()?;
        let mut builder = Builder::new(&ir);
        builder.formats(
            formats
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        );
        for rule in crate::Config::default().rules.into_iter().chain(extra) {
            builder.with(rule)?;
        }
        builder.render()
    }

    /// `Error` and `Blob` would shadow globals (`Blob` is also what
    /// `format: binary` emits), so they're suffixed and refs follow.
    #[test]
    fn declarations_shadowing_globals_are_renamed() -> anyhow::Result<()> {
        let extra = rule(json!({
            "each": "schema",
            "where": { "deprecated": false },
            "emit": { "type": "Messages", "fields": { "{name}": "{name}['sha']" } }
        }));
        let ts = names_output(&[("binary", "Blob")], None)?;
        assert!(ts.contains("export type Blob2 ="), "{ts}");
        assert!(ts.contains("export type Error2 ="), "{ts}");
        assert!(!ts.contains("export type Blob ="), "{ts}");
        // A spec ref, and `format: binary`, which stays the global.
        assert!(ts.contains("body: Blob2;"), "{ts}");
        assert!(ts.contains("body: Blob;"), "{ts}");
        assert!(ts.contains("body: Error2;"), "{ts}");

        // A template ref by the declaration's name follows the rename.
        let ts = names_output(&[], Some(extra))?;
        assert!(ts.contains("Blob: Blob2[\"sha\"]"), "{ts}");
        Ok(())
    }

    /// Without `formats`, a binary string is a plain `string`, and a
    /// `formats` type is reserved like a global.
    #[test]
    fn formats_replace_base_types() -> anyhow::Result<()> {
        let ts = names_output(&[], None)?;
        assert!(ts.contains("body: string;"), "{ts}");
        let ts = names_output(&[("binary", "Error")], None)?;
        assert!(ts.contains("body: Error;"), "{ts}");
        assert!(ts.contains("export type Error2 ="), "{ts}");
        Ok(())
    }

    /// The route's declaration gets `@summary` / `@description` /
    /// `@deprecated`; its `Routes` key the summary only.
    #[test]
    fn routes_are_documented() -> anyhow::Result<()> {
        let ts = names_output(&[], None)?;
        assert!(
            // oxc re-indents block comment lines, so no space before `*`.
            ts.contains("/**\n* @summary Upload a blob\n* @description Stores it.\n*/\nexport type PostBlobs ="),
            "{ts}"
        );
        assert!(
            ts.contains("/** @deprecated */\nexport type GetBlobs ="),
            "{ts}"
        );
        assert!(
            ts.contains("\t/** @summary Upload a blob */\n\t\"POST /blobs\": {"),
            "{ts}"
        );
        // A shared declaration isn't documented with one route's summary.
        assert!(ts.contains("\nexport type Routes ="), "{ts}");
        Ok(())
    }

    #[test]
    fn unknown_references_are_errors() -> anyhow::Result<()> {
        let ir = JsonOrYaml::<Oapi>::deserialize(SPEC)?.lower()?;
        let mut builder = Builder::new(&ir);
        builder.with(rule(json!({
            "each": "route",
            "emit": { "type": "Routes", "fields": { "{method}{path}": "Nope['x']" } }
        })))?;
        assert!(builder.render().is_err());
        Ok(())
    }
}
