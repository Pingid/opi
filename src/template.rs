//! The tiny string template language used throughout the config:
//! `"{Method}{Path}"`, `"{METHOD} {path}"`, `"{operationId|pascal}"`.
//!
//! - `{var}` interpolates a variable. The known variables are listed in
//!   [`VARS`]; which ones are available depends on where the template is used
//!   (see [`Template::check`]).
//! - The *casing of the name* picks a case: `{method}` -> `get`, `{Method}` ->
//!   `Get`, `{METHOD}` -> `GET`.
//! - `|filter` applies a transform: `pascal`, `camel`, `lower`, `upper`.
//! - `{{` / `}}` are literal braces.
//!
//! Rendering returns `None` when a variable has no value (e.g.
//! `{operationId}` on an operation without one); callers decide whether that
//! means "skip" or "fall back".

use std::fmt;

use anyhow::{Result, bail};

use crate::case::{pascal_case, path_pascal_case};

/// Every variable any template can reference.
pub const VARS: &[&str] = &["method", "path", "operationId", "tag", "type"];

/// Deserialised from (and serialised to) its source string, so it reads as
/// a plain string in config files and in the generated config schema.
///
/// Deserialising never fails: facet discards a proxy's conversion error, so
/// a malformed template keeps its error and reports it from
/// [`Template::check`], which config validation runs on load.
#[derive(Clone, PartialEq, facet::Facet)]
#[facet(proxy = String)]
pub struct Template {
    source: String,
    #[facet(opaque)]
    parts: Vec<Part>,
    error: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
enum Part {
    Literal(String),
    Var {
        name: &'static str,
        case: Case,
        filters: Vec<Filter>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Case {
    AsIs,
    Pascal,
    Upper,
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

    /// Variables referenced, by canonical name (`{METHOD}` -> `method`).
    pub fn vars(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.parts.iter().filter_map(|p| match p {
            Part::Var { name, .. } => Some(*name),
            Part::Literal(_) => None,
        })
    }

    /// Error if the template uses a variable not in `allowed`.
    pub fn check(&self, allowed: &[&str], context: &str) -> Result<()> {
        if let Some(error) = &self.error {
            bail!("{context}: {error}");
        }
        for var in self.vars() {
            if !allowed.contains(&var) {
                let allowed = allowed.iter().map(|v| format!("{{{v}}}")).collect::<Vec<_>>();
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

    /// `None` if any referenced variable has no value.
    pub fn render(&self, vars: impl Fn(&str) -> Option<String>) -> Option<String> {
        if self.error.is_some() {
            return None;
        }
        let mut out = String::new();
        for part in &self.parts {
            match part {
                Part::Literal(s) => out.push_str(s),
                Part::Var {
                    name,
                    case,
                    filters,
                } => {
                    let mut value = vars(name)?;
                    value = match case {
                        Case::AsIs => value,
                        Case::Pascal => Filter::Pascal.apply(&value),
                        Case::Upper => Filter::Upper.apply(&value),
                    };
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
                let var = parse_var(&body)
                    .map_err(|e| anyhow::anyhow!("in template {source:?}: {e}"))?;
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
    let Some(&name) = VARS.iter().find(|v| v.eq_ignore_ascii_case(raw)) else {
        let known = VARS.iter().map(|v| format!("{{{v}}}")).collect::<Vec<_>>();
        bail!("unknown variable `{{{raw}}}` (known: {})", known.join(", "));
    };
    let case = if raw == name {
        Case::AsIs
    } else if raw.chars().all(|c| !c.is_ascii_lowercase()) {
        Case::Upper
    } else if raw.starts_with(|c: char| c.is_ascii_uppercase()) {
        Case::Pascal
    } else {
        bail!("`{{{raw}}}`: use `{{{name}}}`, `{{{}}}` or `{{{}}}`", pascal_case(name), name.to_uppercase());
    };
    let filters = pieces
        .map(|f| match f {
            "pascal" => Ok(Filter::Pascal),
            "camel" => Ok(Filter::Camel),
            "lower" => Ok(Filter::Lower),
            "upper" => Ok(Filter::Upper),
            f => bail!("unknown filter `{f}` (known: pascal, camel, lower, upper)"),
        })
        .collect::<Result<_>>()?;
    Ok(Part::Var {
        name,
        case,
        filters,
    })
}

impl Filter {
    fn apply(self, s: &str) -> String {
        match self {
            // Path-shaped values get `{param}` -> `ByParam`.
            Filter::Pascal if s.contains('/') => path_pascal_case(s),
            Filter::Pascal => pascal_case(s),
            Filter::Camel => {
                let pascal = Filter::Pascal.apply(s);
                let mut chars = pascal.chars();
                chars
                    .next()
                    .map(|c| c.to_lowercase().chain(chars).collect())
                    .unwrap_or_default()
            }
            Filter::Lower => s.to_lowercase(),
            Filter::Upper => s.to_uppercase(),
        }
    }
}

impl fmt::Debug for Template {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Template({:?})", self.source)
    }
}

/// Proxy conversions for `#[facet(proxy = String)]`.
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

#[cfg(test)]
mod tests {
    use super::*;

    fn render(t: &str) -> Option<String> {
        Template::parse(t).unwrap().render(|v| match v {
            "method" => Some("get".into()),
            "path" => Some("/components/{name}".into()),
            "tag" => Some("shop-api".into()),
            _ => None,
        })
    }

    #[test]
    fn cases_and_filters() {
        assert_eq!(render("{Method}{Path}").unwrap(), "GetComponentsByName");
        assert_eq!(render("{METHOD} {path}").unwrap(), "GET /components/{name}");
        assert_eq!(render("{tag|camel}Routes").unwrap(), "shopApiRoutes");
        assert_eq!(render("{Tag}").unwrap(), "ShopApi");
        assert_eq!(render("{{literal}}").unwrap(), "{literal}");
        assert_eq!(render("{operationId}"), None);
    }

    #[test]
    fn errors() {
        assert!(Template::parse("{nope}").is_err());
        assert!(Template::parse("{method|shout}").is_err());
        assert!(Template::parse("oops}").is_err());
        assert!(Template::parse("{method").is_err());
        let t = Template::parse("{type}").unwrap();
        assert!(t.check(&["method"], "test").is_err());
    }
}
