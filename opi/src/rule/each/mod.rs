use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ast;
use crate::rule::{ItemSets, Stage, ViewRule};

/// Handles either a simple string (`"route"`) or a nested map (`{ route: "response" }`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
#[serde(untagged)]
pub enum Each {
    Object(EachObject),
    Bare(EachBare),
}

impl Each {
    pub const DEFAULT: Each = Each::Bare(EachBare::Route);
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EachObject {
    Route(Option<EachRoute>),
    Schema,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EachBare {
    Route,
    Schema,
}

/// Which part of a route to iterate: its request bodies or its responses.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EachRoute {
    Request,
    Response,
}

impl<'a> ViewRule<'a> for Each {
    fn stage(&mut self, stage: &mut ItemSets<'a>, ir: &'a ast::Doc) {
        stage.add(match self {
            Each::Object(EachObject::Route(Some(EachRoute::Request))) => {
                Stage::Requests(ir.requests().collect())
            }
            Each::Object(EachObject::Route(Some(EachRoute::Response))) => {
                Stage::Responses(ir.responses().collect())
            }
            Each::Object(EachObject::Route(None)) | Each::Bare(EachBare::Route) => {
                Stage::Routes(ir.routes().collect())
            }
            Each::Object(EachObject::Schema) | Each::Bare(EachBare::Schema) => {
                Stage::Schemas(ir.schemas().collect())
            }
        });
    }
}
