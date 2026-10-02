use std::path::Path;

use crate::{ast, util::JsonOrYamlFile};

mod ir;

mod refs;
pub use refs::*;

mod model;
pub use model::*;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Oapi(Document);

impl Oapi {
    pub fn from_file(path: &Path) -> anyhow::Result<Self> {
        JsonOrYamlFile::from_file(path)
    }

    pub fn lower(self) -> anyhow::Result<ast::Doc> {
        ir::lower(self.0)
    }
}
