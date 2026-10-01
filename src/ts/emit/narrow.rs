//! Narrowing a request / response union to the variants a `where` or a
//! bound variable selects: `Extract<..., { status: 200 }>`.

use oxc_ast::ast::*;

use super::shape::Env;
use crate::config::Root;
use crate::ir::Status;
use crate::list::distinct;
use crate::select::{Subject, Where};
use crate::template::Var;
use crate::ts::Generator;

impl<'a> Generator<'a> {
    /// `ty` (the operation's `request` / `response` union) narrowed to the
    /// variants matching `filter` and the variant fields `env` has bound (a
    /// key or group over `{response.status}` narrows `{response}` below it):
    /// `Extract<ty, D>` with `D` built from the fields that tell the matching
    /// variants apart.
    pub(super) fn narrow(
        &self,
        ty: TSType<'a>,
        root: Root,
        filter: Option<&Where>,
        env: &Env,
    ) -> TSType<'a> {
        let Some(variants) = Variants::new(root, filter, env) else {
            return ty;
        };
        if variants.all_matched() {
            return ty;
        }
        if variants.none_matched() {
            return self.b.never();
        }
        let by = match variants.exact(asks_status_only(filter, env)) {
            Some(fields) => self.discriminant(root, &fields),
            // No field set picks out exactly these: one literal per variant.
            None => {
                let each = variants
                    .matched()
                    .map(|d| self.discriminant(root, &d.fields()));
                self.b.union(each)
            }
        };
        self.b.generic("Extract", [ty, by])
    }

    /// `{ status: 200 | 201; contentType: "application/json" }`, with only
    /// the given fields.
    fn discriminant(&self, root: Root, fields: &Fields<'_>) -> TSType<'a> {
        let b = self.b;
        let mut members = b.member_builder();
        if let Some(statuses) = &fields.statuses {
            members = members.prop("status", b.union(statuses.iter().map(|s| self.status(*s))));
        }
        if let Some(content_types) = &fields.content_types {
            let literals = content_types
                .iter()
                .flatten()
                .map(|ct| b.string_literal(b.str(ct)));
            let bodiless = content_types.contains(&None);
            members = match root {
                // A response without a body has `contentType: null`.
                Root::Response => {
                    let null = bodiless.then(|| b.null());
                    members.prop("contentType", b.union(literals.chain(null)))
                }
                // A request without one has `contentType?: never`, which
                // only `contentType?: undefined` matches.
                _ if bodiless => {
                    let undefined = b.undefined();
                    members.optional_prop("contentType", b.union(literals.chain([undefined])))
                }
                _ => members.prop("contentType", b.union(literals)),
            };
        }
        members.into_literal()
    }
}

/// One operation's request / response variants, each with whether the
/// narrowing keeps it.
struct Variants<'o> {
    root: Root,
    all: Vec<(Discriminant<'o>, bool)>,
}

impl<'o> Variants<'o> {
    /// `None` for [`Root::Op`], which has no variants to narrow.
    fn new(root: Root, filter: Option<&Where>, env: &Env<'o>) -> Option<Self> {
        let keep = |v: &dyn Subject| filter.is_none_or(|f| f.matches(v));
        let all = match root {
            Root::Op => return None,
            Root::Request => {
                let content_type = env.bound(Var::RequestContentType);
                env.op
                    .requests
                    .iter()
                    .map(|v| {
                        let matched = keep(v)
                            && content_type.is_none_or(|ct| v.content_type.as_deref() == Some(ct));
                        (Discriminant::request(v.content_type.as_deref()), matched)
                    })
                    .collect()
            }
            Root::Response => {
                let status = env.bound(Var::ResponseStatus);
                let content_type = env.bound(Var::ResponseContentType);
                env.op
                    .responses
                    .iter()
                    .map(|v| {
                        let matched = keep(v)
                            && status.is_none_or(|s| v.status.to_string() == s)
                            && content_type.is_none_or(|ct| v.content_type.as_deref() == Some(ct));
                        (
                            Discriminant::response(v.status, v.content_type.as_deref()),
                            matched,
                        )
                    })
                    .collect()
            }
        };
        Some(Self { root, all })
    }

    fn matched(&self) -> impl Iterator<Item = &Discriminant<'o>> {
        self.all.iter().filter(|(_, m)| *m).map(|(d, _)| d)
    }

    fn all_matched(&self) -> bool {
        self.all.iter().all(|(_, m)| *m)
    }

    fn none_matched(&self) -> bool {
        !self.all.iter().any(|(_, m)| *m)
    }

    /// The fewest fields whose matching values pick out exactly the matching
    /// variants: content type, status, or both, trying status first when
    /// `prefer_status`. `None` when no field set does.
    fn exact(&self, prefer_status: bool) -> Option<Fields<'o>> {
        let content_types = distinct(self.matched().map(|d| d.content_type));
        let statuses = distinct(self.matched().filter_map(|d| d.status));
        let by_content_type = Fields {
            content_types: Some(content_types.clone()),
            statuses: None,
        };
        let by_status = Fields {
            content_types: None,
            statuses: Some(statuses.clone()),
        };
        let both = Fields {
            content_types: Some(content_types),
            statuses: Some(statuses),
        };
        let (first, second) = if prefer_status {
            (by_status, by_content_type)
        } else {
            (by_content_type, by_status)
        };
        [first, second, both].into_iter().find(|fields| {
            fields.applies(self.root)
                && self
                    .all
                    .iter()
                    .all(|(d, matched)| fields.covers(d) == *matched)
        })
    }
}

/// The fields a discriminating literal names, and the values it allows.
struct Fields<'o> {
    content_types: Option<Vec<Option<&'o str>>>,
    statuses: Option<Vec<Status>>,
}

impl<'o> Fields<'o> {
    /// Requests have no `status`, so a status-only literal can't name them.
    fn applies(&self, root: Root) -> bool {
        self.content_types.is_some() || root == Root::Response
    }

    /// Whether `d`'s values are among the allowed ones of every named field.
    fn covers(&self, d: &Discriminant<'o>) -> bool {
        let content_type = self.content_types.as_ref();
        let status = self.statuses.as_ref();
        content_type.is_none_or(|v| v.contains(&d.content_type))
            && status.is_none_or(|v| d.status.is_some_and(|s| v.contains(&s)))
    }
}

/// Describe the narrowing by the field it was asked by when that's exact:
/// `{ status: 204 }` rather than `{ contentType: null }`.
fn asks_status_only(filter: Option<&Where>, env: &Env) -> bool {
    let asks_status =
        filter.is_some_and(|f| f.status.is_some()) || env.bound(Var::ResponseStatus).is_some();
    let asks_content_type = filter.is_some_and(|f| f.content_type.is_some())
        || env.bound(Var::RequestContentType).is_some()
        || env.bound(Var::ResponseContentType).is_some();
    asks_status && !asks_content_type
}

/// What tells a request / response variant apart from its siblings.
#[derive(Debug, Clone, Copy)]
struct Discriminant<'o> {
    /// Responses only.
    status: Option<Status>,
    content_type: Option<&'o str>,
}

impl<'o> Discriminant<'o> {
    fn request(content_type: Option<&'o str>) -> Self {
        Self {
            status: None,
            content_type,
        }
    }

    fn response(status: Status, content_type: Option<&'o str>) -> Self {
        Self {
            status: Some(status),
            content_type,
        }
    }

    /// Exactly this variant's own fields.
    fn fields(&self) -> Fields<'o> {
        Fields {
            content_types: Some(vec![self.content_type]),
            statuses: self.status.map(|s| vec![s]),
        }
    }
}
