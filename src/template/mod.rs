//! The tiny string template language used throughout the config:
//! `"{Method}{Path}"`, `"{METHOD} {path}"`, `"{id|pascal}"`,
//! `"{request.content_type|pascal}Ops"`.
//!
//! - `{var}` interpolates a variable. The variables mirror an operation (and
//!   its `--emit-ir` entry): its fields, and `.` for those of its request /
//!   response (see [`Var`]). Which ones are available, and whether as text
//!   or as a reference to a generated type, depends on where the template is
//!   used (see [`Template::check`] and [`Template::single_var`]).
//! - The *casing of the name* picks a case: `{method}` -> `get`, `{Method}` ->
//!   `Get`, `{METHOD}` -> `GET`.
//! - `|filter` applies a transform: `pascal`, `camel`, `lower`, `upper`.
//! - `{{` / `}}` are literal braces.
//!
//! Rendering returns `None` when a variable has no value (e.g. `{id}` on an
//! operation without an `operationId`); callers decide whether that means
//! "skip" or "fall back".

use std::fmt;

use anyhow::{Result, bail};

use crate::case::Case;

#[cfg(test)]
mod tests;
mod var;

pub use var::Var;

/// Old names, for a hint in the "unknown variable" error.
const RENAMED: &[(&str, Var)] = &[
    ("op", Var::TypeName),
    ("type", Var::TypeName),
    ("op.request", Var::Request),
    ("op.response", Var::Response),
    ("operationId", Var::Id),
    ("tag", Var::Tags),
];

/// Deserialised from (and serialised to) its source string, so it reads as
/// a plain string in config files and in the generated config schema.
///
/// Deserialising never fails: a malformed template keeps its error and
/// reports it from [`Template::check`], which config validation runs on
/// load, so the error comes with the config path it's at.
#[derive(Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(from = "String", into = "String")]
#[cfg_attr(feature = "facet", derive(facet::Facet), facet(proxy = String))]
pub struct Template {
    source: String,
    #[cfg_attr(feature = "facet", facet(opaque))]
    parts: Vec<Part>,
    error: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
enum Part {
    Literal(String),
    /// A variable with its filters, the one its casing picked first.
    Var {
        var: Var,
        filters: Vec<Filter>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Filter {
    Pascal,
    Camel,
    Lower,
    Upper,
}

impl Template {
    pub fn parse(source: &str) -> Result<Self> {
        Ok(Self {
            source: source.to_string(),
            parts: parse_parts(source)?,
            error: None,
        })
    }

    /// Like [`Template::parse`], but keeps a parse error for
    /// [`Template::check`] to report instead of failing.
    pub fn lenient(source: String) -> Self {
        match parse_parts(&source) {
            Ok(parts) => Self {
                source,
                parts,
                error: None,
            },
            Err(e) => Self {
                source,
                parts: Vec::new(),
                error: Some(e.to_string()),
            },
        }
    }

    pub fn source(&self) -> &str {
        &self.source
    }

    /// Variables referenced (`{METHOD}` is [`Var::Method`]).
    pub fn vars(&self) -> impl Iterator<Item = Var> + '_ {
        self.parts.iter().filter_map(|p| match p {
            Part::Var { var, .. } => Some(*var),
            Part::Literal(_) => None,
        })
    }

    /// The variable if the template is exactly one, as written (no case
    /// change, no filters, no surrounding text).
    pub fn single_var(&self) -> Option<Var> {
        match self.parts.as_slice() {
            [Part::Var { var, filters }] if filters.is_empty() => Some(*var),
            _ => None,
        }
    }

    pub fn uses(&self, var: Var) -> bool {
        self.vars().any(|v| v == var)
    }

    /// Error if the template uses a variable not in `allowed`.
    pub fn check(&self, allowed: &[Var], context: &str) -> Result<()> {
        if let Some(error) = &self.error {
            bail!("{context}: {error}");
        }
        for var in self.vars() {
            if !allowed.contains(&var) {
                let allowed = allowed
                    .iter()
                    .map(|v| format!("{{{v}}}"))
                    .collect::<Vec<_>>();
                let allowed = if allowed.is_empty() {
                    "no variables".to_string()
                } else {
                    allowed.join(", ")
                };
                bail!("{context}: `{{{var}}}` isn't available here (allowed: {allowed})");
            }
        }
        Ok(())
    }

    /// `None` if any referenced variable has no value (or the template didn't
    /// parse).
    pub fn render(&self, vars: impl Fn(Var) -> Option<String>) -> Option<String> {
        if self.error.is_some() {
            return None;
        }
        let mut out = String::new();
        for part in &self.parts {
            match part {
                Part::Literal(s) => out.push_str(s),
                Part::Var { var, filters } => {
                    let mut value = vars(*var)?;
                    for filter in filters {
                        value = filter.apply(&value);
                    }
                    out.push_str(&value);
                }
            }
        }
        Some(out)
    }
}

fn parse_parts(source: &str) -> Result<Vec<Part>> {
    let mut parts = Vec::new();
    let mut literal = String::new();
    let mut chars = source.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '{' if chars.peek() == Some(&'{') => {
                chars.next();
                literal.push('{');
            }
            '}' if chars.peek() == Some(&'}') => {
                chars.next();
                literal.push('}');
            }
            '{' => {
                let mut body = String::new();
                let mut closed = false;
                for c in chars.by_ref() {
                    if c == '}' {
                        closed = true;
                        break;
                    }
                    body.push(c);
                }
                if !closed {
                    bail!("in template {source:?}: unterminated `{{`");
                }
                if !literal.is_empty() {
                    parts.push(Part::Literal(std::mem::take(&mut literal)));
                }
                let var =
                    parse_var(&body).map_err(|e| anyhow::anyhow!("in template {source:?}: {e}"))?;
                parts.push(var);
            }
            '}' => bail!("in template {source:?}: unmatched `}}` (use `}}}}` for a literal brace)"),
            c => literal.push(c),
        }
    }
    if !literal.is_empty() {
        parts.push(Part::Literal(literal));
    }
    Ok(parts)
}

fn parse_var(body: &str) -> Result<Part> {
    let mut pieces = body.split('|').map(str::trim);
    let raw = pieces.next().unwrap_or_default();
    let Some(var) = Var::parse(raw) else {
        if let Some((_, new)) = RENAMED
            .iter()
            .find(|(old, _)| old.eq_ignore_ascii_case(raw))
        {
            bail!("unknown variable `{{{raw}}}`; it's `{{{new}}}` now");
        }
        let known = Var::ALL
            .iter()
            .map(|v| format!("{{{v}}}"))
            .collect::<Vec<_>>();
        bail!("unknown variable `{{{raw}}}` (known: {})", known.join(", "));
    };
    let name = var.name();
    // The casing of the name picks a case: `{Method}` -> `Get`, `{METHOD}` -> `GET`.
    let case = if raw == name {
        None
    } else if raw.chars().all(|c| !c.is_ascii_lowercase()) {
        Some(Filter::Upper)
    } else if raw.starts_with(|c: char| c.is_ascii_uppercase()) {
        Some(Filter::Pascal)
    } else {
        bail!(
            "`{{{raw}}}`: use `{{{name}}}`, `{{{}}}` or `{{{}}}`",
            Case::pascal_case(&name),
            name.to_uppercase()
        );
    };
    let filters = pieces
        .map(|f| match f {
            "pascal" => Ok(Filter::Pascal),
            "camel" => Ok(Filter::Camel),
            "lower" => Ok(Filter::Lower),
            "upper" => Ok(Filter::Upper),
            f => bail!("unknown filter `{f}` (known: pascal, camel, lower, upper)"),
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(Part::Var {
        var,
        filters: case.into_iter().chain(filters).collect(),
    })
}

impl Filter {
    fn apply(self, s: &str) -> String {
        match self {
            // Path-shaped values get `{param}` -> `ByParam`.
            Filter::Pascal if s.contains('/') => Case::path_pascal_case(&s),
            Filter::Pascal => Case::pascal_case(&s),
            Filter::Camel => Case::lower_first(&Filter::Pascal.apply(s)),
            Filter::Lower => Case::lower(&s),
            Filter::Upper => s.to_uppercase(),
        }
    }
}

impl fmt::Debug for Template {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Template({:?})", self.source)
    }
}

/// Conversions for serde's `from` / `into` and facet's `proxy = String`.
impl From<String> for Template {
    fn from(source: String) -> Self {
        Self::lenient(source)
    }
}

impl From<&Template> for String {
    fn from(template: &Template) -> Self {
        template.source.clone()
    }
}

impl From<Template> for String {
    fn from(template: Template) -> Self {
        template.source
    }
}
