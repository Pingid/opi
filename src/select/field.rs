//! The fields of a [`Where`], which of them a [`Scope`] allows, and
//! validation.

use std::fmt;

use anyhow::{Result, bail};

use super::{List, Where};
use crate::ir::Method;

/// A field of [`Where`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Field {
    Method,
    Path,
    Tag,
    OperationId,
    Deprecated,
    Request,
    Response,
    ContentType,
    Status,
    Name,
    Not,
    Any,
}

impl Field {
    /// Every field, in declaration order.
    pub const ALL: [Field; 12] = [
        Field::Method,
        Field::Path,
        Field::Tag,
        Field::OperationId,
        Field::Deprecated,
        Field::Request,
        Field::Response,
        Field::ContentType,
        Field::Status,
        Field::Name,
        Field::Not,
        Field::Any,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Field::Method => "method",
            Field::Path => "path",
            Field::Tag => "tag",
            Field::OperationId => "operation_id",
            Field::Deprecated => "deprecated",
            Field::Request => "request",
            Field::Response => "response",
            Field::ContentType => "content_type",
            Field::Status => "status",
            Field::Name => "name",
            Field::Not => "not",
            Field::Any => "any",
        }
    }
}

impl fmt::Display for Field {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// What a selector is applied to, which decides the fields it may use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    Operation,
    Request,
    Response,
    Schema,
}

impl Scope {
    fn fields(self) -> &'static [Field] {
        use Field::*;
        match self {
            Scope::Operation => &[
                Method,
                Path,
                Tag,
                OperationId,
                Deprecated,
                Request,
                Response,
                Not,
                Any,
            ],
            Scope::Request => &[ContentType, Not, Any],
            Scope::Response => &[ContentType, Status, Not, Any],
            Scope::Schema => &[Name, Not, Any],
        }
    }
}

impl Where {
    fn is_set(&self, field: Field) -> bool {
        match field {
            Field::Method => self.method.is_some(),
            Field::Path => self.path.is_some(),
            Field::Tag => self.tag.is_some(),
            Field::OperationId => self.operation_id.is_some(),
            Field::Deprecated => self.deprecated.is_some(),
            Field::Request => self.request.is_some(),
            Field::Response => self.response.is_some(),
            Field::ContentType => self.content_type.is_some(),
            Field::Status => self.status.is_some(),
            Field::Name => self.name.is_some(),
            Field::Not => self.not.is_some(),
            Field::Any => self.any.is_some(),
        }
    }

    /// Fields that are set, in declaration order.
    pub(super) fn set_fields(&self) -> impl Iterator<Item = Field> + '_ {
        Field::ALL.into_iter().filter(move |f| self.is_set(*f))
    }

    /// Catch fields that don't apply here and typos that would otherwise
    /// silently match nothing. `context` names the selector in errors.
    pub fn validate(&self, scope: Scope, context: &str) -> Result<()> {
        let allowed = scope.fields();
        for field in self.set_fields() {
            if !allowed.contains(&field) {
                let names: Vec<_> = allowed.iter().map(|f| f.name()).collect();
                bail!(
                    "{context}: `{field}` isn't available here (allowed: {})",
                    names.join(", ")
                );
            }
        }
        for method in self.method.iter().flat_map(List::as_slice) {
            if Method::parse(method).is_none() {
                bail!("{context}: unknown method {method:?}");
            }
        }
        for status in self.status.iter().flat_map(List::as_slice) {
            status
                .parse()
                .map_err(|e| anyhow::anyhow!("{context}: {e}"))?;
        }
        if let Some(request) = &self.request {
            request.validate(Scope::Request, &format!("{context}.request"))?;
        }
        if let Some(response) = &self.response {
            response.validate(Scope::Response, &format!("{context}.response"))?;
        }
        if let Some(not) = &self.not {
            not.validate(scope, &format!("{context}.not"))?;
        }
        for (i, any) in self.any.iter().flatten().enumerate() {
            any.validate(scope, &format!("{context}.any[{i}]"))?;
        }
        Ok(())
    }
}
