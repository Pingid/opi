//! Checks that parsing alone can't make.

use anyhow::{Result, bail};

use super::{Config, Emit};
use crate::select::Scope;
use crate::template::Var;

impl Config {
    /// Checks that parsing alone can't: variables used where they have no
    /// value, selector fields that don't apply, unknown methods / statuses,
    /// values that aren't references.
    pub(crate) fn validate(&self) -> Result<()> {
        self.filter.validate(Scope::Operation, "filter")?;
        if let Some(keep) = &self.schemas.keep {
            keep.validate(Scope::Schema, "schemas.keep")?;
        }
        self.operation.name.check(Var::NAME, "operation.name")?;
        for (i, o) in self.operation.overrides.iter().enumerate() {
            let context = format!("operation.overrides[{i}]");
            o.filter
                .validate(Scope::Operation, &format!("{context}.where"))?;
            o.name.check(Var::NAME, &format!("{context}.name"))?;
        }
        for (i, emit) in self.emit.iter().enumerate() {
            emit.validate(i)?;
        }
        Ok(())
    }
}

impl Emit {
    fn validate(&self, index: usize) -> Result<()> {
        let context = format!("emit[{index}] ({:?})", self.name.source());
        if let Some(filter) = &self.filter {
            filter.validate(Scope::Operation, &format!("{context}.where"))?;
        }

        let group_var = self.group_var();
        if let Some(var) = &self.group_by
            && group_var.is_none()
        {
            let names: Vec<_> = Var::NAME.iter().map(|v| v.name()).collect();
            bail!(
                "{context}.group_by: `{var}` isn't a variable to group on (one of: {})",
                names.join(", ")
            );
        }
        self.name
            .check(group_var.as_slice(), &format!("{context}.name"))?;
        if let Some(var) = group_var
            && !self.name.uses(var)
        {
            bail!(
                "{context}.name: doesn't use `{{{var}}}`, so every group would get the same \
                 name"
            );
        }

        self.shape.validate(Var::TEXT, &format!("{context}.shape"))
    }
}
