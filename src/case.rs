//! Identifier casing shared by templates and the TS backend.

pub trait Case {
    fn pascal_case(&self) -> String;
    fn path_pascal_case(&self) -> String;
    fn identifier(&self) -> String;
    fn is_identifier(&self) -> bool;
    fn lower(&self) -> String;
    fn lower_first(&self) -> String;
}

impl<T> Case for T
where
    T: AsRef<str>,
{
    fn pascal_case(&self) -> String {
        pascal_case(self.as_ref())
    }
    fn path_pascal_case(&self) -> String {
        path_pascal_case(self.as_ref())
    }
    fn identifier(&self) -> String {
        identifier(self.as_ref())
    }
    fn is_identifier(&self) -> bool {
        is_identifier(self.as_ref())
    }
    fn lower(&self) -> String {
        self.as_ref().to_lowercase()
    }
    fn lower_first(&self) -> String {
        lower_first(self.as_ref())
    }
}

/// `/components/{name}` -> `ComponentsByName`.
fn path_pascal_case(path: &str) -> String {
    let mut out = String::new();
    for segment in path.split('/').filter(|s| !s.is_empty()) {
        match segment.strip_prefix('{').and_then(|s| s.strip_suffix('}')) {
            Some(param) => {
                out.push_str("By");
                out.push_str(&pascal_case(param));
            }
            None => out.push_str(&pascal_case(segment)),
        }
    }
    out
}

/// `get-component_by.name` -> `GetComponentByName`. Casing within a word is
/// preserved (`HTTPError` stays `HTTPError`). Always returns a valid identifier.
fn pascal_case(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for word in s.split(|c: char| !c.is_ascii_alphanumeric()) {
        let mut chars = word.chars();
        if let Some(first) = chars.next() {
            out.extend(first.to_uppercase());
            out.push_str(chars.as_str());
        }
    }
    if out.is_empty() || out.starts_with(|c: char| c.is_ascii_digit()) {
        out.insert(0, '_');
    }
    out
}

/// `ShopApi` -> `shopApi`.
fn lower_first(s: &str) -> String {
    let mut chars = s.chars();
    chars
        .next()
        .map(|c| c.to_lowercase().chain(chars).collect())
        .unwrap_or_default()
}

/// Force a rendered name into a valid identifier: invalid characters become
/// `_`, and a leading digit gets a `_` prefix.
fn identifier(s: &str) -> String {
    let mut out: String = s
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '$' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if out.is_empty() || out.starts_with(|c: char| c.is_ascii_digit()) {
        out.insert(0, '_');
    }
    out
}

/// ASCII-only identifier check. Non-ASCII keys get quoted, which is always
/// valid TS, so there's no need to pull in `oxc_syntax` for full Unicode rules.
fn is_identifier(s: &str) -> bool {
    let mut chars = s.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_alphabetic() || c == '_' || c == '$')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pascal() {
        assert_eq!(pascal_case("get-component_by.name"), "GetComponentByName");
        assert_eq!(pascal_case("GET"), "GET");
        assert_eq!(pascal_case("simple-user"), "SimpleUser");
        assert_eq!(pascal_case("2fa"), "_2fa");
        assert_eq!(
            path_pascal_case("/orgs/{org}/projectsV2"),
            "OrgsByOrgProjectsV2"
        );
    }

    #[test]
    fn identifiers() {
        assert_eq!(identifier("GET /a-b"), "GET__a_b");
        assert_eq!(identifier("2fa"), "_2fa");
        assert_eq!(identifier(""), "_");
        assert_eq!(lower_first("ShopApi"), "shopApi");
        for raw in ["GET /a-b", "2fa", "", "ok$"] {
            assert!(is_identifier(&identifier(raw)), "{raw:?}");
        }
        assert!(!is_identifier("200") && !is_identifier("content-type"));
    }
}
