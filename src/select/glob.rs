//! `*` within a path segment, `**` across segments.

pub fn path_matches(glob: &str, path: &str) -> bool {
    let glob: Vec<_> = glob.split('/').collect();
    let path: Vec<_> = path.split('/').collect();
    segments_match(&glob, &path)
}

fn segments_match(glob: &[&str], path: &[&str]) -> bool {
    match glob.split_first() {
        None => path.is_empty(),
        Some((&"**", rest)) => (0..=path.len()).any(|i| segments_match(rest, &path[i..])),
        Some((segment, rest)) => match path.split_first() {
            Some((first, path)) => wildcard(segment, first) && segments_match(rest, path),
            None => false,
        },
    }
}

/// `*` matches any run of characters within a segment.
pub fn wildcard(pattern: &str, s: &str) -> bool {
    match pattern.split_once('*') {
        None => pattern == s,
        Some((prefix, rest)) => {
            let Some(s) = s.strip_prefix(prefix) else {
                return false;
            };
            (0..=s.len())
                .filter(|&i| s.is_char_boundary(i))
                .any(|i| wildcard(rest, &s[i..]))
        }
    }
}
