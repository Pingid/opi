//! Identifier casing shared by templates and the TS backend. Dependency-free
//! so `build.rs` can include it (see `build.rs`).

/// `/components/{name}` -> `ComponentsByName`.
pub fn path_pascal_case(path: &str) -> String {
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
pub fn pascal_case(s: &str) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pascal() {
        assert_eq!(pascal_case("get-component_by.name"), "GetComponentByName");
        assert_eq!(pascal_case("GET"), "GET");
        assert_eq!(pascal_case("simple-user"), "SimpleUser");
        assert_eq!(pascal_case("2fa"), "_2fa");
        assert_eq!(path_pascal_case("/orgs/{org}/projectsV2"), "OrgsByOrgProjectsV2");
    }
}
