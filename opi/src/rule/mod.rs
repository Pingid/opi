use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ast;

pub mod each;
pub mod emit;
pub mod r#where;

use each::Each;
use emit::Emit;
use r#where::Where;

mod build;
pub use build::*;

/// Top-level rule representing one transform entry in the YAML/JSON list.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Rule {
    pub each: Option<Each>,
    #[serde(rename = "where", skip_serializing_if = "Option::is_none")]
    pub where_: Option<Where>,
    pub emit: Option<Emit>,
}

impl TryFrom<serde_json::Value> for Rule {
    type Error = anyhow::Error;
    fn try_from(value: serde_json::Value) -> Result<Rule, Self::Error> {
        Ok(serde_json::from_value(value)?)
    }
}

impl Default for Rule {
    fn default() -> Self {
        Self::new()
    }
}

impl Rule {
    pub const ROUTE: Each = Each::DEFAULT;
    pub const WHERE: Option<Where> = None;
    pub const EMIT: Emit = Emit::DEFAULT;

    pub fn new() -> Self {
        Self {
            each: Some(Self::ROUTE),
            where_: Self::WHERE,
            emit: Some(Self::EMIT),
        }
    }

    pub fn for_each(mut self, each: Each) -> Self {
        self.each = Some(each);
        self
    }
    pub fn where_(mut self, where_: Where) -> Self {
        self.where_ = Some(where_);
        self
    }
    pub fn emit(mut self, emit: impl TryInto<Emit, Error = anyhow::Error>) -> anyhow::Result<Self> {
        self.emit = Some(emit.try_into()?);
        Ok(self)
    }
}

/// `each` stages, `where` filters. `emit` isn't a [`ViewRule`]: it collects
/// declarations in [`Builder::with`] so they can be resolved across rules.
impl<'a> ViewRule<'a> for Rule {
    fn stage(&mut self, stage: &mut ItemSets<'a>, ir: &'a ast::Doc) {
        if let Some(each) = &mut self.each {
            each.stage(stage, ir);
        }
    }
    fn retain(&mut self, view: &mut Stage<'a>) {
        if let Some(where_) = &mut self.where_ {
            where_.retain(view);
        }
    }
    fn update(&mut self, view: &mut Stage<'a>) {
        if let Some(where_) = &mut self.where_ {
            where_.update(view);
        }
    }
}
