//! Evaluating a selector against a [`Subject`].

use super::field::Field;
use super::{List, Where, path_matches, wildcard};
use crate::ir::{Operation, Request, Response, Status};

/// The order fields are checked (and blamed) in: the nested selectors last.
const CHECK_ORDER: [Field; 12] = [
    Field::Method,
    Field::Path,
    Field::Tag,
    Field::OperationId,
    Field::Deprecated,
    Field::ContentType,
    Field::Status,
    Field::Name,
    Field::Not,
    Field::Any,
    Field::Request,
    Field::Response,
];

impl Where {
    /// The first field that doesn't match (`deprecated`,
    /// `request.content_type`, `not.tag`), `None` if `subject` matches. For
    /// reporting why something was left out.
    pub fn mismatch(&self, subject: &(impl Subject + ?Sized)) -> Option<String> {
        let field = CHECK_ORDER.into_iter().find(|&f| !self.holds(f, subject))?;
        Some(self.blame(field, subject))
    }

    /// Whether `field`'s constraint holds for `s` (an unset field always does).
    fn holds(&self, field: Field, s: &(impl Subject + ?Sized)) -> bool {
        match field {
            Field::Method => any(&self.method, |m| {
                s.method().is_some_and(|x| x.eq_ignore_ascii_case(m))
            }),
            Field::Path => any(&self.path, |g| s.path().is_some_and(|p| path_matches(g, p))),
            Field::Tag => any(&self.tag, |g| s.tags().iter().any(|t| wildcard(g, t))),
            Field::OperationId => any(&self.operation_id, |g| {
                s.operation_id().is_some_and(|id| wildcard(g, id))
            }),
            Field::Deprecated => self.deprecated.is_none_or(|d| s.deprecated() == Some(d)),
            Field::ContentType => any(&self.content_type, |g| {
                s.content_type()
                    .is_some_and(|ct| wildcard(&g.to_ascii_lowercase(), &ct.to_ascii_lowercase()))
            }),
            Field::Status => any(&self.status, |p| s.status().is_some_and(|st| p.matches(st))),
            Field::Name => any(&self.name, |g| s.name().is_some_and(|n| wildcard(g, n))),
            Field::Not => self.not.as_ref().is_none_or(|not| !not.matches(s)),
            Field::Any => self
                .any
                .as_ref()
                .is_none_or(|any| any.iter().any(|w| w.matches(s))),
            Field::Request => self
                .request
                .as_ref()
                .is_none_or(|w| s.requests().iter().any(|v| w.matches(v))),
            Field::Response => self
                .response
                .as_ref()
                .is_none_or(|w| s.responses().iter().any(|v| w.matches(v))),
        }
    }

    /// Why `field` failed: `deprecated`; `not.tag`; `request.content_type` /
    /// `response.status`, blaming the first variant's own mismatch.
    fn blame(&self, field: Field, s: &(impl Subject + ?Sized)) -> String {
        match field {
            Field::Not => {
                let inner = self.not.as_ref().expect("checked").set_fields().next();
                format!("not.{}", inner.map_or("{}", Field::name))
            }
            Field::Request => nested(
                "request",
                self.request.as_deref(),
                s.requests(),
                "content_type",
            ),
            Field::Response => nested(
                "response",
                self.response.as_deref(),
                s.responses(),
                "status",
            ),
            field => field.name().to_string(),
        }
    }
}

/// `<prefix>.<field>`: the first variant's own mismatch, else `default`.
fn nested(
    prefix: &str,
    selector: Option<&Where>,
    variants: &[impl Subject],
    default: &str,
) -> String {
    let inner = variants.first().and_then(|v| selector?.mismatch(v));
    format!("{prefix}.{}", inner.as_deref().unwrap_or(default))
}

/// Something a [`Where`] can be matched against. Fields that don't apply
/// keep the defaults (no value), which validation ensures aren't asked for.
pub trait Subject {
    fn method(&self) -> Option<&str> {
        None
    }
    fn path(&self) -> Option<&str> {
        None
    }
    fn tags(&self) -> &[String] {
        &[]
    }
    fn operation_id(&self) -> Option<&str> {
        None
    }
    fn deprecated(&self) -> Option<bool> {
        None
    }
    fn requests(&self) -> &[Request] {
        &[]
    }
    fn responses(&self) -> &[Response] {
        &[]
    }
    fn content_type(&self) -> Option<&str> {
        None
    }
    fn status(&self) -> Option<Status> {
        None
    }
    fn name(&self) -> Option<&str> {
        None
    }
}

impl Subject for Operation {
    fn method(&self) -> Option<&str> {
        Some(self.method.as_str())
    }
    fn path(&self) -> Option<&str> {
        Some(&self.path)
    }
    fn tags(&self) -> &[String] {
        &self.tags
    }
    fn operation_id(&self) -> Option<&str> {
        self.id.as_deref()
    }
    fn deprecated(&self) -> Option<bool> {
        Some(self.deprecated)
    }
    fn requests(&self) -> &[Request] {
        &self.requests
    }
    fn responses(&self) -> &[Response] {
        &self.responses
    }
}

impl<S> Subject for Request<S> {
    fn content_type(&self) -> Option<&str> {
        self.content_type.as_deref()
    }
}

impl<S> Subject for Response<S> {
    fn content_type(&self) -> Option<&str> {
        self.content_type.as_deref()
    }
    fn status(&self) -> Option<Status> {
        Some(self.status)
    }
}

/// A schema, by its raw `components.schemas` name.
impl Subject for str {
    fn name(&self) -> Option<&str> {
        Some(self)
    }
}

/// `None` (field not given) matches everything.
fn any<T>(list: &Option<List<T>>, f: impl Fn(&T) -> bool) -> bool {
    list.as_ref().is_none_or(|l| l.as_slice().iter().any(f))
}
