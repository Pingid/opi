//! Spec-parsing benchmark: serde (openapiv3, previous) vs serde vs facet on
//! the same minimal "loose" OpenAPI model.
//!
//! cargo run --release --example parse_bench -- [spec.json] [iterations]

#![allow(dead_code)] // fields exist to be parsed, not read

use std::time::{Duration, Instant};

use indexmap::IndexMap;

/// The subset of OpenAPI lowering actually reads. Unknown fields are ignored
/// by both deserializers, so they do comparable work.
mod model {
    use super::*;

    #[derive(serde::Deserialize, facet::Facet)]
    pub struct OpenApi {
        pub paths: IndexMap<String, PathItem>,
        pub components: Option<Components>,
    }

    #[derive(serde::Deserialize, facet::Facet)]
    pub struct Components {
        #[serde(default)]
        #[facet(default)]
        pub schemas: IndexMap<String, Schema>,
        #[serde(default)]
        #[facet(default)]
        pub parameters: IndexMap<String, Parameter>,
        #[serde(default, rename = "requestBodies")]
        #[facet(default, rename = "requestBodies")]
        pub request_bodies: IndexMap<String, RequestBody>,
        #[serde(default)]
        #[facet(default)]
        pub responses: IndexMap<String, Response>,
    }

    #[derive(serde::Deserialize, facet::Facet)]
    pub struct PathItem {
        pub get: Option<Operation>,
        pub put: Option<Operation>,
        pub post: Option<Operation>,
        pub delete: Option<Operation>,
        pub patch: Option<Operation>,
        #[serde(default)]
        #[facet(default)]
        pub parameters: Vec<Parameter>,
    }

    #[derive(serde::Deserialize, facet::Facet)]
    pub struct Operation {
        #[serde(rename = "operationId")]
        #[facet(rename = "operationId")]
        pub operation_id: Option<String>,
        pub summary: Option<String>,
        pub description: Option<String>,
        #[serde(default)]
        #[facet(default)]
        pub deprecated: bool,
        #[serde(default)]
        #[facet(default)]
        pub tags: Vec<String>,
        #[serde(default)]
        #[facet(default)]
        pub parameters: Vec<Parameter>,
        #[serde(rename = "requestBody")]
        #[facet(rename = "requestBody")]
        pub request_body: Option<RequestBody>,
        #[serde(default)]
        #[facet(default)]
        pub responses: IndexMap<String, Response>,
    }

    #[derive(serde::Deserialize, facet::Facet)]
    pub struct Parameter {
        #[serde(rename = "$ref")]
        #[facet(rename = "$ref")]
        pub reference: Option<String>,
        pub name: Option<String>,
        #[serde(rename = "in")]
        #[facet(rename = "in")]
        pub location: Option<String>,
        #[serde(default)]
        #[facet(default)]
        pub required: bool,
        pub description: Option<String>,
        pub schema: Option<Schema>,
    }

    #[derive(serde::Deserialize, facet::Facet)]
    pub struct RequestBody {
        #[serde(rename = "$ref")]
        #[facet(rename = "$ref")]
        pub reference: Option<String>,
        #[serde(default)]
        #[facet(default)]
        pub required: bool,
        pub description: Option<String>,
        #[serde(default)]
        #[facet(default)]
        pub content: IndexMap<String, MediaType>,
    }

    #[derive(serde::Deserialize, facet::Facet)]
    pub struct Response {
        #[serde(rename = "$ref")]
        #[facet(rename = "$ref")]
        pub reference: Option<String>,
        pub description: Option<String>,
        #[serde(default)]
        #[facet(default)]
        pub content: IndexMap<String, MediaType>,
    }

    #[derive(serde::Deserialize, facet::Facet)]
    pub struct MediaType {
        pub schema: Option<Schema>,
    }

    /// One struct for every schema shape, `$ref` included.
    #[derive(serde::Deserialize, facet::Facet)]
    pub struct Schema {
        #[serde(rename = "$ref")]
        #[facet(rename = "$ref")]
        pub reference: Option<String>,
        #[serde(rename = "type")]
        #[facet(rename = "type")]
        pub ty: Option<String>,
        pub format: Option<String>,
        pub title: Option<String>,
        pub description: Option<String>,
        #[serde(default)]
        #[facet(default)]
        pub nullable: bool,
        #[serde(default)]
        #[facet(default)]
        pub deprecated: bool,
        #[serde(default, rename = "readOnly")]
        #[facet(default, rename = "readOnly")]
        pub read_only: bool,
        #[serde(default)]
        #[facet(default)]
        pub properties: IndexMap<String, Schema>,
        #[serde(default)]
        #[facet(default)]
        pub required: Vec<String>,
        #[serde(rename = "additionalProperties")]
        #[facet(rename = "additionalProperties")]
        pub additional_properties: Option<AdditionalProperties>,
        pub items: Option<Box<Schema>>,
        #[serde(default, rename = "allOf")]
        #[facet(default, rename = "allOf")]
        pub all_of: Vec<Schema>,
        #[serde(default, rename = "oneOf")]
        #[facet(default, rename = "oneOf")]
        pub one_of: Vec<Schema>,
        #[serde(default, rename = "anyOf")]
        #[facet(default, rename = "anyOf")]
        pub any_of: Vec<Schema>,
    }

    #[derive(serde::Deserialize, facet::Facet)]
    #[serde(untagged)]
    #[facet(untagged)]
    #[repr(u8)]
    pub enum AdditionalProperties {
        Bool(bool),
        Schema(Box<Schema>),
    }
}

fn bench<T>(name: &str, iterations: usize, mut f: impl FnMut() -> T) -> Duration {
    // Warm up (and fail fast on parse errors).
    std::hint::black_box(f());
    let mut times: Vec<_> = (0..iterations)
        .map(|_| {
            let start = Instant::now();
            std::hint::black_box(f());
            start.elapsed()
        })
        .collect();
    times.sort();
    let median = times[times.len() / 2];
    println!(
        "{name:<34} min {:>8.1?}  median {:>8.1?}  max {:>8.1?}",
        times[0],
        median,
        times[times.len() - 1]
    );
    median
}

fn main() {
    let mut args = std::env::args().skip(1);
    let path = args
        .next()
        .unwrap_or_else(|| "schemas/api.github.com.json".into());
    let iterations = args.next().map_or(10, |n| n.parse().unwrap());
    let source = std::fs::read_to_string(&path).unwrap();
    println!(
        "{path}: {:.1} MB, {iterations} iterations\n",
        source.len() as f64 / 1e6
    );

    // Sanity check: both loose parsers see the same spec.
    let a: model::OpenApi = serde_json::from_str(&source).unwrap();
    let b: model::OpenApi = facet_json::from_str(&source).unwrap();
    assert_eq!(a.paths.len(), b.paths.len());
    assert_eq!(
        a.components.map(|c| c.schemas.len()),
        b.components.map(|c| c.schemas.len())
    );

    bench("serde_json -> serde_json::Value", iterations, || {
        serde_json::from_str::<serde_json::Value>(&source).unwrap()
    });
    bench("serde_json -> openapiv3 (previous)", iterations, || {
        serde_json::from_str::<openapiv3::OpenAPI>(&source).unwrap()
    });
    bench(
        "serde_json -> openapi::Document (current)",
        iterations,
        || serde_json::from_str::<opi::openapi::Document>(&source).unwrap(),
    );
    let serde = bench("serde_json -> loose model", iterations, || {
        serde_json::from_str::<model::OpenApi>(&source).unwrap()
    });
    let facet = bench("facet_json -> loose model", iterations, || {
        facet_json::from_str::<model::OpenApi>(&source).unwrap()
    });
    println!(
        "\nfacet / serde on the same model: {:.2}x",
        facet.as_secs_f64() / serde.as_secs_f64()
    );
}
